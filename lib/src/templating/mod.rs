#[cfg(feature = "plugins")]
pub mod plugins;
pub mod minijinja;

use std::fmt::Debug;
use std::sync::Arc;

use serde::Serialize;

use crate::error::Result;
use crate::fstree::{EntryId, FsTree};
use crate::taxonomy::{Site, Item, Collection, Metadata};

pub trait EngineInit {
    type Engine: Engine + 'static;

    fn init<G: Serialize>(tree: Arc<FsTree>, root: Option<EntryId>, globals: G) -> Self::Engine;
}

pub trait Engine: Send + Sync + Debug {
    fn render(
        &self,
        name: &str,
        site: &Arc<Site>,
        collection: Option<&Arc<Collection>>,
        item: &Arc<Item>,
    ) -> Result<String>;

    fn render_raw(
        &self,
        name: Option<&str>,
        template_str: &str,
        site: &Arc<Site>,
        collection: Option<&Arc<Collection>>,
        item: &Arc<Item>,
    ) -> Result<String>;

    fn render_str(
        &self,
        name: Option<&str>,
        template_str: &str,
        meta: Metadata,
    ) -> Result<String>;
}
