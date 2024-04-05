use pulldown_cmark::{Event, Tag, CowStr};
use rustc_hash::FxHashMap;

// type Map = std::collections::BTreeMap<String, String>;
type Map = FxHashMap<String, String>;

#[derive(Clone)]
pub struct Alias<'a> {
    map: &'a Map,
}

struct AliasIterator<'e, I: Iterator<Item = Event<'e>>> {
    inner: I,
    map: &'e Map
}

impl<'a> Alias<'a> {
    pub fn new(map: &'a Map) -> Self { Self { map } }
}

impl crate::markdown::Plugin for Alias<'_> {
    fn remap<'a, I>(&'a mut self, events: I) -> impl Iterator<Item = Event<'a>> + 'a
        where I: Iterator<Item = Event<'a>> + 'a
    {
        AliasIterator { inner: events, map: self.map }
    }
}

impl<'e, I: Iterator<Item = Event<'e>>> Iterator for AliasIterator<'e, I> {
    type Item = Event<'e>;

    fn next(&mut self) -> Option<Self::Item> {
        let event = match self.inner.next()? {
            Event::Start(Tag::Link { link_type, dest_url, title, id }) => {
                let dest_url = rewrite(self.map, dest_url);
                Event::Start(Tag::Link { link_type, dest_url, title, id })
            },
            event => event,
        };

        Some(event)
    }
}

fn rewrite<'a>(aliases: &'a Map, href: CowStr<'a>) -> CowStr<'a> {
    if !href.starts_with('@') {
        return href;
    }

    let (alias, suffix) = href[1..].split_once('/')
        .map(|(alias, suffix)| (alias, suffix))
        .unwrap_or((&href[1..], ""));

    aliases.get(alias)
        .map(|prefix| {
            if !prefix.ends_with('/') && !suffix.is_empty() && !suffix.starts_with('/') {
                format!("{prefix}/{suffix}").into()
            } else {
                format!("{prefix}{suffix}").into()
            }
        })
        .unwrap_or(href)
}
