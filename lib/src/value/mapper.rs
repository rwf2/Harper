use std::sync::Arc;

use crate::error::{ErrorDetail, Result};
use crate::value::{Value, Source, Sink};

pub trait Mapper {
    type Output: Into<Value> + 'static;

    fn map<I: Source>(&self, input: I) -> Result<Self::Output>;

    fn map_copy<I: Source, O: Sink>(&self, input: I, output: O) -> Result<()> {
        output.write(self.map(input)?)
    }

    fn try_map<T: TryInto<Value>>(&self, input: T) -> Result<Self::Output> {
        self.map(input.try_into().map_err(|_| "failed to map input to value")?)
    }

    fn try_map_copy<T: TryInto<Value>, O: Sink>(&self, input: T, output: O) -> Result<()> {
        output.write(self.try_map(input)?)
    }
}

pub trait Format: Sized {
    /// The data format's error type.
    type Error: serde::de::Error + ErrorDetail + 'static;

    /// Parses `string` as the data format `Self` as a `T` or returns an error
    /// if the `string` is an invalid `T`. **_Note:_** This method is _not_
    /// intended to be called directly. Instead, it is intended to be
    /// _implemented_ and then used indirectly via the [`Data::file()`] or
    /// [`Data::string()`] methods.
    fn from_str<'de, T: serde::de::DeserializeOwned>(string: &'de str) -> Result<T, Self::Error>;

    fn read<'de, I: Source, T: serde::de::DeserializeOwned>(input: I) -> Result<T> {
        let input = input.try_read::<Arc<str>>()?;
        Ok(Self::from_str(&*input)?)
    }
}

impl<F: Format> Mapper for F {
    type Output = Value;

    fn map<I: Source>(&self, input: I) -> Result<Self::Output> {
        Self::read(input)
    }
}

#[allow(unused_macros)]
macro_rules! impl_format {
    ($name:ident : $func:expr, $E:ty) => (
        pub struct $name;

        impl Format for $name {
            type Error = $E;

            fn from_str<'de, T: serde::de::DeserializeOwned>(s: &'de str) -> Result<T, $E> {
                $func(s)
            }
        }
    );
}

impl_format!(Toml: toml::from_str, toml::de::Error);
impl_format!(Json: serde_json::from_str, serde_json::error::Error);

#[derive(Debug, Default)]
pub struct Grass {
    options: grass::Options<'static>,
}

impl Mapper for Grass {
    type Output = String;

    fn map<I: Source>(&self, input: I) -> Result<Self::Output> {
        let result = match input.path() {
            Some(path) => grass::from_path(path, &self.options),
            None => input.try_read::<Arc<str>>()
                .map(|string| grass::from_string(&*string, &self.options))?,
        };

        result.map_err(|e| error!("failed to render sass as css", e))
    }
}
