use std::path::{Path, PathBuf};
use std::{hash::Hash, sync::Arc};
use std::collections::BTreeMap;

use either::Either;
use serde::{Serialize, Deserialize};

use crate::path_str::*;
use crate::url::{Url, UrlBuf};

pub type Dict<K = Arc<str>, V = Value> = BTreeMap<K, V>;

/// Represents any valid value.
#[derive(Debug, Serialize, Hash, Deserialize, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[serde(untagged)]
pub enum Value {
    Null,
    Bool(bool),
    Num(Num),
    String(Arc<str>),
    #[serde(serialize_with = "PathStr::serialize")]
    #[serde(deserialize_with = "PathStr::deserialize")]
    Path(Arc<PathStr>),
    Array(Arc<Vec<Value>>),
    Dict(Arc<Dict>),
    // TODO: IDEA: Make `Item` and `Collection` something you can put in a
    // value. Then do the generational-on-clone metadata thing.
}

impl Value {
    pub fn to_null(&self) -> Option<()> {
        match self {
            Value::Null => Some(()),
            _ => None
        }
    }

    pub fn to_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None
        }
    }

    pub fn to_num(&self) -> Option<Num> {
        match self {
            Value::Num(n) => Some(*n),
            _ => None
        }
    }

    pub fn into_str(self) -> Result<Arc<str>, Value> {
        match self {
            Value::String(s) => Ok(s.clone()),
            _ => Err(self),
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(&**s),
            _ => None
        }
    }

    pub fn into_vec(self) -> Result<Arc<Vec<Value>>, Value> {
        match self {
            Value::Array(v) => Ok(v),
            _ => Err(self)
        }
    }

    pub fn as_slice(&self) -> Option<&[Value]> {
        match self {
            Value::Array(v) => Some(v.as_slice()),
            _ => None
        }
    }

    pub fn as_dict(&self) -> Option<&Dict> {
        match self {
            Value::Dict(v) => Some(&**v),
            _ => None
        }
    }

    pub fn into_dict(self) -> Result<Arc<Dict>, Value> {
        match self {
            Value::Dict(v) => Ok(v),
            _ => Err(self)
        }
    }

    pub fn into_path(self) -> Result<Arc<PathStr>, Value> {
        match self {
            Value::Path(v) => Ok(v),
            _ => Err(self)
        }
    }

    pub fn kind(&self) -> &'static str {
        match self {
            Value::Null => "null",
            Value::Bool(_) => "boolean",
            Value::Num(_) => "number",
            Value::String(_) => "string",
            Value::Path(_) => "path",
            Value::Array(_) => "array",
            Value::Dict(_) => "dict",
        }
    }
}

macro_rules! impl_from_primitive {
    ($($T:ty),+ => $E:ident::$kind:ident) => {
        $(
            impl From<$T> for $E {
                fn from(value: $T) -> Self {
                    $E::$kind(value.into())
                }
            }
        )+
    };
}

impl_from_primitive!(bool => Value::Bool);
impl_from_primitive!(&str => Value::String);
impl_from_primitive!(std::borrow::Cow<'_, str> => Value::String);
impl_from_primitive!(String => Value::String);
impl_from_primitive!(Arc<str> => Value::String);
impl_from_primitive!(Arc<Url> => Value::String);
impl_from_primitive!(Arc<PathStr> => Value::Path);
impl_from_primitive!(Arc<Vec<Value>> => Value::Array);
impl_from_primitive!(Arc<Dict> => Value::Dict);
impl_from_primitive!(u8, u16, u32, u64, u128, usize => Value::Num);
impl_from_primitive!(i8, i16, i32, i64, i128, isize => Value::Num);

impl From<()> for Value  {
    fn from(_: ()) -> Self {
        Value::Null
    }
}

impl<A, B> From<Either<A, B>> for Value where Value: From<A>, Value: From<B> {
    fn from(value: Either<A, B>) -> Self {
        either::for_both!(value, v => v.into())
    }
}

impl<T> From<Option<T>> for Value where Value: From<T> {
    fn from(value: Option<T>) -> Self {
        value.map(Value::from).unwrap_or(Value::Null)
    }
}

impl From<Arc<Path>> for Value {
    fn from(value: Arc<Path>) -> Self {
        Value::Path(value.into_path_str_lossy())
    }
}

impl From<PathBuf> for Value {
    fn from(value: PathBuf) -> Self {
        Value::Path(value.into_path_str_lossy())
    }
}

impl From<UrlBuf> for Value {
    fn from(value: UrlBuf) -> Self {
        Value::String(value.into_arc_url().into())
    }
}

impl<T> From<Vec<T>> for Value where Value: From<T> {
    fn from(value: Vec<T>) -> Self {
        value.into_iter()
            .map(Value::from)
            .collect()
    }
}

impl<K, V> From<Dict<K, V>> for Value where Arc<str>: From<K>, Value: From<V> {
    fn from(value: Dict<K, V>) -> Self {
        let dict = value.into_iter()
            .map(|(k, v)| (<Arc::<str>>::from(k), Value::from(v)))
            .collect::<Dict>();

        Value::Dict(Arc::new(dict))
    }
}

impl FromIterator<Value> for Value {
    fn from_iter<T: IntoIterator<Item = Value>>(iter: T) -> Self {
        let vec = iter.into_iter().collect::<Vec<Value>>();
        Value::Array(Arc::from(vec))
    }
}

/// A signed or unsigned numeric value.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Num {
    /// An 8-bit unsigned integer.
    U8(u8),
    /// A 16-bit unsigned integer.
    U16(u16),
    /// A 32-bit unsigned integer.
    U32(u32),
    /// A 64-bit unsigned integer.
    U64(u64),
    /// A 128-bit unsigned integer.
    U128(u128),
    /// An unsigned integer of platform width.
    USize(usize),
    /// An 8-bit signed integer.
    I8(i8),
    /// A 16-bit signed integer.
    I16(i16),
    /// A 32-bit signed integer.
    I32(i32),
    /// A 64-bit signed integer.
    I64(i64),
    /// A 128-bit signed integer.
    I128(i128),
    /// A signed integer of platform width.
    ISize(isize),
}

impl Num {
    /// Converts `self` into a `u128`.
    pub fn to_u128_strict(self) -> Result<u128, i128> {
        match self {
            Num::U8(v) => Ok(v as u128),
            Num::U16(v) => Ok(v as u128),
            Num::U32(v) => Ok(v as u128),
            Num::U64(v) => Ok(v as u128),
            Num::U128(v) => Ok(v as u128),
            Num::USize(v) => Ok(v as u128),
            Num::I8(v) => Err(v as i128),
            Num::I16(v) => Err(v as i128),
            Num::I32(v) => Err(v as i128),
            Num::I64(v) => Err(v as i128),
            Num::I128(v) => Err(v as i128),
            Num::ISize(v) => Err(v as i128),
        }
    }

    /// Converts `self` into a `u128`.
    pub fn to_u128_lossy(self) -> Result<u128, i128> {
        Ok(match self {
            Num::U8(v) => v as u128,
            Num::U16(v) => v as u128,
            Num::U32(v) => v as u128,
            Num::U64(v) => v as u128,
            Num::U128(v) => v as u128,
            Num::USize(v) => v as u128,
            Num::I8(v) if v >= 0 => v as u128,
            Num::I16(v) if v >= 0 => v as u128,
            Num::I32(v) if v >= 0 => v as u128,
            Num::I64(v) if v >= 0 => v as u128,
            Num::I128(v) if v >= 0 => v as u128,
            Num::ISize(v) if v >= 0 => v as u128,
            Num::I8(v) => return Err(v as i128),
            Num::I16(v) => return Err(v as i128),
            Num::I32(v) => return Err(v as i128),
            Num::I64(v) => return Err(v as i128),
            Num::I128(v) => return Err(v as i128),
            Num::ISize(v) => return Err(v as i128),
        })
    }
}

impl PartialEq for Num {
    fn eq(&self, other: &Self) -> bool {
        match (self.to_u128_lossy(), other.to_u128_lossy()) {
            (Ok(a), Ok(b)) => a == b,
            (Err(a), Err(b)) => a == b,
            (Ok(_), Err(_)) | (Err(_), Ok(_)) => false,
        }
    }
}

impl Eq for Num { }

impl std::hash::Hash for Num {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self.to_u128_lossy() {
            Ok(v) => v.hash(state),
            Err(v) => v.hash(state),
        }
    }
}

impl PartialOrd for Num {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Num {
    /// ```rust
    /// use harper::value::Num;
    ///
    /// assert!(Num::from(-1i8) < Num::from(0u8));
    /// assert!(Num::from(-0i8) == Num::from(0u8));
    /// assert!(Num::from(-20i32) == Num::from(-20i32));
    /// assert!(Num::from(10i32) == Num::from(10u64));
    /// assert!(Num::from(-2i8) > Num::from(-3i8));
    /// assert!(Num::from(1i8) > Num::from(0u8));
    /// assert!(Num::from(5u32) > Num::from(-1i64));
    /// ```
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (self.to_u128_lossy(), other.to_u128_lossy()) {
            (Ok(a), Ok(b)) => a.cmp(&b),
            (Ok(_), Err(_)) => std::cmp::Ordering::Greater,
            (Err(_), Ok(_)) => std::cmp::Ordering::Less,
            (Err(a), Err(b)) => a.cmp(&b),
        }
    }
}

macro_rules! impl_from_for_num_value {
    ($($T:ty: $V:ident),* $(,)?) => ($(
        impl From<$T> for Num {
            fn from(value: $T) -> Num {
                Num::$V(value)
            }
        }
    )*)
}

impl_from_for_num_value! {
    u8: U8, u16: U16, u32: U32, u64: U64, u128: U128, usize: USize,
    i8: I8, i16: I16, i32: I32, i64: I64, i128: I128, isize: ISize,
}

macro_rules! impl_try_from_value {
    ($($T:ty),+ => | $v:ident | $e:expr) => {
        $(
            impl TryFrom<$crate::value::Value> for $T {
                type Error = Value;

                fn try_from($v: $crate::value::Value) -> Result<Self, Self::Error> {
                    (|| $e)()
                }
            }
        )+
    };
}

impl_try_from_value!(() => |v| v.to_null().ok_or(v));
impl_try_from_value!(bool => |v| v.to_bool().ok_or(v));
impl_try_from_value!(Arc<str> => |v| v.into_str());
impl_try_from_value!(Arc<Dict> => |v| v.into_dict());
impl_try_from_value!(Arc<Path> => |v| v.into_path().map(|v| v.into()));
impl_try_from_value!(Arc<PathStr> => |v| v.into_path());
impl_try_from_value!(Arc<Url> =>  |v| {
    v.into_str().and_then(|v| Url::try_from(v).map_err(Value::from))
});

impl_try_from_value!(Num => |v| v.to_num().ok_or(v));

impl_try_from_value!(u8, u16, u32, u64, u128, usize =>
    |v| v.to_num().and_then(|v| v.to_u128_lossy().ok()?.try_into().ok()).ok_or(v));

impl_try_from_value!(i8, i16, i32, i64, i128, isize =>
    |v| v.to_num().and_then(|v| v.to_u128_lossy().ok()?.try_into().ok()).ok_or(v));

impl<T: TryFrom<Value, Error = Value>> TryFrom<Value> for Vec<T> {
    type Error = Value;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        let arc = value.into_vec()?;
        match Arc::try_unwrap(arc) {
            Ok(vec) => vec.into_iter().map(|v| v.try_into()).collect(),
            Err(arc) => arc.iter().cloned().map(|v| v.try_into()).collect()
        }
    }
}
