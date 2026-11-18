use std::sync::Arc;

use rayon::prelude::*;

use crate::error::Result;
use crate::fstree::{Entry, EntryId, FsTree, OwnedEntry};
use crate::util::{DuoIndexMap, PathExt};
use crate::value::List;
use crate::taxonomy::*;

#[derive(Debug)]
pub struct Collection {
    pub entry: OwnedEntry,
    pub name: Arc<str>,
    pub index: Option<Arc<Item>>,
    pub items: Arc<List<Arc<Item>>>,
    pub data: DuoIndexMap<EntryId, Arc<str>, Arc<List<Arc<Item>>>>,
}

// TODO: Add metadata to collection? Use it for all of its items?

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum Kind {
    /// An item that's the index.
    Index,
    /// An item in a data directory (`self.data`).
    Data,
    /// The sequence of the item.
    Item(usize),
}

impl Collection {
    pub fn new(name: Arc<str>, tree: Arc<FsTree>, root: EntryId) -> Collection {
        Collection {
            name,
            entry: OwnedEntry::new(tree, root),
            index: None,
            items: Default::default(),
            data: Default::default(),
        }
    }

    pub fn entry(&self) -> &Entry {
        &self.entry
    }

    pub fn new_item(&mut self, id: EntryId) -> Arc<Item> {
        let item = Arc::new(Item::new(self.entry.tree.clone(), id));
        self.items.push(item.clone());
        item
    }

    /// Adds `entries`, files in the data directory `dir`, as data. Fails if
    /// `dir`'s name is already the name of another data directory.
    pub fn new_data<I>(&mut self, dir: EntryId, entries: I) -> Result<()>
        where I: IntoIterator<Item = EntryId>
    {
        let tree = &self.entry.tree;
        let dir = &tree[dir];
        let name = || dir.path_relative_to(&self.entry)
            .unwrap()
            .to_slash_lossy()
            .into();

        let data = match self.data.get_or_insert_default(dir.id, name) {
            Ok(data) => data,
            Err(name) => return err! {
                "multiple data directories in a collection have the same name",
                "collection" => self.name,
                "name" => name,
                "directory" => format!("{:?}", dir.path),
                "note: non-UTF-8 path components are converted lossily",
            }
        };

        for entry in entries {
            data.push(Arc::new(Item::new(tree.clone(), entry)));
        }

        Ok(())
    }

    pub fn set_index_item(&mut self, entry: EntryId) -> Arc<Item> {
        let index = Arc::new(Item::new(self.entry.tree.clone(), entry));
        self.index = Some(index.clone());
        index
    }

    #[inline]
    pub fn par_map_items<C, M, R: Send>(&self, map: M) -> C
        where M: Fn(Kind, &Arc<Item>) -> R + Send + Sync,
              C: FromParallelIterator<R>
    {
        let data_content = self.data.par_iter()
            .flat_map(|items| items.par_iter().map(move |item| (Kind::Data, item)));

        let index_content = self.index.as_ref()
            .into_par_iter()
            .map(|index| (Kind::Index, index));

        let item_content = self.items.par_iter()
            .enumerate()
            .map(|(i, item)| (Kind::Item(i), item));

        data_content.chain(index_content)
            .chain(item_content)
            .map(|(kind, item)| map(kind, item))
            .collect()
    }
}
