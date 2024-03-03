use pulldown_cmark::{html, Event, Tag, CowStr};

use crate::value::Sink;
use crate::error::Result;
use crate::markdown::Plugin;

const SEPERATOR: &str = "===";

pub struct Parts<O> {
    output: O,
    sections: Vec<String>,
}

impl<O> Parts<O> {
    pub fn new(output: O) -> Self {
        Self { output, sections: vec![] }
    }
}

struct SectionIterator<'a, I: Iterator<Item = Event<'a>>> {
    stack: Vec<Event<'a>>,
    found: bool,
    inner: I,
}

impl<'a, I: Iterator<Item = Event<'a>>> Iterator for SectionIterator<'a, I> {
    type Item = Event<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(event) = self.stack.pop() {
            return Some(event);
        }

        match self.inner.next()? {
            start@Event::Start(Tag::Paragraph) => match self.inner.next()? {
                Event::Text(text) if text.starts_with(SEPERATOR) => {
                    self.found = true;
                    let _end = self.inner.next()?;
                    self.stack.push(self.inner.next()?);
                    return None;
                }
                inner => {
                    self.stack.push(inner);
                    Some(start)
                }
            }
            event => Some(event),
        }
    }
}

impl<O: Sink> Plugin for Parts<O> {
    // TODO: This is a totally lie and breaks any plugins downchain. Fix it. The
    // way this works is that the iterator returns `None` (stops) for every
    // secion. This causes the HTML renderer to emit the string so far. We then
    // reuse the same iterator in the renderer again until it signals it cannot
    // be reused. We finally return one string containing all of the HTML.
    fn remap<'a, I>(&'a mut self, events: I) -> Box<dyn Iterator<Item = Event<'a>> + 'a>
        where I: Iterator<Item = Event<'a>> + 'a
    {
        let mut sections = SectionIterator {
            stack: vec![],
            found: false,
            inner: events,
        };

        loop {
            // Render until we get a `None` and record the section.
            let mut html_output = String::new();
            html::push_html(&mut html_output, &mut sections);
            self.sections.push(html_output);

            // There's no where to continue, so bail.
            if sections.stack.is_empty() {
                break;
            }
        }

        let have_parts = sections.found;
        let complete_html = match self.sections.len() {
            0 if !have_parts => CowStr::from(""),
            1 if !have_parts => self.sections.pop().unwrap().into(),
            1 if have_parts => self.sections.last().map(|s| s.as_str()).unwrap().into(),
            _ => self.sections.join("").into(),
        };

        Box::new(Some(Event::Html(complete_html)).into_iter())
    }

    fn finalize(&mut self) -> Result<()> {
        let sections = std::mem::replace(&mut self.sections, vec![]);
        self.output.write(sections)
    }
}
