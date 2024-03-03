mod variation;
mod macros;
mod path_ext;
mod lazy_result;

pub mod hlist;

pub use path_ext::*;
pub use macros::*;
pub use lazy_result::*;
pub use variation::*;

use std::path::{Path, PathBuf, Component};

/// Convert spaces to hyphens. Remove characters that aren't alphanumerics,
/// underscores, or hyphens. Convert to lowercase. Also strip leading and
/// trailing whitespace.
pub fn slugify(string: &str) -> String {
    let mut output = String::with_capacity(string.len());

    let mut need_dash = false;
    for ch in string.chars() {
        for b in deunicode::deunicode_char(ch).unwrap_or("-").bytes() {
            match b {
                b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_' => {
                    if need_dash {
                        output.push('-');
                        need_dash = false;
                    }

                    output.push(b.to_ascii_lowercase() as char);
                }
                _ => {
                    // This deviates from Django: all sequences of characters
                    // not alphanumeric or `_` or converted into one `-`.
                    need_dash = !output.is_empty();
                }
            }
        }
    }

    output
}

/// A helper function to determine the relative path to `path` from `base`.
///
/// Returns `None` if there is no relative path from `base` to `path`, that is,
/// `base` and `path` do not share a common ancestor. `path` and `base` must be
/// either both absolute or both relative; returns `None` if one is relative and
/// the other absolute.
///
/// ```
/// use std::path::Path;
/// use harper::util::diff_paths;
///
/// // Paths must be both relative or both absolute.
/// assert_eq!(diff_paths("/a/b/c", "b/c"), None);
/// assert_eq!(diff_paths("a/b/c", "/b/c"), None);
///
/// // The root/relative root is always a common ancestor.
/// assert_eq!(diff_paths("/a/b/c", "/b/c"), Some("../../a/b/c".into()));
/// assert_eq!(diff_paths("c/a", "b/c/a"), Some("../../../c/a".into()));
///
/// let bar = "/foo/bar";
/// let baz = "/foo/bar/baz";
/// let quux = "/foo/bar/quux";
///
/// assert_eq!(diff_paths(bar, baz), Some("../".into()));
/// assert_eq!(diff_paths(baz, bar), Some("baz".into()));
/// assert_eq!(diff_paths(quux, baz), Some("../quux".into()));
/// assert_eq!(diff_paths(baz, quux), Some("../baz".into()));
/// assert_eq!(diff_paths(bar, quux), Some("../".into()));
/// assert_eq!(diff_paths(baz, bar), Some("baz".into()));
/// ```
// Copyright 2021 Sergio Benitez
// Copyright 2012-2015 The Rust Project Developers.
// Copyright 2017 The Rust Project Developers.
// Adapted from `figment`, which adapted from `pathdiff`, which itself adapted
// from rustc's path_relative_from.
pub fn diff_paths<P, B>(path: P, base: B) -> Option<PathBuf>
     where P: AsRef<Path>, B: AsRef<Path>
{
    let (path, base) = (path.as_ref(), base.as_ref());
    if path.has_root() != base.has_root() {
        return None;
    }

    let mut ita = path.components();
    let mut itb = base.components();
    let mut comps: Vec<Component> = vec![];
    loop {
        match (ita.next(), itb.next()) {
            (None, None) => break,
            (Some(a), None) => {
                comps.push(a);
                comps.extend(ita.by_ref());
                break;
            }
            (None, _) => comps.push(Component::ParentDir),
            (Some(a), Some(b)) if comps.is_empty() && a == b => (),
            (Some(a), Some(b)) if b == Component::CurDir => comps.push(a),
            (Some(_), Some(b)) if b == Component::ParentDir => return None,
            (Some(a), Some(_)) => {
                comps.push(Component::ParentDir);
                for _ in itb {
                    comps.push(Component::ParentDir);
                }
                comps.push(a);
                comps.extend(ita.by_ref());
                break;
            }
        }
    }

    Some(comps.iter().map(|c| c.as_os_str()).collect())
}

/// Returns `true` if `input` is likely to contain a template.
pub fn is_template(input: &str) -> bool {
    let mut slice = input.as_bytes();
    while let Some(i) = memchr::memchr(b'{', slice) {
        match slice.get(i + 1) {
            Some(b'{') | Some(b'%') => return true,
            Some(_) => slice = &slice[(i + 1)..],
            None => return false,
        }
    }

    false
}

#[cfg(test)]
mod slug_tests {
    #[test]
    fn test_slugify() {
        use crate::util::slugify;

        assert_eq!(slugify("My Test String!!!1!1"), "my-test-string-1-1");
        assert_eq!(slugify("test\nit   now!"), "test-it-now");
        assert_eq!(slugify("  --test_-_cool- -  "), "test_-_cool");
        assert_eq!(slugify("Æúű--cool?"), "aeuu-cool");
        assert_eq!(slugify("You & Me"), "you-me");
        assert_eq!(slugify("  user@-- example.com  "), "user-example-com");
    }
}
