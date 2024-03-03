use std::{fs, io};
use std::path::Path;
use std::fmt::Debug;

use either::Either;

use crate::error::{Result, Chainable};
use crate::fstree::Entry;
use crate::value::{Value, Sink};

pub trait Source: Debug {
    type Value: Into<Value> + 'static;

    fn read(self) -> Result<Self::Value>;

    fn try_read<T: TryFrom<Value> + 'static>(self) -> Result<T> where Self: Sized {
        let value = self.read()?;
        if std::any::TypeId::of::<T>() == std::any::TypeId::of::<Self::Value>() {
            unsafe {
                let value_ptr = &value as *const Self::Value as *const T;
                let transmuted_value = value_ptr.read();
                std::mem::forget(value);
                return Ok(transmuted_value);
            }
        }

        value.into()
            .try_into()
            .map_err(|_| error! {
                "invalid input value type",
                "expected" => std::any::type_name::<T>(),
                "actual type" => std::any::type_name::<Self::Value>(),
            })
    }

    fn path(&self) -> Option<&Path> {
        None
    }

    #[inline]
    fn read_to<S: Sink>(self, sink: S) -> Result<()> where Self: Sized {
        sink.write(self.read()?)
    }
}

impl Source for Value {
    type Value = Self;

    fn read(self) -> Result<Self::Value> {
        Ok(self)
    }
}

impl Source for String {
    type Value = String;

    fn read(self) -> Result<Self> {
        Ok(self)
    }
}

impl Source for &fs::File {
    type Value = Either<String, Vec<u8>>;

    fn read(self) -> Result<Self::Value> {
        use io::Read;

        let mut data = Vec::new();
        let mut file = io::BufReader::new(self);
        file.read_to_end(&mut data)?;

        let value = String::from_utf8(data)
            .map(Either::Left)
            .map_err(|v| v.into_bytes())
            .unwrap_or_else(Either::Right);

        Ok(value)
    }
}

impl Source for &Path {
    type Value = <&'static fs::File as Source>::Value;

    fn read(self) -> Result<Self::Value> {
        let file = fs::File::open(self).chain(error! {
            "failed to open file for reading",
            "file path" => self.display()
        })?;

        file.read()
    }

    fn path(&self) -> Option<&Path> {
        Some(self)
    }
}

impl Source for &Entry {
    type Value = <&'static Path as Source>::Value;

    fn read(self) -> Result<Self::Value> {
        self.path.as_ref().read()
    }

    fn path(&self) -> Option<&Path> {
        Some(&*self.path)
    }
}
