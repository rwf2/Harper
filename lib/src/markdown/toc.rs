use pulldown_cmark::{Event, Tag};
use serde::Serialize;

use crate::error::Result;
use crate::markdown::Plugin;
use crate::value::Sink;
use crate::value::{Dict, Value};

#[derive(Debug, Serialize, Clone)]
pub struct Entry {
    pub title: String,
    pub level: usize,
    pub id: Option<String>,
    pub children: Vec<Entry>,
}

#[derive(Debug, Clone)]
pub struct TableOfContents<O> {
    pub entries: Vec<Entry>,
    entry: Option<Entry>,
    output: O,
}

impl<O: Sink> TableOfContents<O> {
    pub fn new(output: O) -> Self {
        Self { entries: vec![], entry: None, output }
    }

    pub fn reset(&mut self) {
        self.entries = vec![];
        self.entry = None;
    }

    /// SAFETY: We checked this with Polonius...
    #[inline]
    fn find_parent<'h>(&'h mut self, needle: &Entry) -> Option<&'h mut Entry> {
        unsafe fn _find<'h>(haystack: *mut Vec<Entry>, needle: &Entry) -> Option<*mut Entry> {
            for entry in (*haystack).iter_mut().rev() {
                if entry.level < needle.level {
                    match _find(&mut entry.children, needle) {
                        Some(parent) => return Some(parent),
                        None => return Some(entry)
                    }
                }
            }

            None
        }

        unsafe { _find(&mut self.entries as *mut _, needle).map(|parent| &mut *parent) }
    }
}

impl<O: Sink> Plugin for TableOfContents<O> {
    fn remap<'a, I>(&'a mut self, events: I) -> impl Iterator<Item = Event<'a>> + 'a
        where I: Iterator<Item = Event<'a>> + 'a
    {
        self.reset();

        events.inspect(|ev| match ev {
            Event::Start(Tag::Heading { level, id, .. }) => {
                self.entry = Some(Entry {
                    title: String::new(),
                    level: *level as usize,
                    children: vec![],
                    id: id.as_ref().map(|c| c.to_string()),
                });
            },
            Event::Text(text) | Event::Code(text) if self.entry.is_some() => {
                let mut entry = self.entry.take().unwrap();
                entry.title.push_str(text);
                if let Some(parent) = self.find_parent(&entry) {
                    parent.children.push(entry);
                } else {
                    self.entries.push(entry);
                }
            }
            _ => {}
        })
    }

    fn finalize(&mut self) -> Result<()> {
        let entries = self.entries.iter()
            .map(Value::from)
            .collect::<Value>();

        self.output.write_value(entries)
    }
}

impl From<&Entry> for Value {
    fn from(value: &Entry) -> Self {
        let dict: Dict = crate::dict![
            "title" => value.title.as_str(),
            "level" => value.level,
            "id" => value.id.as_deref(),
            "children" => value.children.iter()
                .map(Value::from)
                .collect::<Vec<_>>(),
        ];

        Value::from(dict)
    }
}
