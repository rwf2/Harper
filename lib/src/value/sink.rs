use std::{fs, io};
use std::path::{Path, PathBuf};
use std::fmt::Debug;

use crate::error::{Result, Chainable};
use crate::value::{Value, Source};

pub trait Sink: Debug {
    fn write<V: Into<Value> + 'static>(&self, value: V) -> Result<()> {
        self.write_value(value.into())
    }

    fn write_value(&self, value: Value) -> Result<()>;

    #[inline]
    fn write_from<'a, S: Source>(&self, source: S) -> Result<()> {
        self.write(source.read()?)
    }
}

impl Sink for fs::File {
    fn write_value(&self, value: Value) -> Result<()> {
        #[inline(always)]
        fn write_bytes(to: &mut dyn io::Write, bytes: &[u8]) -> Result<()> {
            Ok(to.write_all(bytes)?)
        }

        fn write_value(to: &mut dyn io::Write, value: &Value) -> Result<()> {
            match value {
                Value::Null => Ok(()),
                Value::Bool(b) => write_bytes(to, &[*b as u8]),
                Value::String(s) => write_bytes(to, s.as_bytes()),
                Value::Path(s) => write_bytes(to, s.as_bytes()),
                Value::Array(array) => array.iter().try_for_each(|v| write_value(to, v)),
                Value::Num(n) => {
                    match n.to_u128_strict() {
                        Ok(v) => {
                            if let Ok(v) = u8::try_from(v) {
                                write_bytes(to, &v.to_le_bytes()[..])
                            } else {
                                write_bytes(to, &v.to_le_bytes()[..])
                            }
                        },
                        Err(v) => write_bytes(to, &v.to_le_bytes()[..]),
                    }
                },
                Value::Dict(_) => Err("file endpoint does not support dictionary writes".into()),
            }
        }

        let mut file = io::BufWriter::new(self);
        write_value(&mut file, &value.into())
    }
}

impl Sink for &Path {
    fn write_value(&self, value: Value) -> Result<()> {
        fs::File::create(self)
            .chain(error! {
                "failed to open/create file for writing",
                "file path" => self.display()
            })?
            .write(value)
    }
}

impl Sink for PathBuf {
    fn write_value(&self, value: Value) -> Result<()> {
        <&Path as Sink>::write(&self.as_path(), value)
    }
}

impl<T: Sink> Sink for &T {
    fn write_value(&self, value: Value) -> Result<()> {
        <T as Sink>::write(self, value)
    }
}
