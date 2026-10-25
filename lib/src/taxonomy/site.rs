use std::sync::Arc;

use crate::fstree::{EntryId, FsTree};
use crate::taxonomy::*;
use crate::util::DuoIndexMap;

#[derive(Debug)]
pub struct Site {
    pub tree: Arc<FsTree>,
    pub items: Vec<Arc<Item>>,
    pub collections: DuoIndexMap<EntryId, Arc<str>, Arc<Collection>>,
}

impl Site {
    pub fn new(tree: Arc<FsTree>) -> Site {
        // Site { tree, items: vec![], collections: Default::default(), index: Default::default() }
        Site { tree, items: vec![], collections: Default::default() }
    }

    /// Panics if `name` is not unique to `root`.
    pub fn get_or_insert_collection(
        &mut self,
        name: impl FnOnce() -> Arc<str>,
        root: EntryId
    ) -> &mut Collection {
        let arc = self.collections.get_or_insert_with(root, name,
            |_, name| {
                println!("{name} -> {}", self.tree[root].relative_path().display());
                Arc::new(Collection::new(name.clone(), self.tree.clone(), root))
            })
            .expect("unique root/name combo");

        Arc::get_mut(arc).expect("&mut -> &mut")
    }

    pub fn new_resource(&mut self, id: EntryId) -> Arc<Item> {
        let item = Arc::new(Item::new(self.tree.clone(), id));
        self.items.push(item.clone());
        item
    }
}

impl Site {
    fn vis_heading(&self, siblings: &[bool], heading: &str, prefix: &str) {
        for (j, sibling) in siblings.iter().enumerate() {
            match (sibling, j == siblings.len().saturating_sub(1)) {
                (false, false) => print!("    "),
                (false, true) => print!("└── "),
                (true, false) => print!("│   "),
                (true, true) => print!("├── "),
            }
        }

        println!("{prefix}{heading}");
    }

    pub fn visualize(&self) {
        self.vis_heading(&[], "·", "");

        for (i, (name, collection)) in self.collections.iter_secondary().enumerate() {
            let i_sib = i < self.collections.len().saturating_sub(1);
            self.vis_heading(&[i_sib], name, "");

            for (j, (name, data_items)) in collection.data.iter_secondary().enumerate() {
                let j_sib = !collection.items.is_empty()
                    || collection.index.is_some()
                    || j < collection.data.len().saturating_sub(1);

                self.vis_heading(&[i_sib, j_sib], name, "📦 ");

                for (k, item) in data_items.iter().enumerate() {
                    let k_sib = k < data_items.len().saturating_sub(1);
                    self.vis_heading(&[i_sib, j_sib, k_sib], &item.entry.file_name, "💾 ");
                }
            }

            if let Some(item) = &collection.index {
                let j_sib = !collection.items.is_empty();
                self.vis_heading(&[i_sib, j_sib], &item.entry.file_name, "📑 ");
            }

            for (j, item) in collection.items.iter().enumerate() {
                let j_sib = j < collection.items.len().saturating_sub(1);
                self.vis_heading(&[i_sib, j_sib], &item.entry.file_name, "📝 ");
            }
        }
    }
}
