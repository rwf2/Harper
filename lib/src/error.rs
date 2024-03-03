use std::{fmt, io};
use std::panic::Location;
use std::convert::Infallible;
use std::error::Error as StdError;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug)]
pub struct Error {
    detail: Vec<Box<dyn ErrorDetail>>,
    prev: Option<Box<Error>>,
    _location: &'static Location<'static>,
}

pub trait ErrorDetail: fmt::Display + fmt::Debug + Send + Sync {
    fn context(&self) -> Vec<(Option<String>, String)> { vec![] }
}

impl Error {
    #[track_caller]
    pub fn from_std<E>(error: E) -> Self
        where E: StdError + Send + Sync + 'static
    {
        Error::from(Box::new(error) as Box<dyn StdError + Send + Sync>)
    }

    pub fn from_detail(detail: &dyn ErrorDetail) -> Self {
        Error::from(MakeshiftError::from(detail))
    }

    pub fn chain(self, mut other: Error) -> Self {
        #[inline]
        fn _chain(error: Error, behind: &mut Error) {
            if let Some(prev) = behind.prev.as_mut() {
                _chain(error, prev);
            } else {
                behind.prev = Some(Box::new(error));
            }
        }

        _chain(self, &mut other);
        other
    }
}

impl ErrorDetail for &(dyn StdError + Send + Sync) {
    fn context(&self) -> Vec<(Option<String>, String)> {
        let mut ctxt = vec![];
        let mut error = self.source();
        while let Some(e) = error {
            ctxt.push((None, e.to_string()));
            error = e.source();
        }

        ctxt
    }
}

impl ErrorDetail for Box<dyn StdError + Send + Sync> {
    fn context(&self) -> Vec<(Option<String>, String)> {
        let error: &(dyn StdError + Send + Sync) = &**self;
        error.context()
    }
}

impl<E: StdError + Send + Sync> ErrorDetail for Box<E> {
    fn context(&self) -> Vec<(Option<String>, String)> {
        let error: &(dyn StdError + Send + Sync) = &**self;
        error.context()
    }
}

macro_rules! impl_error_detail_with_std_error {
    ($T:ty) => {
        impl $crate::error::ErrorDetail for $T {
            fn context(&self) -> Vec<(Option<String>, String)> {
                let error: &(dyn std::error::Error + Send + Sync) = self;
                error.context()
            }
        }
    }
}

impl_error_detail_with_std_error!(io::Error);
impl_error_detail_with_std_error!(toml::de::Error);
impl_error_detail_with_std_error!(serde_json::Error);

impl ErrorDetail for String { }
impl ErrorDetail for &str { }

impl Clone for Error {
    fn clone(&self) -> Self {
        Error {
            detail: self.detail.iter()
                .map(|detail| MakeshiftError::from(&**detail))
                .map(|error| Box::new(error) as Box<dyn ErrorDetail>)
                .collect(),
            prev: self.prev.clone(),
            _location: self._location,
        }
    }
}

impl<T: ErrorDetail + 'static> From<T> for Error {
    #[track_caller]
    fn from(detail: T) -> Self {
        Error {
            prev: None,
            detail: vec![Box::new(detail)],
            _location: std::panic::Location::caller(),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        #[derive(Copy, Clone)] struct Indent(usize);

        impl fmt::Display for Indent {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                for _ in 0..(self.0 * 4) { write!(f, " ")? }
                Ok(())
            }
        }

        struct NestedError<'a>(Indent, &'a Error);

        impl fmt::Display for NestedError<'_> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                let NestedError(indent, e) = self;

                for detail in &e.detail {
                    let indent_line = format!("\n{indent}");

                    writeln!(f, "{indent}{}", format!("{:#}", detail).replace('\n', &indent_line))?;
                    if let Some(prev) = &e.prev {
                        NestedError(Indent(indent.0 + 1), prev).fmt(f)?;
                    }

                    for (key, value) in detail.context() {
                        let value = value.to_string().replace('\n', &indent_line);
                        if let Some(key) = key {
                            writeln!(f, "{indent}{key}: {value}")?;
                        } else {
                            writeln!(f, "{indent}{value}")?;
                        }
                    }

                    if std::env::var_os("RUST_BACKTRACE").is_some() {
                        writeln!(f, "{indent}[{}]", e._location)?;
                    }
                }

                Ok(())
            }
        }

        NestedError(Indent(0), self).fmt(f)
    }
}

#[derive(Debug)]
pub struct MakeshiftError {
    pub message: String,
    pub parameters: Vec<(Option<String>, String)>,
}

impl From<&dyn ErrorDetail> for MakeshiftError {
    #[inline]
    fn from(detail: &dyn ErrorDetail) -> Self {
        MakeshiftError {
            message: detail.to_string(),
            parameters: detail.context()
        }
    }
}

#[doc(hidden)]
#[macro_export]
macro_rules! err {
    ($($token:tt)*) => (Err($crate::error!($($token)*)));
}

#[doc(hidden)]
#[macro_export]
macro_rules! error {
    ($msg:expr, $($rest:tt)*) => (
        $crate::error::Error::from($crate::error::MakeshiftError {
            message: $msg.to_string(),
            parameters: {
                #[allow(unused_mut)]
                let mut v: Vec<(Option<String>, String)> = Vec::new();
                $crate::error!(@param v $($rest)*);
                v
            },
        })
    );

    ($msg:expr) => ( error!($msg,) );

    (@param $v:ident if $cond:expr => $value:expr $(, $rest:tt)*) => {
        if $cond {
            $v.push((None, $value.to_string()));
        }

        error!(@param $v $($rest)*);
    };

    (@param $v:ident if $cond:expr => $key:expr => $value:expr, $($rest:tt)*) => {
        $crate::error!(@param $v if $cond => $key => $value);
        $crate::error!(@param $v $($rest)*);
    };

    (@param $v:ident if $cond:expr => $key:expr => $value:expr) => {
        if $cond {
            $crate::error!(@param $v $key => $value);
        }
    };

    (@param $v:ident $key:expr => $value:expr, $($rest:tt)*) => {
        $crate::error!(@param $v $key => $value);
        $crate::error!(@param $v $($rest)*);
    };

    (@param $v:ident $key:expr => $value:expr) => {
        $v.push((Some($key.to_string()), $value.to_string()));
    };

    (@param $v:ident $value:expr, $($rest:tt)*) => {
        $crate::error!(@param $v $value);
        $crate::error!(@param $v $($rest)*);
    };

    (@param $v:ident $value:expr) => {
        $v.push((None, $value.to_string()));
    };

    (@param $v:ident $(,)?) => { };
}

impl fmt::Display for MakeshiftError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.message.fmt(f)
    }
}

impl ErrorDetail for MakeshiftError {
    fn context(&self) -> Vec<(Option<String>, String)> {
        self.parameters.clone()
    }
}

pub trait Chainable<T> {
    fn chain(self, other: impl Into<Error>) -> Result<T>;

    fn chain_with<F, E>(self, f: F) -> Result<T>
        where F: FnOnce() -> E, E: Into<Error>;
}

impl<T, E: Into<Error>> Chainable<T> for Result<T, E> {
    #[track_caller]
    fn chain(self, other: impl Into<Error>) -> Result<T> {
        match self {
            Ok(v) => Ok(v),
            Err(e) => Err(e.into().chain(other.into()))
        }
    }

    fn chain_with<F, Err>(self, f: F) -> Result<T>
        where F: FnOnce() -> Err, Err: Into<Error>,
     {
        match self {
            Ok(v) => Ok(v),
            Err(e) => Err(e.into().chain(f().into()))
        }
    }
}

impl ErrorDetail for Infallible {
    fn context(&self) -> Vec<(Option<String>, String)> { vec![] }
}
