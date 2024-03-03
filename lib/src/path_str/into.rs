use std::sync::Arc;
use std::ffi::OsStr;

use super::PathStr;

pub trait IntoPathStr<T>: Sized {
    fn into_path_str(self) -> T;
}

impl IntoPathStr<Arc<PathStr>> for Arc<PathStr> {
    #[inline(always)]
    fn into_path_str(self) -> Arc<PathStr> {
        self
    }
}

impl IntoPathStr<Arc<PathStr>> for Arc<str> {
    #[inline(always)]
    fn into_path_str(self) -> Arc<PathStr> {
        unsafe {
            Arc::from_raw(Arc::into_raw(self) as *const OsStr as *const PathStr)
        }
    }
}

impl IntoPathStr<Box<PathStr>> for Box<str> {
    #[inline(always)]
    fn into_path_str(self) -> Box<PathStr> {
        unsafe {
            Box::from_raw(Box::into_raw(self) as *mut OsStr as *mut PathStr)
        }
    }
}

impl IntoPathStr<Arc<PathStr>> for Box<str> {
    #[inline(always)]
    fn into_path_str(self) -> Arc<PathStr> {
        let boxed: Box<PathStr> = self.into_path_str();
        Arc::from(boxed)
    }
}

impl IntoPathStr<Box<PathStr>> for String {
    #[inline(always)]
    fn into_path_str(self) -> Box<PathStr> {
        self.into_boxed_str().into_path_str()
    }
}

impl IntoPathStr<Arc<PathStr>> for String {
    #[inline(always)]
    fn into_path_str(self) -> Arc<PathStr> {
        self.into_boxed_str().into_path_str()
    }
}
