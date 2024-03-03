use std::sync::Arc;
use std::ffi::OsStr;
use std::path::Path;

use super::PathStr;

pub trait FromPathStr {
    fn from_path_str(arc: Arc<PathStr>) -> Self;
}

impl FromPathStr for Arc<PathStr> {
    #[inline(always)]
    fn from_path_str(arc: Arc<PathStr>) -> Self {
        arc
    }
}

impl FromPathStr for Arc<Path> {
    #[inline(always)]
    fn from_path_str(arc: Arc<PathStr>) -> Self {
        unsafe {
            Arc::from_raw(Arc::into_raw(arc) as *const OsStr as *const Path)
        }
    }
}

impl FromPathStr for Arc<str> {
    #[inline(always)]
    fn from_path_str(arc: Arc<PathStr>) -> Self {
        unsafe {
            Arc::from_raw(Arc::into_raw(arc) as *const str)
        }
    }
}
