use pulldown_cmark::{Event, Tag, CodeBlockKind, TagEnd};
use pulldown_cmark_escape::{escape_html, escape_html_body_text};
use jellybean::{Language, Highlight, ALL_LANGUAGES};

use super::Plugin;

#[derive(Default, Clone)]
pub struct SyntaxHighlight;

pub struct Highlighter<I> {
    block: Option<CodeBlock>,
    inner: I,
}

struct CodeBlock {
    language: Option<&'static Language>,
    code: String,
}

impl SyntaxHighlight {
    /// Decodes every language's highlighting configuration in the background.
    #[inline]
    pub fn warm_up() {
        for language in ALL_LANGUAGES {
            rayon::spawn(|| { language.highlighter(); });
        }
    }
}

impl Plugin for SyntaxHighlight {
    fn remap<'a, I>(&'a mut self, events: I) -> impl Iterator<Item = Event<'a>> + 'a
        where I: Iterator<Item = Event<'a>> + 'a
    {
        Highlighter { block: None, inner: events }
    }
}

impl CodeBlock {
    /// Writes `code` as HTML with a `<span>` for every highlight. The span's
    /// classes are the parts of its capture name: `function.macro` becomes
    /// `<span class="function macro">`.
    fn highlight(language: &'static Language, code: &str, html: &mut String) -> bool {
        let mut highlighter = language.highlighter();
        for event in highlighter.highlight(code) {
            match event {
                Ok(Highlight::Start { group, .. }) => {
                    html.push_str("<span class=\"");
                    for (i, part) in group.split('.').enumerate() {
                        if i > 0 { html.push(' '); }
                        let _ = escape_html(&mut *html, part);
                    }

                    html.push_str("\">");
                }
                Ok(Highlight::Source { text, .. }) => {
                    let _ = escape_html_body_text(&mut *html, text);
                }
                Ok(Highlight::End) => html.push_str("</span>"),
                Err(_) => return false,
            }
        }

        true
    }

    #[allow(unused_must_use)]
    fn render(&self) -> String {
        use std::fmt::Write;

        let lines = memchr::memchr_iter(b'\n', self.code.as_bytes()).count();
        let mut div = String::with_capacity(self.code.len() * 3);
        write!(&mut div, "<div class=\"code\" style=\"display: flex;\">");

        write!(&mut div, "<pre class=\"line-nums\">");
        for i in 1..=lines {
            if i < lines { write!(&mut div, "{}\n", i); }
            else { write!(&mut div, "{}", i); }
        }
        write!(&mut div, "</pre>");

        write!(&mut div, "<pre class=\"code\">");
        let start = div.len();
        let highlighted = self.language
            .is_some_and(|language| Self::highlight(language, &self.code, &mut div));

        if !highlighted {
            div.truncate(start);
            escape_html_body_text(&mut div, &self.code);
        }

        write!(&mut div, "</pre>");
        write!(&mut div, "</div>");
        div
    }
}

impl<'a, I: Iterator<Item = Event<'a>>> Iterator for Highlighter<I> {
    type Item = Event<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.inner.next()? {
                Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(label))) => {
                    let token = label.split_once(',')
                        .map(|(prefix, _)| prefix)
                        .unwrap_or(&*label);

                    let language = Language::find(token.trim());
                    self.block = Some(CodeBlock { language, code: String::new() });
                }
                Event::Text(text) if self.block.is_some() => {
                    self.block.as_mut().unwrap().code.push_str(&text);
                }
                Event::End(TagEnd::CodeBlock) if self.block.is_some() => {
                    let block = self.block.take().unwrap();
                    return Some(Event::Html(block.render().into()));
                },
                ev => return Some(ev),
            }
        }
    }
}

#[cfg(test)]
mod tests;
