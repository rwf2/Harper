use std::sync::Arc;

use rustc_hash::FxHashMap;

use crate::fstree::{EntryId, FsTree};
use crate::taxonomy::*;

#[derive(Debug)]
pub struct Site {
    pub tree: Arc<FsTree>,
    pub items: Vec<Arc<Item>>,
    pub collections: FxHashMap<EntryId, Arc<Collection>>,
    pub index: FxHashMap<Arc<str>, EntryId>,
}

impl Site {
    pub fn new(tree: Arc<FsTree>) -> Site {
        Site { tree, items: vec![], collections: Default::default(), index: Default::default() }
    }

    /// Panics if `name` is not unique to `root`.
    pub fn get_or_insert_collection(
        &mut self,
        name: impl FnOnce() -> Arc<str>,
        root: EntryId
    ) -> &mut Collection {
        let arc = self.collections.entry(root).or_insert_with(|| {
            let name = name();
            let collection = Arc::new(Collection::new(name.clone(), self.tree.clone(), root));
            assert!(self.index.insert(name, root).is_none());
            collection
        });

        Arc::get_mut(arc).expect("&mut -> &mut")
    }

    pub fn new_resource(&mut self, id: EntryId) -> Arc<Item> {
        let item = Arc::new(Item::new(self.tree.clone(), id));
        self.items.push(item.clone());
        item
    }
}

impl Site {
    fn vis_heading(&self, siblings: &[bool], id: EntryId, root: EntryId, prefix: &str) {
        let (entry, root) = (&self.tree[id], &self.tree[root]);
        for (j, sibling) in siblings.iter().enumerate() {
            match (sibling, j == siblings.len() - 1) {
                (false, false) => print!("    "),
                (false, true) => print!("└── "),
                (true, false) => print!("│   "),
                (true, true) => print!("├── "),
            }
        }

        println!("{prefix}{}", entry.path.strip_prefix(&root.path).unwrap().display());
    }

    pub fn visualize(&self) {
        let root_id = self.tree.root_id();
        self.vis_heading(&[], root_id, root_id, "🗂 ");

        for (i, collection) in self.collections.values().enumerate() {
            let i_sib = i < self.collections.len() - 1;
            self.vis_heading(&[i_sib], collection.entry.id, self.tree.root_id(), "");

            for (j, (&data_id, data_items)) in collection.data.iter().enumerate() {
                let j_sib = !collection.items.is_empty()
                    || collection.index.is_some()
                    || j < collection.data.len() - 1;

                self.vis_heading(&[i_sib, j_sib], data_id, collection.entry.id, "📦 ");

                for (k, item) in data_items.iter().enumerate() {
                    let k_sib = k < data_items.len() - 1;
                    self.vis_heading(&[i_sib, j_sib, k_sib], item.entry.id, data_id, "💾 ");
                }
            }

            if let Some(item) = &collection.index {
                let j_sib = !collection.items.is_empty();
                self.vis_heading(&[i_sib, j_sib], item.entry.id, collection.entry.id, "📑 ");
            }

            for (j, item) in collection.items.iter().enumerate() {
                let j_sib = j < collection.items.len() - 1;
                self.vis_heading(&[i_sib, j_sib], item.entry.id, collection.entry.id, "📝 ");
            }
        }
    }
}
