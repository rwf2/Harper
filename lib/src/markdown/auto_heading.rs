use std::collections::VecDeque;
use std::fmt::Write;

use pulldown_cmark::{Event, Tag, CowStr, TagEnd};
use rustc_hash::FxHashMap;

use super::Plugin;

#[derive(Default)]
pub struct AutoHeading;

struct HeadingIterator<'a, I: Iterator<Item = Event<'a>>> {
    stack: VecDeque<Event<'a>>,
    seen: FxHashMap<String, usize>,
    inner: I,
}

impl<'a, I: Iterator<Item = Event<'a>>> Iterator for HeadingIterator<'a, I> {
    type Item = Event<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(event) = self.stack.pop_front() {
            return Some(event);
        }

        match self.inner.next()? {
            Event::Start(Tag::Heading { level, id: None, classes, attrs }) => {
                let mut text = String::new();
                loop {
                    let event = self.inner.next()?;
                    if let Event::Text(ref s) | Event::Code(ref s) = event {
                        text.push_str(&s);
                    } else if let Event::End(TagEnd::Heading(..)) = event {
                        break;
                    }

                    self.stack.push_back(event);
                }

                let mut id = crate::util::slugify(&text);
                if let Some(n) = self.seen.get(&id) {
                    let _ = write!(&mut id, "-{}", n);
                } else {
                    self.seen.insert(id.clone(), 1);
                }

                let tag = Tag::Heading { level, id: Some(id.into()), classes, attrs };
                self.stack.push_back(Event::End(TagEnd::Heading(level)));
                Some(Event::Start(tag))
            },
            event => Some(event)
        }
    }
}

impl Plugin for AutoHeading {
    fn remap<'a, I>(&'a mut self, events: I) -> Box<dyn Iterator<Item = Event<'a>> + 'a>
        where I: Iterator<Item = Event<'a>> + 'a
    {
        Box::new(HeadingIterator {
            seen: FxHashMap::default(),
            inner: events,
            stack: VecDeque::with_capacity(4),
        })
    }
}

#[derive(Default)]
pub struct HeadingAnchor;

struct AnchorIterator<'a, I: Iterator<Item = Event<'a>>> {
    pending: Option<CowStr<'a>>,
    inner: I,
}

impl<'a, I: Iterator<Item = Event<'a>>> Iterator for AnchorIterator<'a, I> {
    type Item = Event<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(id) = self.pending.take() {
            let html = format!(r##"<a class="anchor" title="anchor" href="#{id}"></a>"##);
            return Some(Event::Html(html.into()));
        }

        let event = self.inner.next()?;
        if let Event::Start(Tag::Heading { id: Some(ref id), .. }) = event {
            self.pending = Some(id.clone());
        }

        Some(event)
    }
}

impl Plugin for HeadingAnchor {
    fn remap<'a, I>(&'a mut self, events: I) -> Box<dyn Iterator<Item = Event<'a>> + 'a>
        where I: Iterator<Item = Event<'a>> + 'a
    {
        Box::new(AnchorIterator {
            inner: events,
            pending: None,
        })
    }
}
