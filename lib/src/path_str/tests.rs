use super::PathStr;
use std::ffi::OsStr;
use std::path::Path;

static_assertions::assert_eq_size!(*const PathStr, *const Path);
static_assertions::assert_eq_align!(*const PathStr, *const Path);
static_assertions::assert_eq_size!(&PathStr, &Path);
static_assertions::assert_eq_align!(&PathStr, &Path);
static_assertions::assert_eq_size!(*const PathStr, *const OsStr);
static_assertions::assert_eq_align!(*const PathStr, *const OsStr);
static_assertions::assert_eq_size!(&PathStr, &OsStr);
static_assertions::assert_eq_align!(&PathStr, &OsStr);
static_assertions::assert_impl_all!(str: AsRef<Path>, AsRef<OsStr>);
