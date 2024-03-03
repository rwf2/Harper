use std::sync::Arc;

use rayon::prelude::*;
use rustc_hash::FxHashMap;
use derive_more::Debug;

use crate::fstree::{Entry, EntryId, FsTree};
use crate::value::List;
use crate::taxonomy::*;

#[derive(Debug)]
pub struct Collection {
    #[debug(ignore)]
    pub tree: Arc<FsTree>,
    #[debug("{:?}", tree[**root])]
    pub root: EntryId,
    pub index: Option<Arc<Item>>,
    pub items: Arc<List<Arc<Item>>>,
    pub data: FxHashMap<EntryId, Arc<List<Arc<Item>>>>,
}

// TODO: Add metadata to collection? Use it for all of its items?

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum Kind {
    /// An item that's the index.
    Index,
    /// This is the the entryid of the directory (self.data[entry]) it's in.
    Datum(EntryId),
    /// The sequence of the item.
    Item(usize),
}

impl Collection {
    pub fn new(tree: Arc<FsTree>, root: EntryId) -> Collection {
        Collection {
            tree,
            root,
            index: None,
            items: Default::default(),
            data: Default::default(),
        }
    }

    pub fn root_entry(&self) -> &Entry {
        &self.tree[self.root]
    }

    pub fn new_item(&mut self, entry: EntryId) -> Arc<Item> {
        let item = Arc::new(Item::new(self.tree.clone(), entry));
        self.items.push(item.clone());
        item
    }

    pub fn new_datum(&mut self, parent: EntryId, entry: EntryId) -> Arc<Item> {
        let datum = Arc::new(Item::new(self.tree.clone(), entry));
        self.data.entry(parent).or_default().push(datum.clone());
        datum
    }

    pub fn set_index_item(&mut self, entry: EntryId) -> Arc<Item> {
        let index = Arc::new(Item::new(self.tree.clone(), entry));
        self.index = Some(index.clone());
        index
    }

    #[inline]
    pub fn par_map_items<C, M, R: Send>(&self, map: M) -> C
        where M: Fn(Kind, &Arc<Item>) -> R + Send + Sync,
              C: FromParallelIterator<R>
    {
        let data_content = self.data.par_iter()
            .flat_map(|(&id, items)| items.par_iter().map(move |item| (Kind::Datum(id), item)));

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
