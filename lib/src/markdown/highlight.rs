use pulldown_cmark::{Event, Tag, CodeBlockKind, TagEnd};
use syntect::html::{ClassedHTMLGenerator, ClassStyle};
use syntect::parsing::{SyntaxSet, SyntaxReference};
use once_cell::sync::Lazy;

use super::Plugin;

static SYNTAX_SET: Lazy<SyntaxSet> = Lazy::new(||
    syntect::dumps::from_uncompressed_data(include_bytes!(
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/syntax-set-newlines-uncompressed.packdump")
    )).unwrap());

static DEFAULT_SYNTAX: Lazy<&'static SyntaxReference>
    = Lazy::new(|| SYNTAX_SET.find_syntax_plain_text());

#[derive(Default, Clone)]
pub struct SyntaxHighlight;

pub struct Highlighter<I> {
    generator: Option<ClassedHTMLGenerator<'static>>,
    lines: usize,
    inner: I,
}

impl SyntaxHighlight {
    #[inline]
    pub fn warm_up() {
        rayon::spawn(|| { Lazy::force(&SYNTAX_SET); });
        rayon::spawn(|| { Lazy::force(&DEFAULT_SYNTAX); });
    }
}

impl Plugin for SyntaxHighlight {
    fn remap<'a, I>(&'a mut self, events: I) -> impl Iterator<Item = Event<'a>> + 'a
        where I: Iterator<Item = Event<'a>> + 'a
    {
        Highlighter { generator: None, lines: 0, inner: events }
    }
}

fn html_generator(syntax: &SyntaxReference) -> ClassedHTMLGenerator<'_> {
    ClassedHTMLGenerator::new_with_class_style(syntax, &*SYNTAX_SET, ClassStyle::Spaced)
}

#[allow(unused_must_use)]
fn code_div(lines: usize, code: String) -> String {
    use std::fmt::Write;

    let mut div = String::new();
    write!(&mut div, "<div class=\"code\" style=\"display: flex;\">");

    write!(&mut div, "<pre class=\"line-nums\">");
    for i in 1..=lines {
        if i < lines { write!(&mut div, "{}\n", i); }
        else { write!(&mut div, "{}", i); }
    }
    write!(&mut div, "</pre>");

    write!(&mut div, "<pre class=\"code\">{}</pre>", code);
    write!(&mut div, "</div>");

    div
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

                    let syntax = SYNTAX_SET.find_syntax_by_token(lang)
                        .unwrap_or_else(|| &*DEFAULT_SYNTAX);

                    self.generator = Some(html_generator(syntax));
                    self.lines = 0;
                }
                Event::Text(text) if self.generator.is_some() => {
                    let generator = self.generator.as_mut().unwrap();
                    self.lines += memchr::memrchr_iter(b'\n', text.as_bytes()).count();
                    let _ = generator.parse_html_for_line_which_includes_newline(&text);
                }
                Event::End(TagEnd::CodeBlock) if self.generator.is_some() => {
                    let generator = self.generator.take().unwrap();
                    let code_html = code_div(self.lines, generator.finalize());
                    return Some(Event::Html(code_html.into()));
                },
                ev => return Some(ev),
            }
        }
    }
}
