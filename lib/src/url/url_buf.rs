use std::ops::Deref;
use std::borrow::Borrow;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};

pub use super::Url;

#[derive(Debug, Default, Serialize, Deserialize)]
#[repr(transparent)]
#[serde(transparent)]
pub struct UrlBuf(String);

impl UrlBuf {
    pub fn new() -> UrlBuf {
        UrlBuf(String::new())
    }

    pub fn as_url(&self) -> &Url {
        Url::new(self.0.as_str())
    }

    pub fn into_arc_url(self) -> Arc<Url> {
        Url::from(Arc::from(self.0.into_boxed_str()))
    }

    /// ```rust
    /// use harper::url::UrlBuf;
    ///
    /// let mut url = UrlBuf::from("foo/bar");
    /// assert_eq!(url.as_str(), "foo/bar");
    ///
    /// url.prepend("/");
    /// assert_eq!(url.as_str(), "/foo/bar");
    ///
    /// url.prepend("bar/baz/");
    /// assert_eq!(url.as_str(), "bar/baz/foo/bar");
    ///
    /// url.prepend("https://rocket.rs");
    /// assert_eq!(url.as_str(), "https://rocket.rs/bar/baz/foo/bar");
    ///
    /// url.prepend("/bar/baz");
    /// assert_eq!(url.as_str(), "https://rocket.rs/bar/baz/foo/bar");
    /// ```
    // FIXME: Deal with query and hash, in `self` and `url`.
    pub fn prepend<T: AsRef<Url>>(&mut self, url: T) -> &mut Self {
        if self.scheme().is_some() {
            return self;
        }

        let mut url = url.as_ref().to_owned();
        let suffix = std::mem::replace(self, UrlBuf::new());
        url.append(suffix);
        *self = url;
        self
    }

    /// ```rust
    /// use harper::url::UrlBuf;
    ///
    /// let mut url = UrlBuf::from("https://rocket.rs");
    /// url.append("bar/baz");
    /// assert_eq!(url.as_str(), "https://rocket.rs/bar/baz");
    ///
    /// url.append("/foo/bar/");
    /// assert_eq!(url.as_str(), "https://rocket.rs/bar/baz/foo/bar/");
    ///
    /// url.append("https://rwf2.org/foo");
    /// assert_eq!(url.as_str(), "https://rwf2.org/foo");
    ///
    /// let mut url = UrlBuf::from("/foo/bar");
    /// url.append("baz");
    /// assert_eq!(url.as_str(), "/foo/bar/baz");
    ///
    /// url.append("/");
    /// assert_eq!(url.as_str(), "/foo/bar/baz/");
    /// ```
    // FIXME: Deal with query and hash, in `self` and `url`.
    pub fn append<T: AsRef<Url>>(&mut self, url: T) -> &mut Self {
        let url = url.as_ref();
        if url.scheme().is_some() {
            *self = url.to_owned();
        } else {
            match (self.ends_with('/'), url.starts_with('/')) {
                (true, true) => self.0.push_str(&url[1..]),
                (true, false) | (false, true) => self.0.push_str(&*url),
                (false, false) => {
                    self.0.push('/');
                    self.0.push_str(&*url);
                }
            }
        }

        self
    }

    pub fn extend<T: AsRef<Url>, I: IntoIterator<Item = T>>(&mut self, iter: I) -> &mut Self {
        for url in iter.into_iter() {
            self.append(url.as_ref());
        }

        self
    }

    pub fn make_absolute(&mut self) -> &mut Self {
        self.prepend("/");
        self
    }

    /// ```rust
    /// use harper::url::UrlBuf;
    ///
    /// let mut url = UrlBuf::from("https://rocket.rs/foo");
    /// url.make_relative();
    /// assert_eq!(url.as_str(), "foo");
    ///
    /// let mut url = UrlBuf::from("https://rocket.rs/foo");
    /// url.make_relative();
    /// assert_eq!(url.as_str(), "foo");
    /// ```
    pub fn make_relative(&mut self) -> &mut Self {
        let relative = self.relative_part();
        if relative.len() != self.len() {
            *self = UrlBuf::from(relative);
        }

        self
    }
}

impl From<String> for UrlBuf {
    fn from(value: String) -> Self {
        UrlBuf(value)
    }
}

impl From<&str> for UrlBuf {
    fn from(value: &str) -> Self {
        Url::new(value).to_url_buf()
    }
}

impl From<&Url> for UrlBuf {
    fn from(value: &Url) -> Self {
        value.to_url_buf()
    }
}

impl From<&Path> for UrlBuf {
    fn from(value: &Path) -> Self {
        use std::path::Component;

        let mut sane_path = PathBuf::new();
        for component in value.components() {
            match component {
                Component::Prefix(_) | Component::CurDir => continue,
                Component::RootDir => sane_path.push(component),
                Component::ParentDir => { sane_path.pop(); },
                Component::Normal(v) => match v.to_str() {
                    Some(v) => sane_path.push(v),
                    None => sane_path.push(v.to_string_lossy().to_string())
                }
            }
        }

        let string = sane_path.into_os_string().into_string();
        UrlBuf(string.expect("should contain only UTF-8"))
    }
}

impl Deref for UrlBuf {
    type Target = Url;

    fn deref(&self) -> &Self::Target {
        self.as_url()
    }
}

impl AsRef<Url> for UrlBuf {
    fn as_ref(&self) -> &Url {
        self.as_url()
    }
}

impl Borrow<Url> for UrlBuf {
    fn borrow(&self) -> &Url {
        self.as_url()
    }
}

impl AsRef<str> for UrlBuf {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Borrow<str> for UrlBuf {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl<U: AsRef<Url>> FromIterator<U> for UrlBuf {
    fn from_iter<T: IntoIterator<Item = U>>(iter: T) -> Self {
        let mut url = UrlBuf::new();
        url.extend(iter);
        url
    }
}

impl From<UrlBuf> for Arc<str> {
    fn from(value: UrlBuf) -> Self {
        value.into_arc_url().into()
    }
}

impl From<UrlBuf> for Arc<Url> {
    fn from(value: UrlBuf) -> Self {
        value.into_arc_url()
    }
}

impl From<UrlBuf> for String {
    fn from(value: UrlBuf) -> Self {
        value.0
    }
}
