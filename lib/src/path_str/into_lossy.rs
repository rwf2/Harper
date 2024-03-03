use std::sync::Arc;
use std::path::{Path, PathBuf};
use std::ffi::OsStr;

use super::{IntoPathStr as _, PathStr};

pub trait IntoPathStrLossy: Sized {
    fn into_path_str_lossy(self) -> Arc<PathStr>;
}

impl IntoPathStrLossy for Arc<Path> {
    #[inline(always)]
    fn into_path_str_lossy(self) -> Arc<PathStr> {
        if self.to_str().is_some() {
            unsafe {
                Arc::from_raw(Arc::into_raw(self) as *const OsStr as *const PathStr)
            }
        } else {
            self.to_string_lossy().to_string().into_path_str()
        }
    }
}

impl IntoPathStrLossy for Box<Path> {
    #[inline(always)]
    fn into_path_str_lossy(self) -> Arc<PathStr> {
        if self.to_str().is_some() {
            let boxed = unsafe {
                Box::from_raw(Box::into_raw(self) as *mut OsStr as *mut PathStr)
            };

            Arc::from(boxed)
        } else {
            self.to_string_lossy().to_string().into_path_str()
        }
    }
}

impl IntoPathStrLossy for PathBuf {
    #[inline(always)]
    fn into_path_str_lossy(self) -> Arc<PathStr> {
        self.into_boxed_path().into_path_str_lossy()
    }
}
