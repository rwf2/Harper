use std::fmt::Write;

use pulldown_cmark::{Event, Tag, TagEnd};

use crate::error::Result;
use crate::markdown::Plugin;
use crate::value::Sink;

pub struct Snippet<O> {
    output: O,
    snippet: String,
    length: usize,
}

impl<O> Snippet<O> {
    pub fn new(output: O, length: usize) -> Self {
        Self { output, snippet: String::new(), length }
    }
}

struct SnippetIterator<'a, I: Iterator<Item = Event<'a>>> {
    snippet: &'a mut String,
    inner: I,
    capture: Vec<bool>,
    snip_text_len: usize,
    min_length: usize,
    done: bool,
}

macro_rules! open {
    ($it:expr) => ($it.capture.push(false));
    ($it:expr, $($fmt:tt)*) => ({
        let _ = write!($it.snippet, $($fmt)*);
        $it.capture.push(true);
    })
}

macro_rules! close {
    ($it:expr) => ({
        $it.capture.pop();
        if $it.capture.is_empty() && $it.snip_text_len >= $it.min_length {
            $it.done = true;
        }
    });
    ($it:expr, $($fmt:tt)*) => ({
        let _ = write!($it.snippet, $($fmt)*);
        close!($it);
    })
}

macro_rules! capture {
    ($it:expr, $str:expr) => (capture!($it, $str, $str));
    ($it:expr, $str:expr, $($fmt:tt)*) => ({
        if !$it.done && $it.capture.last().copied().unwrap_or_default() {
            let _ = write!($it.snippet, $($fmt)*);
            $it.snip_text_len += $str.len();
        }
    })
}

impl<'a, I: Iterator<Item = Event<'a>>> Iterator for SnippetIterator<'a, I> {
    type Item = Event<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let event = self.inner.next()?;
        if self.done {
            return Some(event);
        }

        match &event {
            Event::Start(tag) => match tag {
                Tag::Paragraph => open!(self, "<p>"),
                Tag::Emphasis => open!(self, "<em>"),
                Tag::Strong => open!(self, "<strong>"),
                Tag::Strikethrough => open!(self, "<strike>"),
                Tag::BlockQuote => open!(self, "<blockquote>"),
                Tag::Link { dest_url, title, .. } => {
                    open!(self, r#"<a href="{dest_url}" title="{title}">"#)
                }
                _ => open!(self),
            },
            Event::End(tag) => match tag {
                TagEnd::Paragraph => close!(self, "</p>"),
                TagEnd::Emphasis => close!(self, "</em>"),
                TagEnd::Strong => close!(self, "</strong>"),
                TagEnd::Strikethrough => close!(self, "</strike>"),
                TagEnd::BlockQuote => close!(self, "</blockquote>"),
                TagEnd::Link => close!(self, "</a>"),
                _ => close!(self),
            },

            Event::SoftBreak => capture!(self, " "),
            Event::HardBreak => capture!(self, "", "<br>"),
            Event::Code(text) => capture!(self, text, "<code>{text}</code>"),
            Event::Text(text) => capture!(self, text, "{text}"),
            _ => { /* do nothing */ }
        }

        Some(event)
    }
}

// IDEA: What if we just take the first k character of the text and render that as markdown?

impl<O: Sink> Plugin for Snippet<O> {
    fn remap<'a, I>(&'a mut self, events: I) -> Box<dyn Iterator<Item = Event<'a>> + 'a>
        where I: Iterator<Item = Event<'a>> + 'a
    {
        Box::new(SnippetIterator {
            snippet: &mut self.snippet,
            snip_text_len: 0,
            inner: events,
            capture: vec![],
            min_length: self.length,
            done: self.length == 0,
        })
    }

    fn finalize(&mut self) -> Result<()> {
        let snippet = std::mem::replace(&mut self.snippet, String::new());
        self.output.write(snippet)
    }
}
