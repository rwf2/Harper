use std::ops::Deref;
use std::borrow::Borrow;
use std::sync::Arc;

pub use super::{UrlBuf, is_url_char};

#[derive(Debug)]
#[repr(transparent)]
pub struct Url(str);

impl Url {
    pub const fn new(from: &str) -> &Url {
        match Self::try_new(from) {
            Some(url) => url,
            None => panic!("invalid URL"),
        }
    }

    pub const fn try_new(from: &str) -> Option<&Url> {
        if !Self::is_valid_str(from) {
            return None;
        }

        Some(unsafe { &*(from as *const str as *const Url) })
    }

    pub fn from(arc: Arc<str>) -> Arc<Url> {
        Self::try_from(arc).expect("invalid URL")
    }

    pub fn try_from(arc: Arc<str>) -> Result<Arc<Url>, Arc<str>> {
        if !Self::is_valid_str(&*arc) {
            return Err(arc);
        }

        Ok(unsafe {
            Arc::from_raw(Arc::into_raw(arc) as *const str as *const Url)
        })
    }

    pub fn into(self: Arc<Self>) -> Arc<str> {
        unsafe {
            Arc::from_raw(Arc::into_raw(self) as *const Url as *const str)
        }
    }

    pub const fn as_str(&self) -> &str {
        &self.0
    }

    pub fn to_url_buf(&self) -> UrlBuf {
        UrlBuf::from(self.0.to_owned())
    }

    pub fn relative_part(&self) -> &Url {
        let mut url = self.as_str();
        if let Some(scheme) = self.scheme() {
            url = &url[scheme.len() + 1..];
        }

        url = url.trim_start_matches('/');
        Url::new(url)
    }

    /// ```rust
    /// use harper::url::Url;
    ///
    /// let url = Url::new("http://rocket.rs");
    /// assert_eq!(url.scheme(), Some("http"));
    ///
    /// let url = Url::new("ftp:/rocket.rs");
    /// assert_eq!(url.scheme(), Some("ftp"));
    ///
    /// let url = Url::new("mailto:foo@bar.com");
    /// assert_eq!(url.scheme(), Some("mailto"));
    ///
    /// let url = Url::new("foo#bar:baz");
    /// assert_eq!(url.scheme(), None);
    ///
    /// let url = Url::new("foo?bar:baz");
    /// assert_eq!(url.scheme(), None);
    /// ```
    pub fn scheme(&self) -> Option<&str> {
        let bytes = self.as_bytes();
        match memchr::memchr3(b':', b'?', b'/', bytes) {
            Some(i) if bytes[i] == b':' => match memchr::memrchr(b'#', &bytes[..i]) {
                Some(_) => None,
                None => Some(&self[..i]),
            }
            _ => None,
        }
    }

    const fn is_valid_str(string: &str) -> bool {
        let mut i = 0;
        let bytes = string.as_bytes();
        while i < bytes.len() {
            if !is_url_char(&bytes[i]) {
                return false;
            }

            i += 1;
        }

        true
    }

    pub const fn is_valid(&self) -> bool {
        Self::is_valid_str(self.as_str())
    }

    pub fn is_absolute(&self) -> bool {
        self.starts_with('/') || self.scheme().is_some()
    }

    pub fn is_relative(&self) -> bool {
        !self.is_absolute()
    }
}

impl<'a> From<&'a str> for &'a Url {
    fn from(value: &'a str) -> Self {
        Url::new(value)
    }
}

impl Deref for Url {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl Borrow<str> for Url {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl AsRef<str> for Url {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl AsRef<Url> for str {
    fn as_ref(&self) -> &Url {
        Url::new(self)
    }
}

impl AsRef<Url> for Arc<str> {
    fn as_ref(&self) -> &Url {
        Url::new(&*self)
    }
}

impl AsRef<Url> for Url {
    fn as_ref(&self) -> &Url {
        self
    }
}

impl ToOwned for Url {
    type Owned = UrlBuf;

    fn to_owned(&self) -> Self::Owned {
        self.to_url_buf()
    }
}
