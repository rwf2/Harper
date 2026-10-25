use std::sync::Arc;
use std::path::{Path, PathBuf};

use harper::{err, Site};
use harper::fstree::{Entry, EntryId, FsTree};
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
        self.build_content(&mut site, &self.tree[self.content_root], None)?;
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

    /// The name of the collection rooted at `dir`: its path relative to the
    /// content root, or `/` for the content root itself. Unique per `dir`.
    fn collection_name(&self, dir: &Entry) -> Arc<str> {
        let path = dir.path_relative_to(&self.tree[self.content_root]).unwrap();
        match path.as_os_str().is_empty() {
            true => "/".into(),
            false => path.to_string_lossy().into(),
        }
    }

    /// Adds the content in `dir` and its subdirectories to `site`. A directory
    /// with an index file roots a collection. `collection` is the root of the
    /// closest collection containing `dir`, if any.
    fn build_content(
        &self,
        site: &mut Site,
        dir: &Entry,
        collection: Option<EntryId>,
    ) -> Result<()> {
        let children = || dir.children.iter().map(|&id| &self.tree[id]);
        let is_index = |e: &Entry| e.metadata.is_file() && e.file_stem() == "index";

        let mut index_files = children().filter(|e| is_index(e));
        let collection = match index_files.next() {
            Some(index) => {
                if let Some(second) = index_files.next() {
                    return err!(
                        "found multiple index files for a single collection",
                        "faulting collection", dir.path.display(),
                        "first index" => index.path.display(),
                        "second index" => second.path.display(),
                    );
                }

                let name = || self.collection_name(dir);
                site.get_or_insert_collection(name, dir.id)?.set_index_item(index.id);
                Some(dir.id)
            }
            None => collection,
        };

        let mut files = children().filter(|e| e.metadata.is_file() && !is_index(e)).peekable();
        if files.peek().is_some() {
            let root = &self.tree[collection.unwrap_or(self.content_root)];
            let collection = site.get_or_insert_collection(|| self.collection_name(root), root.id)?;
            if root.id == dir.id {
                files.for_each(|e| { collection.new_item(e.id); });
            } else {
                collection.new_data(dir.id, files.map(|e| e.id))?;
            }
        }

        for child in children().filter(|e| e.metadata.is_dir()) {
            self.build_content(site, child, collection)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests;
