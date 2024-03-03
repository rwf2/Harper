use std::borrow::Cow;
use std::path::{Path, PathBuf, Component};

pub trait PathExt: AsRef<Path> {
    fn without_extension(&self) -> Cow<'_, Path>;
    fn with_root<P: AsRef<Path>>(&self, path: P) -> PathBuf;
}

impl PathExt for Path {
    fn without_extension(&self) -> Cow<'_, Path> {
        if let Some(string) = self.to_str() {
            let mut last_ext = None;
            let mut past_trailing = false;
            for (i, c) in string.bytes().enumerate().rev() {
                if std::path::is_separator(c as char) {
                    if !past_trailing { continue; }
                    break;
                }

                past_trailing = true;
                if c == b'.' {
                    last_ext = Some(i);
                }
            }

            match last_ext {
                Some(i) => Path::new(&string[..i]).into(),
                None => self.into(),
            }
        } else {
            let mut path = self.to_path_buf();
            while path.extension().is_some() {
                path = path.with_extension("");
            }

            path.into()
        }
    }

    fn with_root<P: AsRef<Path>>(&self, path: P) -> PathBuf {
        path.as_ref()
            .components()
            .chain(self.components().filter(|c| matches!(c, Component::Normal(_))))
            .collect()
    }
}
