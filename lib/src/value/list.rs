use rayon::prelude::*;
use rayon::iter::plumbing::*;
use derive_more::Debug;

#[derive(Debug)]
#[debug("{items:?}")]
pub struct List<T> {
    ordering: parking_lot::RwLock<Option<Vec<usize>>>,
    items: boxcar::Vec<T>,
}

impl<T> List<T> {
    pub fn len(&self) -> usize {
        self.items.count()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn push(&self, item: T) {
        self.items.push(item);
    }

    pub fn get(&self, i: usize) -> Option<&T> {
        // TODO: If we kept a len of `ordering` around as an atomic usize, we
        // would only need to lock when there's actually a value.
        let i = match *self.ordering.read() {
            Some(ref ordering) => ordering.get(i).copied().unwrap_or(i),
            None => i
        };

        self.items.get(i)
    }

    #[inline]
    pub fn sort_by<F>(&self, compare: F)
        where F: Fn(&T, &T) -> std::cmp::Ordering,
    {
        let mut ordering: Vec<usize> = (0..self.items.count()).collect();
        ordering.sort_by(|a, b| compare(&*self.get(*a).unwrap(), &*self.get(*b).unwrap()));
        *self.ordering.write() = Some(ordering);
    }

    pub fn iter(&self) -> SliceIter<'_, T> {
        SliceIter {
            slice: Slice::new(self, 0, self.len()),
            next: 0,
        }
    }
}

impl<T> Default for List<T> {
    fn default() -> Self {
        Self {
            ordering: Default::default(),
            items: Default::default(),
        }
    }
}

pub struct Slice<'a, T> {
    list: &'a List<T>,
    start: usize,
    end: usize,
}

impl<'a, T> Slice<'a, T> {
    fn new(list: &'a List<T>, start: usize, end: usize) -> Self {
        debug_assert!(start <= end);
        Self { list, start, end }
    }

    fn len(&self) -> usize {
        self.end - self.start
    }

    fn get(&self, i: usize) -> Option<&'a T> {
        let i = self.start + i;
        if i < self.end && i >= self.start {
            return self.list.get(i);
        }

        None
    }

    fn split_at(self, mid: usize) -> (Self, Self) {
        assert!(mid <= self.len());

        let mid = self.start + mid;
        let left = Self { list: self.list, start: self.start, end: mid };
        let right = Self { list: self.list, start: mid, end: self.end };
        (left, right)
    }

    fn into_iter(self) -> SliceIter<'a, T> {
        SliceIter { slice: self, next: 0 }
    }
}

impl<'a, T: Send + Sync> IntoParallelIterator for &'a List<T> {
    type Iter = ParIter<'a, T>;

    type Item = &'a T;

    fn into_par_iter(self) -> Self::Iter {
        ParIter { slice: Slice::new(self, 0, self.len()) }
    }
}

pub struct SliceIter<'a, T> {
    slice: Slice<'a, T>,
    next: usize,
}

impl<'a, T> Iterator for SliceIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        let item = self.slice.get(self.next)?;
        self.next += 1;
        Some(item)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.slice.len() - self.next;
        (remaining, Some(remaining))
    }
}

impl<T> ExactSizeIterator for SliceIter<'_, T> { }

impl<T> DoubleEndedIterator for SliceIter<'_, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.next > 0 {
            self.next -= 1;
            return self.slice.get(self.next);
        }

        None
    }
}

pub struct ParIter<'a, T> {
    slice: Slice<'a, T>,
}

impl<'a, T: Sync> ParallelIterator for ParIter<'a, T> {
    type Item = &'a T;

    fn drive_unindexed<C>(self, consumer: C) -> C::Result
        where C: UnindexedConsumer<Self::Item>
    {
        bridge(self, consumer)
    }
}

impl<T: Sync> IndexedParallelIterator for ParIter<'_, T> {
    fn len(&self) -> usize {
        self.slice.len()
    }

    fn drive<C: Consumer<Self::Item>>(self, consumer: C) -> C::Result {
        bridge(self, consumer)
    }

    fn with_producer<CB: ProducerCallback<Self::Item>>(self, callback: CB) -> CB::Output {
        callback.callback(IterProducer { slice: self.slice })
    }
}

struct IterProducer<'a, T> {
    slice: Slice<'a, T>,
}

impl<'a, T: Sync> Producer for IterProducer<'a, T> {
    type Item = &'a T;

    type IntoIter = SliceIter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.slice.into_iter()
    }

    fn split_at(self, index: usize) -> (Self, Self) {
        let (left, right) = self.slice.split_at(index);
        (IterProducer { slice: left }, IterProducer { slice: right })
    }
}
