use std::fmt;
use std::borrow::Borrow;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::hash::{BuildHasher, Hash};

use rayon::iter::IntoParallelIterator;
use rustc_hash::FxBuildHasher;

/// A map from a primary key `PK` _and_ a secondary key `SK` to a value `V`.
///
/// Primary keys, secondary keys, and values are in one-to-one correspondence:
/// every value is reachable by exactly one key of each kind. Values are stored
/// contiguously, in insertion order, making (parallel) iteration cheap.
pub struct DuoIndexMap<PK, SK, V, S = FxBuildHasher> {
    values: Vec<V>,
    primary: HashMap<PK, usize, S>,
    secondary: HashMap<SK, usize, S>,
}

impl<PK, SK, V, S> DuoIndexMap<PK, SK, V, S>
    where PK: Eq + Hash, SK: Eq + Hash, S: BuildHasher
{
    /// Returns the value for `pk`. If there is none, first inserts `v(&pk,
    /// &sk)` under `pk` and `sk = sk()`. `sk` and `v` are only called then.
    ///
    /// Returns `Err(sk)`, leaving `self` unchanged, if `pk` is absent but
    /// `sk()` is present.
    pub fn get_or_insert_with<FSK, FV>(&mut self, pk: PK, sk: FSK, v: FV) -> Result<&mut V, SK>
        where FSK: FnOnce() -> SK, FV: FnOnce(&PK, &SK) -> V, SK: Clone
    {
        let i = match self.primary.entry(pk) {
            Entry::Occupied(pentry) => *pentry.get(),
            Entry::Vacant(pentry) => match self.secondary.entry(sk()) {
                Entry::Occupied(sentry) => return Err(sentry.key().clone()),
                Entry::Vacant(sentry) => {
                    let i = self.values.len();
                    self.values.push(v(pentry.key(), sentry.key()));
                    pentry.insert(i);
                    sentry.insert(i);
                    i
                }
            }
        };

        Ok(&mut self.values[i])
    }

    /// Like [`get_or_insert_with()`](Self::get_or_insert_with()) with a value
    /// of `V::default()`.
    pub fn get_or_insert_default<FSK>(&mut self, pk: PK, sk: FSK) -> Result<&mut V, SK>
        where FSK: FnOnce() -> SK, SK: Clone, V: Default
    {
        self.get_or_insert_with(pk, sk, |_, _| V::default())
    }

    pub fn get2<Q>(&self, sk: &Q) -> Option<&V>
        where SK: Borrow<Q>, Q: Hash + Eq + ?Sized
    {
        self.secondary.get(sk).map(|&i| &self.values[i])
    }
}

impl<PK, SK, V, S> DuoIndexMap<PK, SK, V, S> {
    pub fn iter_secondary(&self) -> impl ExactSizeIterator<Item = (&SK, &V)> {
        self.secondary.iter().map(|(k, &i)| (k, &self.values[i]))
    }

    pub fn secondary_keys(&self) -> impl ExactSizeIterator<Item = &SK> {
        self.secondary.keys()
    }

    pub fn len(&self) -> usize {
        self.values.len()
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

impl<PK, SK: fmt::Debug, V: fmt::Debug, S> fmt::Debug for DuoIndexMap<PK, SK, V, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map().entries(self.iter_secondary()).finish()
    }
}

impl<'a, PK, SK, V: Sync + 'a, S> IntoParallelIterator for &'a DuoIndexMap<PK, SK, V, S> {
    type Item = &'a V;

    type Iter = rayon::slice::Iter<'a, V>;

    fn into_par_iter(self) -> Self::Iter {
        self.values.as_slice().into_par_iter()
    }
}
