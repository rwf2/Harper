use std::sync::Arc;
use std::path::{Path, PathBuf};
use std::ffi::OsStr;

use super::PathStr;

pub trait TryIntoPathStr<T>: Sized {
    fn try_into_path_str(self) -> Result<T, Self>;
}

impl TryIntoPathStr<Box<PathStr>> for Box<Path> {
    #[inline(always)]
    fn try_into_path_str(self) -> Result<Box<PathStr>, Self> {
        if self.to_str().is_none() {
            return Err(self);
        }

        Ok(unsafe {
            Box::from_raw(Box::into_raw(self) as *mut OsStr as *mut PathStr)
        })
    }
}

impl TryIntoPathStr<Arc<PathStr>> for Arc<Path> {
    #[inline(always)]
    fn try_into_path_str(self) -> Result<Arc<PathStr>, Self> {
        if self.to_str().is_none() {
            return Err(self);
        }

        Ok(unsafe {
            Arc::from_raw(Arc::into_raw(self) as *const OsStr as *const PathStr)
        })
    }
}

impl TryIntoPathStr<Box<PathStr>> for PathBuf {
    #[inline(always)]
    fn try_into_path_str(self) -> Result<Box<PathStr>, Self> {
        let box_path = self.into_boxed_path();
        box_path.try_into_path_str().map_err(PathBuf::from)
    }
}
