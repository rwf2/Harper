mod try_into;
mod into;
mod into_lossy;
mod from;

use std::fmt;
use std::sync::Arc;
use std::ffi::OsStr;
use std::path::Path;

pub use from::FromPathStr;
pub use into::IntoPathStr;
pub use try_into::TryIntoPathStr;
pub use into_lossy::IntoPathStrLossy;

use serde::{Serialize, Deserialize, Deserializer, Serializer};

#[repr(transparent)]
#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct PathStr(OsStr);

impl PathStr {
    #[inline(always)]
    pub fn into<T: FromPathStr>(self: Arc<Self>) -> T {
        T::from_path_str(self)
    }

    #[inline(always)]
    pub fn as_str(&self) -> &str {
        unsafe {
            &*(self as *const PathStr as *const str)
        }
    }

    #[inline(always)]
    pub fn as_path(&self) -> &Path {
        self.as_str().as_ref()
    }

    #[inline(always)]
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_encoded_bytes()
    }

    #[inline(always)]
    pub fn deserialize<'de, D: Deserializer<'de>>(de: D) -> Result<Arc<PathStr>, D::Error> {
        Ok(<Arc<str>>::deserialize(de)?.into_path_str())
    }

    #[inline(always)]
    pub fn serialize<S: Serializer>(v: &Arc<PathStr>, s: S) -> Result<S::Ok, S::Error> {
        v.as_str().serialize(s)
    }
}

impl AsRef<str> for PathStr {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl AsRef<[u8]> for PathStr {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl fmt::Display for PathStr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)] static_assertions::assert_eq_size!(*const PathStr, *const Path);
#[cfg(test)] static_assertions::assert_eq_align!(*const PathStr, *const Path);
#[cfg(test)] static_assertions::assert_eq_size!(&PathStr, &Path);
#[cfg(test)] static_assertions::assert_eq_align!(&PathStr, &Path);
#[cfg(test)] static_assertions::assert_eq_size!(*const PathStr, *const OsStr);
#[cfg(test)] static_assertions::assert_eq_align!(*const PathStr, *const OsStr);
#[cfg(test)] static_assertions::assert_eq_size!(&PathStr, &OsStr);
#[cfg(test)] static_assertions::assert_eq_align!(&PathStr, &OsStr);
#[cfg(test)] static_assertions::assert_impl_all!(str: AsRef<Path>, AsRef<OsStr>);
