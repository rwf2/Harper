use std::fmt;
use std::borrow::Borrow;
use std::marker::PhantomData;
use std::sync::Arc;

use crate::value::{Source, Sink};
use crate::error::Result;
use crate::value::Value;

type Hasher = std::hash::BuildHasherDefault<rustc_hash::FxHasher>;

pub trait MetaKey: 'static {
    const KEY: &'static str;

    type Value: TryFrom<Value> + Into<Value> + fmt::Debug;
}

#[macro_export]
macro_rules! define_meta_key {
    ($($v:vis $T:ident : $key:literal => $V:ty),+ $(,)?) => {
        $(
            $v struct $T;

            impl $crate::MetaKey for $T {
                const KEY: &'static str = $key;
                type Value = $V;
            }
        )+
    }
}

#[derive(Clone)]
pub struct Key<'m, 'k, V> {
    map: &'m Metadata,
    key: &'k str,
    _value: PhantomData<fn() -> V>,
}

#[derive(Debug, Clone, Default)]
pub struct Metadata {
    pub(crate) map: Arc<dashmap::DashMap<Arc<str>, Value, Hasher>>,
}

impl Metadata {
    #[inline(always)]
    pub fn get_raw(&self, key: &str) -> Option<Value> {
        self.map.get(key).map(|v| v.clone())
    }

    #[inline(always)]
    pub fn contains_key(&self, key: &str) -> bool {
        self.map.contains_key(key)
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    #[inline(always)]
    pub fn keys(&self) -> impl Iterator<Item = Arc<str>> + '_ {
        self.map.iter().map(|r| r.key().clone())
    }

    pub fn insert_raw<K, V>(&self, key: K, value: V) -> Option<Value>
        where K: Into<Arc<str>> + Borrow<str>, V: Into<Value>
    {
        let mut value = value.into();
        if let Some(mut existing) = self.map.get_mut(key.borrow()) {
            std::mem::swap(&mut *existing, &mut value);
            Some(value)
        } else {
            self.map.insert(key.into(), value)
        }
    }
}

impl Metadata {
    #[inline(always)]
    pub fn new() -> Self {
        Metadata::default()
    }

    #[inline]
    pub fn get<K: MetaKey>(&self, _: K) -> Option<Result<K::Value, Value>> {
        let value = self.get_raw(K::KEY)?;
        Some(value.clone().try_into().map_err(|_| value))
    }

    #[inline(always)]
    pub fn contains<K: MetaKey>(&self, _: K) -> bool {
        self.contains_key(K::KEY)
    }

    #[inline(always)]
    pub fn key<'k>(&self, key: &'k str) -> Key<'_, 'k, Value> {
        Key { map: self, key, _value: PhantomData }
    }

    #[inline(always)]
    pub fn metakey<K: MetaKey>(&self, _: K) -> Key<'_, 'static, K::Value> {
        Key { map: self, key: K::KEY, _value: PhantomData }
    }

    /// Insert if no value for key exists.
    pub fn get_or_insert<K, V>(&self, _: K, value: V) -> Result<K::Value, Value>
        where K: MetaKey, V: Into<K::Value>
    {
        let value = self.get_or_insert_raw(K::KEY, value.into().into());
        value.clone().try_into().map_err(|_| value)
    }

    pub fn get_or_insert_with<K, V, F>(&self, _: K, f: F) -> Result<K::Value, Value>
        where K: MetaKey, V: Into<K::Value>, F: FnOnce() -> V
    {
        let value = self.get_or_insert_raw_with(K::KEY, || f().into().into());
        value.clone().try_into().map_err(|_| value)
    }

    /// Insert if no value for key exists.
    pub fn get_or_insert_raw<K, V>(&self, key: K, value: V) -> Value
        where K: Into<Arc<str>> + Borrow<str>, V: Into<Value>
    {
        self.get_or_insert_raw_with(key.into(), || value.into())
    }

    /// Insert if no value for key exists.
    pub fn get_or_insert_raw_with<K, V, F>(&self, key: K, f: F) -> Value
        where K: Into<Arc<str>> + Borrow<str>, V: Into<Value>, F: FnOnce() -> V,
    {
        if let Some(value) = self.get_raw(key.borrow()) {
            return value;
        }

        let value = f().into();
        self.insert_raw(key.into(), value.clone());
        value
    }

    pub fn insert<K, V>(&self, _: K, value: V) -> Option<Value>
        where K: MetaKey, V: Into<K::Value>
    {
        self.insert_raw(K::KEY, value.into().into())
    }

    pub fn remove<K: MetaKey>(&self, _: K) -> Option<Value> {
        self.map.remove(K::KEY).map(|(_, v)| v)
    }

    pub fn remove_raw<K: Borrow<str>>(&self, key: K) -> Option<Value> {
        self.map.remove(key.borrow()).map(|(_, v)| v)
    }

    #[inline(always)]
    pub fn append_all(&self, dict: &crate::value::Dict) {
        for (k, v) in dict {
            self.insert_raw(k.clone(), v.clone());
        }
    }

    // #[inline(always)]
    // pub fn extend<I, K, V>(&self, iter: I)
    //     where I: IntoIterator<Item = (K, V)>,
    //           K: Into<Arc<str>>,
    //           V: Into<Value>
    // {
    //     for (k, v) in iter.into_iter() {
    //         self.insert_raw(k.into(), v.into());
    //     }
    // }
}

impl<V> fmt::Debug for Key<'_, '_, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Key")
            .field("map", &self.map)
            .field("key", &self.key)
            .finish()
    }
}

impl fmt::Display for Metadata {
    #[inline(always)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:#?}", self.map)
    }
}

// TODO: This is very slow for things like `V: Vec<Arc<str>>` since it actually
// does the conversion. We just want to check the type.
impl<V: TryFrom<Value> + Into<Value> + 'static> Source for Key<'_, '_, V> {
    type Value = V;

    fn read(self) -> Result<Self::Value> {
        let value = self.map.get_raw(self.key)
            .ok_or_else(|| error! {
                "attempted to read nonexistent metadata key",
                "key" => self.key,
            })?;

        let kind = value.kind();
        V::try_from(value)
            .map_err(|_| error! {
                "unexpected metadata value type",
                "key" => self.key,
                "expected" => std::any::type_name::<V>(),
                "actual type" => kind,
            })
    }
}

// TODO: This is very slow for things like `V: Vec<Arc<str>>` since it actually
// does the conversion. We just want to check the type.
impl<V: TryFrom<Value> + Into<Value> + 'static> Sink for Key<'_, '_, V> {
    fn write<T: Into<Value> + 'static>(&self, value: T) -> Result<()> {
        use std::any::TypeId;

        if TypeId::of::<V>() != TypeId::of::<Value>()
            && TypeId::of::<T>() != TypeId::of::<V>() {
            if !(TypeId::of::<T>() == TypeId::of::<Arc<str>>()
                && TypeId::of::<V>() == TypeId::of::<String>())
            && !(TypeId::of::<T>() == TypeId::of::<String>()
                && TypeId::of::<V>() == TypeId::of::<Arc<str>>())
            {
                return err! {
                    "unexpected value type for metadata",
                    "key" => self.key,
                    "expected" => std::any::type_name::<V>(),
                    "actual type" => std::any::type_name::<T>(),
                }
            }
        }

        self.write_value(value.into())
    }

    fn write_value(&self, value: Value) -> Result<()> {
        self.map.insert_raw(self.key, value);
        Ok(())
    }
}

impl Sink for Metadata {
    fn write_value(&self, value: Value) -> Result<()> {
        if let Value::Dict(dict) = value {
            self.append_all(&dict);
            return Ok(());
        }

        err!(format!("expected value to be an object, found {}", value.kind()))
    }
}
