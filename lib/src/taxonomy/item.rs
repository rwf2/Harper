use std::sync::Arc;

use crate::fstree::{EntryId, FsTree, OwnedEntry};
use crate::taxonomy::*;

#[derive(Debug, Clone)]
pub struct Item {
    pub entry: OwnedEntry,
    // TODO: Do we need private metadata that the user can't touch?
    pub metadata: Metadata,
}

impl Item {
    pub(crate) fn new(tree: Arc<FsTree>, id: EntryId) -> Self {
        Self {
            entry: OwnedEntry::new(tree, id),
            metadata: Metadata::new(),
        }
    }
}
