use pulldown_cmark::{Event, Tag, CodeBlockKind, escape::escape_html};
use tree_sitter_highlight::{HighlightConfiguration, Error};
use once_cell::sync::Lazy;

use super::Plugin;

pub struct Highlighter<I> {
    config: Option<&'static HighlightConfiguration>,
    code: String,
    inner: I,
}

pub static HIGHLIGHT_NAMES: &[&str] = &[
    "attribute",
    "label",
    "constant",
    "function.builtin",
    "function.macro",
    "function",
    "keyword",
    "operator",
    "property",
    "punctuation",
    "punctuation.bracket",
    "punctuation.delimiter",
    "string",
    "string.special",
    "tag",
    "escape",
    "type",
    "type.builtin",
    "constructor",
    "variable",
    "variable.builtin",
    "variable.parameter",
    "comment",
];

// FIXME: Building the `HighlightConfiguration` is really slow and dominates the
// time it takes to perform a syntax highlight (~70ms), which in-turn dominates
// the total render time. If we can somehow do this at compile-time, this would
// be a net win over `syntex`, assuming we can get highlighter parity.
macro_rules! define_languages {
    ($($lib:ident: [$($name:literal),* $(,)?]),* $(,)?) => {
        mod config {
            use super::*;

            $(
                #[allow(non_upper_case_globals)]
                pub static $lib: Lazy<Option<HighlightConfiguration>> = Lazy::new(|| {
                    let lang = $lib::language();
                    let query = $lib::HIGHLIGHT_QUERY;
                    let mut config = HighlightConfiguration::new(lang, query, "", "").ok()?;
                    config.configure(HIGHLIGHT_NAMES);
                    Some(config)
                });
            )*

            pub static ALL: &[&Lazy<Option<HighlightConfiguration>>] = &[$(&$lib),*];
        }

        fn find_ts_highlight_config(name: &str) -> Option<&'static HighlightConfiguration> {
            match name {
                $($($name)|* => config::$lib.as_ref(),)*
                _ => None
            }
        }
    }
}

define_languages! {
    tree_sitter_rust: ["rust", "rs"],
    tree_sitter_bash: ["bash", "sh", "shell"],
    tree_sitter_toml: ["toml"],
}

impl<I> Highlighter<I> {
    fn try_highlight_to_html(&self) -> Result<String, Error> {
        use std::fmt::Write;
        use tree_sitter_highlight::{Highlighter, HighlightEvent};

        let config = self.config.as_ref().ok_or(Error::Unknown)?;
        let source = self.code.as_bytes();

        let mut hl = Highlighter::new();
        let highlights = hl.highlight(config, source, None, |_| None)?;

        let mut html = String::new();
        html.push_str("<div class=\"code\" style=\"display: flex;\">");
        html.push_str("<pre class=\"line-nums\">");
        let lines = memchr::memrchr_iter(b'\n', source).count();
        for i in 1..=lines {
            if i < lines { let _ = write!(&mut html, "{}\n", i); }
            else { let _ = write!(&mut html, "{}", i); }
        }
        html.push_str("</pre>");
        html.push_str("<pre class=\"code\">");

        for event in highlights {
            match event? {
                HighlightEvent::HighlightStart(s) => {
                    let _ = write!(&mut html, "<span class='{}'>", s.0);
                }
                HighlightEvent::Source { start, end } => {
                    let code_span = self.code.get(start..end).ok_or(Error::Unknown)?;
                    escape_html(&mut html, code_span).map_err(|_| Error::Unknown)?;
                }
                HighlightEvent::HighlightEnd => {
                    html.push_str("</span>");
                }
            }
        }

        html.push_str("</div>");
        Ok(html)
    }

    fn highlight_to_html(&self) -> String {
        self.try_highlight_to_html()
            .unwrap_or_else(|_| self.code.to_string())
    }
}

impl<'a, I: Iterator<Item = Event<'a>>> Iterator for Highlighter<I> {
    type Item = Event<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.inner.next()? {
                Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(label))) => {
                    let lang = label.split_once(',')
                        .map(|(prefix, _)| prefix)
                        .unwrap_or(&*label);

                    self.code = String::new();
                    // self.config = time!(find_ts_highlight_config(lang));
                    self.config = find_ts_highlight_config(lang);
                }
                Event::Text(text) if self.config.is_some() => {
                    self.code.push_str(&text);
                }
                Event::End(Tag::CodeBlock(_)) if self.config.is_some() => {
                    let html = self.highlight_to_html();
                    self.config = None;
                    return Some(Event::Html(html.into()));
                },
                ev => return Some(ev),
            }
        }
    }
}

#[derive(Default, Clone)]
pub struct SyntaxHighlight;

impl SyntaxHighlight {
    pub fn warm_up() {
        use rayon::prelude::*;
        rayon::spawn(|| config::ALL.par_iter().for_each(|lazy| { Lazy::force(lazy); }))
    }
}

impl Plugin for SyntaxHighlight {
    fn remap<'a, I>(&'a mut self, events: I) -> Box<dyn Iterator<Item = Event<'a>> + 'a>
        where I: Iterator<Item = Event<'a>> + 'a
    {
        Box::new(Highlighter { config: None, code: String::new(), inner: events })
    }
}
