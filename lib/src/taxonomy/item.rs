use std::sync::Arc;

use derive_more::Debug;

use crate::fstree::{Entry, EntryId, FsTree};
use crate::taxonomy::*;

#[derive(Debug, Clone)]
pub struct Item {
    #[debug(skip)]
    pub tree: Arc<FsTree>,
    #[debug("{:?}", &tree[**id])]
    pub id: EntryId,
    // TODO: Do we need private metadata that the user can't touch?
    pub metadata: Metadata,
}

impl Item {
    pub(crate) fn new(tree: Arc<FsTree>, entry: EntryId) -> Self {
        Self {
            tree,
            id: entry,
            metadata: Metadata::new(),
        }
    }

    pub fn entry(&self) -> &Entry {
        &self.tree[self.id]
    }
}
