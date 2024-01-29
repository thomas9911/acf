use std::hash::BuildHasher;

use ahash::RandomState;
use indexmap::IndexMap;

use winnow::stream::Accumulate;

pub type Map<K, V> = IndexMapWrapper<K, V, RandomState>;
pub type Range = std::ops::Range<usize>;
pub type RangeMap<V> = Map<Range, V>;
pub type Seq<T> = Vec<T>;

#[derive(Debug)]
pub struct IndexMapWrapper<K: std::hash::Hash + std::cmp::Eq, V: std::cmp::Eq, H: BuildHasher>(
    IndexMap<K, V, H>,
);

impl<K, V, H> PartialEq for IndexMapWrapper<K, V, H>
where
    K: std::hash::Hash + std::cmp::Eq,
    V: std::cmp::Eq,
    H: BuildHasher,
{
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<K, V, H> Eq for IndexMapWrapper<K, V, H>
where
    K: std::hash::Hash + std::cmp::Eq,
    V: std::cmp::Eq,
    H: BuildHasher,
{
}

impl<K, V, H> std::ops::Deref for IndexMapWrapper<K, V, H>
where
    K: std::hash::Hash + std::cmp::Eq,
    V: std::cmp::Eq,
    H: BuildHasher,
{
    type Target = IndexMap<K, V, H>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<K, V, H> std::ops::DerefMut for IndexMapWrapper<K, V, H>
where
    K: std::hash::Hash + std::cmp::Eq,
    V: std::cmp::Eq,
    H: BuildHasher,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<K, V, H> FromIterator<(K, V)> for IndexMapWrapper<K, V, H>
where
    K: std::hash::Hash + std::cmp::Eq,
    V: std::cmp::Eq,
    H: BuildHasher + Default,
{
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iterable: I) -> Self {
        IndexMapWrapper(IndexMap::from_iter(iterable))
    }
}

impl<K, V, H> IntoIterator for IndexMapWrapper<K, V, H>
where
    K: std::hash::Hash + std::cmp::Eq,
    V: std::cmp::Eq,
    H: BuildHasher + Default,
{
    type Item = <IndexMap<K, V, H> as IntoIterator>::Item;
    type IntoIter = <IndexMap<K, V, H> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<K, V, S> Accumulate<(K, V)> for IndexMapWrapper<K, V, S>
where
    K: std::hash::Hash + std::cmp::Eq,
    V: std::cmp::Eq,
    S: BuildHasher + Default,
{
    #[inline(always)]
    fn initial(capacity: Option<usize>) -> Self {
        let h = S::default();
        match capacity {
            Some(capacity) => IndexMapWrapper(IndexMap::with_capacity_and_hasher(capacity, h)),
            None => IndexMapWrapper(IndexMap::with_hasher(h)),
        }
    }
    #[inline(always)]
    fn accumulate(&mut self, (key, value): (K, V)) {
        self.insert(key, value);
    }
}
