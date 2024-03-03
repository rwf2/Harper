use pulldown_cmark::{Event, Tag, HeadingLevel};

use crate::markdown::Plugin;

pub type LunrIndex = elasticlunr::Index;

#[derive(Default, Debug)]
pub struct LunrIndexer {
    pub docs: Vec<LunrDocument>,
}

struct Heading {
    level: HeadingLevel,
    name: String,
}

#[derive(Debug)]
pub struct LunrDocument {
    id: String,
    title: String,
    breadcrumb: String,
    body: String,
}

#[derive(Copy, Clone, PartialEq)]
enum State {
    InHeading,
    InBody,
}

struct IndexerIterator<'a, I: Iterator<Item = Event<'a>>> {
    breadcrumb_stack: Vec<Heading>,
    docs: &'a mut Vec<LunrDocument>,
    state: State,
    inner: I,
}

impl<'a, I: Iterator<Item = Event<'a>>> IndexerIterator<'a, I> {
    fn breadcrumb_string(&self) -> String {
        let mut string = String::new();
        let mut headings = self.breadcrumb_stack.iter();
        if let Some(heading) = headings.next() {
            string.push_str(&heading.name);
        }

        for heading in headings {
            string.push_str(" > ");
            string.push_str(&heading.name);
        }

        string
    }
}

impl<'a, I: Iterator<Item = Event<'a>>> Iterator for IndexerIterator<'a, I> {
    type Item = Event<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        // TODO: Check if we have a doc here before sending `None`.
        let event = self.inner.next()?;
        match event {
            Event::Start(Tag::Heading(level, Some(ref id), _)) => {
                while self.breadcrumb_stack.last().map_or(false, |h| h.level >= level) {
                    self.breadcrumb_stack.pop();
                }

                self.state = State::InHeading;
                self.docs.push(LunrDocument {
                    id: id.to_string(),
                    title: String::new(),
                    breadcrumb: String::new(),
                    body: String::new()
                })
            }
            Event::Text(ref s) | Event::Code(ref s) if self.state == State::InHeading => {
                if let Some(doc) = self.docs.last_mut() {
                    doc.title.push_str(s);
                }
            },
            Event::Text(ref s) | Event::Code(ref s) => {
                if let Some(doc) = self.docs.last_mut() {
                    doc.body.push_str(s);
                }
            },
            Event::End(Tag::Heading(level, Some(_), _)) => {
                self.state = State::InBody;
                if let Some(doc) = self.docs.last_mut() {
                    self.breadcrumb_stack.push(Heading { level, name: doc.title.clone() });
                }

                let breadcrumb_string = self.breadcrumb_string();
                if let Some(doc) = self.docs.last_mut() {
                    doc.breadcrumb = breadcrumb_string;
                }
            },
            _ => { /* skip */ }
        }

        Some(event)
    }
}

impl Plugin for &mut LunrIndexer {
    fn remap<'a, I>(&'a mut self, events: I) -> Box<dyn Iterator<Item = Event<'a>> + 'a>
        where I: Iterator<Item = Event<'a>> + 'a
    {
        Box::new(IndexerIterator {
            breadcrumb_stack: vec![],
            docs: &mut self.docs,
            state: State::InBody,
            inner: events,
        })
    }
}

impl LunrDocument {
    pub const FIELDS: [&str; 3] = ["title", "breadcrumb", "body"];

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn fields(&self) -> [&str; 3] {
        [&self.title, &self.breadcrumb, &self.body]
    }
}
