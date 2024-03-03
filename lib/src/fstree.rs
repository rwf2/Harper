use std::sync::Arc;
use std::path::Path;
use std::collections::VecDeque;
use std::{fs, fmt};

use rustc_hash::FxHashMap;

use crate::error::Result;

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct EntryId(pub(crate) usize);

#[derive(Debug)]
pub struct FsTree {
    entries: Vec<Entry>,
    map: FxHashMap<Arc<Path>, EntryId>,
}

pub struct FsSubTree<'a> {
    tree: &'a FsTree,
    root: EntryId
}

#[derive(Debug)]
pub struct Entry {
    pub id: EntryId,
    pub path: Arc<Path>,
    pub metadata: fs::Metadata,
    pub file_name: String,
    pub file_type: fs::FileType,
    pub parent: Option<EntryId>,
    pub children: Vec<EntryId>,
    pub depth: usize,
}

#[derive(Default, Debug)]
struct FsMetadata(Option<fs::Metadata>);

impl FsTree {
    fn new() -> Self {
        Self {
            map: FxHashMap::default(),
            entries: vec![],
        }
    }

    pub fn build<P: AsRef<Path>>(root: P) -> Result<Self> {
        Self::build_with(root.as_ref(), |_, _| Ok(()))
    }

    pub fn build_with<P, F>(root: P, mut callback: F) -> Result<Self>
        where P: AsRef<Path>,
              F: FnMut(&Self, EntryId) -> Result<()>,
    {
        use jwalk::WalkDirGeneric;

        let root = root.as_ref();
        let walker = WalkDirGeneric::<FsMetadata>::new(root)
            .follow_links(true)
            .process_read_dir(|_, _, _, entries| {
                entries.iter_mut()
                    .filter_map(|e| e.as_mut().ok())
                    .for_each(|e| e.client_state = FsMetadata(e.metadata().ok()))
            });

        let mut tree: FsTree = FsTree::new();
        for f in walker.into_iter().filter_map(|e| e.ok()).filter(|e| e.client_state.0.is_some()) {
            let id = tree.insert(f);
            callback(&mut tree, id)?;
        }

        if tree.len() == 0 {
            return err! {
                "file system tree discovery yielded zero files",
                "search root" => root.display(),
            }
        }

        Ok(tree)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn root(&self) -> &Entry {
        &self[self.root_id()]
    }

    pub fn root_id(&self) -> EntryId {
        EntryId(0)
    }

    #[inline]
    pub fn subtree(&self, root: EntryId) -> FsSubTree<'_> {
        assert!(self[root].id == root);
        FsSubTree { tree: self, root }
    }

    #[inline]
    pub fn get<R, P>(&self, root: R, path: P) -> Option<&Entry>
        where R: Into<Option<EntryId>>, P: AsRef<Path>
    {
        self.get_id(root.into(), path.as_ref()).map(|id| &self[id])
    }

    pub fn get_file_id<R, P>(&self, root: R, path: P) -> Option<EntryId>
        where R: Into<Option<EntryId>>, P: AsRef<Path>
    {
        let id = self.get_id(root.into(), path.as_ref())?;
        self[id].file_type.is_file().then_some(id)
    }

    pub fn get_id<R, P>(&self, root: R, path: P) -> Option<EntryId>
        where R: Into<Option<EntryId>>, P: AsRef<Path>
    {
        let root = root.into().unwrap_or(self.root_id());
        let full_path = self[root].path.join(path.as_ref());
        self.map.get(&*full_path).cloned()
    }

    pub fn ancestors_of(&self, mut entry: EntryId) -> impl Iterator<Item = EntryId> + '_ {
        std::iter::from_fn(move || {
            let parent = self[entry].parent?;
            entry = parent;
            Some(parent)
        })
    }

    pub fn iter(&self) -> impl Iterator<Item = &Entry> {
        (0..self.entries.len()).map(|i| &self[EntryId(i)])
    }

    pub fn iter_breadth_first(&self, root: EntryId) -> Bfs<'_> {
        Bfs {
            tree: self,
            root: Some(root),
            stack: VecDeque::new(),
            progress: 0
        }
    }

    pub fn iter_depth_first(&self, root: EntryId) -> Dfs<'_> {
        Dfs {
            tree: self,
            stack: { let mut q = VecDeque::new(); q.push_back(root); q },
        }
    }

    pub fn depth_first_search<F>(&self, root: EntryId, mut progress: F)
        where F: FnMut(&Entry) -> bool
    {
        fn _dfs<F: FnMut(&Entry) -> bool>(tree: &FsTree, root: EntryId, progress: &mut F) {
            let entry = &tree[root];
            if progress(entry) {
                for &child in &entry.children {
                    _dfs(tree, child, progress)
                }
            }
        }

        _dfs(self, root, &mut progress)
    }

    pub fn search<P: AsRef<Path>>(&self, path: P) -> Option<EntryId> {
        let root_path = &self[self.root_id()].path;
        for id in self.iter_breadth_first(self.root_id()) {
            let entry = &self[id];
            let suffix = entry.path.strip_prefix(&root_path).unwrap();
            if path.as_ref() == suffix {
                return Some(id);
            }
        }

        None
    }

    #[inline]
    pub fn search_entry<P: AsRef<Path>>(&self, path: P) -> Option<&Entry> {
        self.search(path.as_ref()).map(|id| &self[id])
    }

    /// Returns `true` `iff` `to` is or is a descendent of `from`.
    pub fn path_exists(&self, from: EntryId, mut to: EntryId) -> bool {
        // simple case: path determines if it descends
        if self[to].path.starts_with(&self[from].path) {
            return true;
        }

        // harder case: path isn't reliable (ie symlinks)
        loop {
            if from == to {
                return true;
            }

            if self[from].depth >= self[to].depth {
                return false;
            }

            match self[to].parent {
                Some(id) => to = id,
                None => return false
            }
        }
    }

    fn insert(&mut self, entry: jwalk::DirEntry<FsMetadata>) -> EntryId {
        let entry = Entry {
            id: EntryId(self.entries.len()),
            path: Arc::from(entry.path().into_boxed_path()),
            metadata: entry.client_state.0.unwrap(),
            file_type: entry.file_type,
            file_name: entry.file_name.to_string_lossy().into_owned(),
            parent: self.map.get(&entry.parent_path).cloned(),
            children: vec![],
            depth: entry.depth,
        };

        self.map.insert(entry.path.clone(), entry.id);
        if let Some(parent) = entry.parent {
            self.entries[parent.0].children.push(entry.id);
        }

		let id = entry.id;
        self.entries.push(entry);
		id
    }
}

impl FsSubTree<'_> {
    pub fn get<P: AsRef<Path>>(&self, path: P) -> Option<&Entry> {
        self.tree.get(self.root, path.as_ref())
    }

    #[inline]
    pub fn get_file_id<P: AsRef<Path>>(&self, path: P) -> Option<EntryId> {
        self.tree.get_file_id(self.root, path.as_ref())
    }

    #[inline]
    pub fn get_id<P: AsRef<Path>>(&self, path: P) -> Option<EntryId> {
        self.tree.get_id(self.root, path.as_ref())
    }
}

impl Entry {
    /// File name without the extension.
    pub fn file_stem(&self) -> &str {
        match self.file_name.rsplit_once('.') {
            Some((left, _)) => left,
            None => &self.file_name,
        }
    }

    /// The complete extension, if any.
    pub fn file_ext(&self) -> Option<&str> {
        self.file_name.rsplit_once('.').map(|(_, right)| right)
    }

    /// Path relative to the root tree of `self`.
    pub fn relative_path(&self) -> &Path {
        let mut components = self.path.components();
        for _ in 0..(self.path.components().count() - self.depth) {
            components.next();
        }

        components.as_path()
    }

    /// Path relative to `other`. `self` must be super-path of `other`.
    pub fn path_relative_to(&self, other: &Entry) -> Option<&Path> {
        if !self.path.starts_with(&other.path) {
            return None;
        }

        let n = self.depth - other.depth;
        let mut components = self.path.components();
        for _ in 0..(self.path.components().count() - n) {
            components.next();
        }

        Some(components.as_path())
    }
}

// This implementation is more memory efficient than the usual since it doesn't
// store all of the potential visits in the stack, instead only storing visits
// that can yield further children. This bounds the length of `stack`. The cost
// is a more complex implementation.
pub struct Bfs<'a> {
    tree: &'a FsTree,
    root: Option<EntryId>,
    stack: VecDeque<EntryId>,
    progress: usize,
}

impl Iterator for Bfs<'_> {
    type Item = EntryId;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(root) = self.root.take() {
                self.stack.push_front(root);
                return Some(root);
            }

            if self.stack.is_empty() {
                return None;
            }

            if let Some(&parent) = self.stack.front() {
                let children = &self.tree[parent].children;
                if self.progress < children.len() {
                    let node = children[self.progress];
                    if !self.tree[node].children.is_empty() {
                        self.stack.push_back(node);
                    }

                    self.progress += 1;
                    return Some(node)
                } else {
                    self.stack.pop_front();
                    self.progress = 0;
                }
            }
        }
    }
}

impl<'a> Bfs<'a> {
    #[inline]
    pub fn entries(self) -> impl Iterator<Item = &'a Entry> {
        let tree = self.tree;
        self.into_iter().map(move |id| &tree[id])
    }

    #[inline]
    pub fn files(self) -> impl Iterator<Item = &'a Entry> {
        let tree = self.tree;
        self.into_iter().map(move |id| &tree[id]).filter(|e| e.metadata.is_file())
    }
}

pub struct Dfs<'a> {
    tree: &'a FsTree,
    stack: VecDeque<EntryId>,
}

impl<'a> Dfs<'a> {
    #[inline]
    pub fn entries(self) -> impl Iterator<Item = &'a Entry> {
        let tree = self.tree;
        self.into_iter().map(move |id| &tree[id])
    }

    #[inline]
    pub fn files(self) -> impl Iterator<Item = &'a Entry> {
        let tree = self.tree;
        self.into_iter().map(move |id| &tree[id]).filter(|e| e.metadata.is_file())
    }
}

impl Iterator for Dfs<'_> {
    type Item = EntryId;

    fn next(&mut self) -> Option<Self::Item> {
        let node = self.stack.pop_front()?;
        for &child in &self.tree[node].children {
            self.stack.push_front(child);
        }

        Some(node)
    }
}

impl jwalk::ClientState for FsMetadata {
    type ReadDirState = ();
    type DirEntryState = Self;
}

impl std::ops::Index<EntryId> for FsTree {
    type Output = Entry;

    fn index(&self, index: EntryId) -> &Self::Output {
        &self.entries[index.0]
    }
}

impl std::ops::IndexMut<EntryId> for FsTree {
    fn index_mut(&mut self, index: EntryId) -> &mut Self::Output {
        &mut self.entries[index.0]
    }
}

impl fmt::Debug for EntryId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
