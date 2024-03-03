use std::path::Path;
use std::fmt::Display;

use harper::{err, error, MetaKey};
use harper::error::{Result, Error};
use harper::fstree::{EntryId, FsTree};
use harper::value::Value;

pub trait StringExt {
    fn slugify(&self) -> &str;
}

impl StringExt for str {
    fn slugify(&self) -> &str {
        self.trim_start_matches(|c: char| !c.is_ascii_alphabetic())
            .trim_end_matches(|c: char| c.is_ascii_punctuation())
    }
}

pub trait ValueExt {
    fn type_err<K: MetaKey, C: Display>(&self, k: K, context: C) -> Error;
}

impl ValueExt for Value {
    fn type_err<K: MetaKey, C: Display>(&self, _: K, context: C) -> Error {
        error! {
            "invalid metadata value type",
            "expected type" => std::any::type_name::<K::Value>(),
            "found value type" => self.kind(),
            "context" => context,
        }
    }
}

#[track_caller]
pub fn dircheck<P: AsRef<Path>>(
    tree: &FsTree,
    root: Option<EntryId>,
    path: P,
    must_exist: bool,
) -> Result<Option<EntryId>> {
    let path = path.as_ref();
    match (tree.get(root, path), must_exist) {
        (Some(e), _) if e.file_type.is_dir() => Ok(Some(e.id)),
        (Some(_) | None, false) => Ok(None),
        (Some(e), true) => err! {
            format!("{} path must point to a directory", e.file_stem()),
            "path is not a directory" => e.path.display(),
        },
        (None, true) => err! {
            format!("{} must point to an existing directory", path.display()),
            "path does not exist" => path.display(),
        },
    }
}
