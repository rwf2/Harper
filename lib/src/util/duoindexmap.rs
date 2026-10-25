use core::fmt;
use std::borrow::Borrow;
use std::hash::{BuildHasher, BuildHasherDefault, Hash};
use std::collections::HashMap;
use std::mem::replace;

use rayon::iter::IntoParallelIterator;
use rustc_hash::FxHasher;

pub struct DuoIndexMap<PK, SK, V, S = BuildHasherDefault<FxHasher>> {
    values: Vec<V>,
    primary: HashMap<PK, usize, S>,
    secondary: HashMap<SK, usize, S>,
}

// insert(K1, KA, v1)
//   K1 -> 0
//   KA -> 0
//   [V1]
//
// insert(K2, KA, v2)
//   K1 -> 0
//   K2 -> 1
//   KA -> 1
//   [V1, V2]
//
// insert(K2, KB, v3)
//   K1 -> 0
//   K2 -> 1
//   KA -> 1
//   KB -> 1
//   [V1, V3]

impl<PK, SK, V, S> DuoIndexMap<PK, SK, V, S>
    where PK: Eq + Hash, SK: Eq + Hash, S: BuildHasher
{
    fn _insert(&mut self, pk: PK, sk: SK, v: V) -> Result<(usize, Option<V>), ()> {
        let (i, old) = match (self.primary.get(&pk), self.secondary.get(&sk)) {
            (None, None) => {
                let i = self.values.len();
                self.values.push(v);
                (i, None)
            },
            (Some(&i), Some(&j)) if i == j => {
                (i, Some(replace(&mut self.values[i], v)))
            }
            _ => return Err(()),
        };

        self.primary.insert(pk, i);
        self.secondary.insert(sk, i);
        Ok((i, old))
    }

    pub fn insert(&mut self, pk: PK, sk: SK, v: V) -> Result<Option<V>, ()> {
        self._insert(pk, sk, v).map(|(_, v)| v)
    }

    pub fn get_or_insert_with<FSK, FV>(&mut self, pk: PK, sk: FSK, v: FV) -> Result<&mut V, ()>
        where FSK: FnOnce() -> SK, FV: FnOnce(&PK, &SK) -> V,
    {
        if let Some(i) = self.primary.get(&pk).copied() {
            return Ok(&mut self.values[i]);
        }

        let sk = sk();
        let v = v(&pk, &sk);
        let i = self._insert(pk, sk, v)?.0;
        Ok(&mut self.values[i])
    }

    pub fn get_or_insert_default<FSK>(&mut self, pk: PK, sk: FSK) -> Result<&mut V, ()>
        where FSK: FnOnce() -> SK, V: Default
    {
        if let Some(i) = self.primary.get(&pk).copied() {
            return Ok(&mut self.values[i]);
        }

        let sk = sk();
        let i = self._insert(pk, sk, V::default())?.0;
        Ok(&mut self.values[i])
    }

    pub fn get<Q>(&self, pk: &Q) -> Option<&V>
        where PK: Borrow<Q>, Q: Hash + Eq + ?Sized
    {
        self.primary.get(pk).map(|&i| &self.values[i])
    }

    pub fn get_mut<Q>(&mut self, pk: &Q) -> Option<&mut V>
        where PK: Borrow<Q>, Q: Hash + Eq + ?Sized
    {
        self.primary.get(pk).map(|&i| &mut self.values[i])
    }

    pub fn get2<Q>(&self, sk: &Q) -> Option<&V>
        where SK: Borrow<Q>, Q: Hash + Eq + ?Sized
    {
        self.secondary.get(sk).map(|&i| &self.values[i])
    }

    pub fn contains_pkey<Q>(&self, pk: &Q) -> bool
        where PK: Borrow<Q>,
              Q: Hash + Eq + ?Sized
    {
        self.primary.contains_key(pk)
    }

    pub fn contains_skey<Q>(&self, sk: &Q) -> bool
        where SK: Borrow<Q>,
              Q: Hash + Eq + ?Sized
    {
        self.secondary.contains_key(sk)
    }
}

impl<PK, SK, V, S: Default> Default for DuoIndexMap<PK, SK, V, S> {
    fn default() -> Self {
        Self {
            values: Default::default(),
            primary: Default::default(),
            secondary: Default::default()
        }
    }
}

impl<PK, SK, V, S> DuoIndexMap<PK, SK, V, S> {
    pub fn iter_primary(&self) -> impl ExactSizeIterator<Item = (&PK, &V)> {
        self.primary.iter().map(|(k, &i)| (k, &self.values[i]))
    }

    pub fn iter_secondary(&self) -> impl ExactSizeIterator<Item = (&SK, &V)> {
        self.secondary.iter().map(|(k, &i)| (k, &self.values[i]))
    }

    pub fn primary_keys(&self) -> impl ExactSizeIterator<Item = &PK> {
        self.primary.keys()
    }

    pub fn secondary_keys(&self) -> impl ExactSizeIterator<Item = &SK> {
        self.secondary.keys()
    }

    pub fn values(&self) -> impl ExactSizeIterator<Item = &V> {
        self.values.iter()
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }
}

impl<PK, SK, V, S> fmt::Debug for DuoIndexMap<PK, SK, V, S>
    where PK: fmt::Debug, SK: fmt::Debug, V: fmt::Debug
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map()
            .entries(self.iter_primary())
            .entries(self.iter_secondary())
            .finish()
    }
}

impl<'a, PK, SK, V: Sync + 'a, S> IntoParallelIterator for &'a DuoIndexMap<PK, SK, V, S> {
    type Item = &'a V;

    type Iter = rayon::slice::Iter<'a, V>;

    fn into_par_iter(self) -> Self::Iter {
        self.values.as_slice().into_par_iter()
    }
}
