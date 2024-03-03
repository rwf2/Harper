use std::collections::VecDeque;

use pulldown_cmark::{Event, Tag, InlineStr, CowStr::*, TagEnd};

use super::Plugin;

pub trait CodeFilter: FnMut(&str, usize) -> bool {}
impl<F: FnMut(&str, usize) -> bool> CodeFilter for F {}

#[derive(Clone)]
pub struct CodeTrim<F> {
    trimmer: F,
}

#[derive(Clone)]
struct Iter<'a, F, I: Iterator<Item = Event<'a>>> {
    trimmer: F,
    inner: I,
    line_num: Option<usize>,
    stack: VecDeque<Event<'a>>,
}

impl<F: CodeFilter> CodeTrim<F> {
    pub fn trim(trimmer: F) -> Self { Self { trimmer } }
}

impl CodeTrim<()> {
    pub fn trim_start() -> CodeTrim<impl CodeFilter> {
        let mut is_start = true;
        CodeTrim::trim(move |line: &str, n: usize| {
            if n == 0 { is_start = true; }
            if is_start && line.bytes().all(|c| c.is_ascii_whitespace()) {
                true
            } else {
                is_start = false;
                false
            }
        })
    }
}

impl<F: CodeFilter> Plugin for CodeTrim<F> {
    fn remap<'a, I>(&'a mut self, i: I) -> Box<dyn Iterator<Item = Event<'a>> + 'a>
        where I: Iterator<Item = Event<'a>> + 'a
    {
        Box::new(Iter {
            trimmer: &mut self.trimmer,
            inner: i,
            line_num: None,
            stack: VecDeque::new(),
        })
    }
}

impl<'a, F: CodeFilter, I: Iterator<Item = Event<'a>>> Iterator for Iter<'a, F, I> {
    type Item = Event<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(event) = self.stack.pop_front() {
                return Some(event);
            }

            let event = self.inner.next()?;
            match &event {
                Event::Start(Tag::CodeBlock(_)) => {
                    self.line_num = Some(0);
                    return Some(event);
                }
                Event::End(TagEnd::CodeBlock) => {
                    self.line_num = None;
                    return Some(event);
                }
                Event::Text(text) if self.line_num.is_some() => {
                    let line_num = self.line_num.as_mut().unwrap();

                    let mut i = 0;
                    while i < text.len() {
                        let j = memchr::memchr(b'\n', text[i..].as_bytes())
                            .map(|k| i + k + 1)
                            .unwrap_or(text.len());

                        let line = &text[i..j];
                        if !(self.trimmer)(line, *line_num) {
                            let text = match text {
                                Inlined(_) => Inlined(InlineStr::try_from(line).unwrap()),
                                Borrowed(s) => Borrowed(&s[i..j]),
                                _ => line.to_string().into(),
                            };

                            self.stack.push_back(Event::Text(text));
                        }

                        *line_num += 1;
                        i = j;
                    }
                }
                _ => return Some(event),
            }
        }
    }
}
