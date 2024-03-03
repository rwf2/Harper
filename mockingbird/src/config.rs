use std::sync::Arc;

use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};

use harper::url::UrlBuf;
use harper::value::{Toml, Format, Value};
use harper::fstree::{EntryId, FsTree};
use harper::error::Result;
use harper::templating::{Engine, EngineInit};

#[derive(Debug)]
pub struct Config {
    pub tree: Arc<FsTree>,
    /// The entry `Settings` was read from, if any.
    pub entry: Option<EntryId>,
    pub engine: Arc<dyn Engine>,
    pub settings: Settings,
}

#[derive(Default, Debug, Deserialize, Serialize)]
pub struct Settings {
    #[serde(default)]
    pub root: UrlBuf,
    #[serde(default)]
    pub aliases: FxHashMap<String, String>,
    #[serde(flatten)]
    pub globals: FxHashMap<String, Value>,
}

impl Config {
    pub fn discover<E: EngineInit>(tree: Arc<FsTree>) -> Result<Self> {
        let (entry, mut settings) = match tree.get(None, crate::CONFIG_FILE) {
            Some(entry) => (Some(entry.id), Toml::read(&*entry.path)?),
            None => (None, Settings::default()),
        };

        settings.root.make_absolute();
        settings.aliases.insert("".into(), settings.root.to_string());
        let templates_entry = crate::util::dircheck(&tree, None, crate::TEMPLATE_DIR, false)?;
        let engine = Arc::new(E::init(tree.clone(), templates_entry, &settings));
        Ok(Config { tree, entry, engine, settings })
    }
}
