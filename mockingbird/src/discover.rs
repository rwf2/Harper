use std::sync::Arc;
use std::path::{Path, PathBuf};

use harper::{err, Collection, Site};
use harper::fstree::{EntryId, FsTree};
use harper::templating::EngineInit;
use harper::error::Result;
use harper::templating::minijinja::MiniJinjaEngine;

use crate::{ASSETS_DIR, CONTENT_DIR, TEMPLATE_DIR, PermaPath};
use crate::config::Config;
use crate::util::dircheck;

#[derive(Debug)]
pub struct Mockingbird {
    pub tree: Arc<FsTree>,
    pub config: Config,
    pub output: PathBuf,
    pub content_root: EntryId,
    pub template_root: Option<EntryId>,
    pub asset_root: Option<EntryId>,
}

impl Mockingbird {
    pub fn new<E, I, O>(input: I, output: O) -> Result<Self>
        where I: AsRef<Path>, O: AsRef<Path>, E: EngineInit
    {
        let tree = Arc::new(FsTree::build(input)?);
        Ok(Mockingbird {
            output: output.as_ref().to_path_buf(),
            content_root: dircheck(&tree, None, CONTENT_DIR, true)?.unwrap(),
            template_root: dircheck(&tree, None, TEMPLATE_DIR, false)?,
            asset_root: dircheck(&tree, None, ASSETS_DIR, false)?,
            config: Config::discover::<MiniJinjaEngine>(tree.clone())?,
            tree,
        })
    }

    pub fn discover(&self) -> Result<Site> {
        let mut site = Site::new(self.tree.clone());
        self.build_site_items(&mut site);
        self.build_collections(&mut site)?;
        self.build_items(&mut site)?;
        Ok(site)
    }

    fn build_site_items(&self, site: &mut Site) {
        let hidden = |filename: &str| filename.starts_with(".")
            || filename.eq_ignore_ascii_case("include")
            || filename.eq_ignore_ascii_case("includes");

        let asset_root = match self.asset_root {
            Some(id) => &self.tree[id],
            None => return
        };

        self.tree.depth_first_search(asset_root.id, |entry| {
            if hidden(&entry.file_name) {
                return false;
            }

            if entry.file_type.is_file() {
                let item = site.new_resource(entry.id);
                let permapath = entry.path_relative_to(asset_root).unwrap();
                item.metadata.insert(PermaPath, permapath);
            }

            true
        });
    }

    fn build_collections(&self, site: &mut Site) -> Result<()> {
        let content_root = &self.tree[self.content_root];
        // TODO: Provide a parallel iterator here?
        let index_files = self.tree.iter_breadth_first(content_root.id)
            .files()
            .filter(|e| e.file_stem() == "index");

        // Find all collections, as identified by the presence of an index file.
        for index in index_files {
            let group_dir = &self.tree[index.parent.unwrap()];
            let collection = site.get_or_insert_collection(|| {
                group_dir.path_relative_to(content_root)
                    .unwrap()
                    .to_string_lossy()
                    .into()
            }, group_dir.id);

            if let Some(ref existing) = collection.index {
                return err!(
                    "found multiple index files for a single collection",
                    "faulting collection", group_dir.path.display(),
                    "first index" => existing.entry.path.display(),
                    "second index" => index.path.display(),
                );
            }

            collection.set_index_item(index.id);
        }

        Ok(())
    }

    fn parent<'a>(&self, site: &'a mut Site, mut entry: EntryId) -> Option<&'a mut Collection> {
        loop {
            let parent = self.tree[entry].parent?;
            if site.collections.contains_key(&parent) {
                let c = site.collections.get_mut(&parent)?;
                return Some(Arc::get_mut(c).expect("&mut -> &mut"));
            }

            entry = parent;
        }
    }

    fn build_items(&self, site: &mut Site) -> Result<()> {
        let tree = self.tree.clone();
        let content_root = &tree[self.content_root];
        let files = tree.iter_breadth_first(content_root.id).files()
            .filter(|e| e.file_stem() != "index");

        for entry in files {
            let collection = match self.parent(site, entry.id) {
                Some(collection) => collection,
                None => site.get_or_insert_collection(|| "/".into(), content_root.id),
            };

            if entry.depth - collection.entry.depth <= 1 {
                collection.new_item(entry.id);
            } else {
                collection.new_datum(entry.parent.unwrap(), entry.id);
            };
        }

        Ok(())
	}
}
