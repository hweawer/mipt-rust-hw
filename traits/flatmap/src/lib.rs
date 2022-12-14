#![forbid(unsafe_code)]

use std::vec::IntoIter;
use std::{borrow::Borrow, iter::FromIterator, mem, ops::Index};

////////////////////////////////////////////////////////////////////////////////

#[derive(Default, Debug, PartialEq, Eq)]
pub struct FlatMap<K, V>(Vec<(K, V)>);

impl<K: Ord, V> FlatMap<K, V> {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn capacity(&self) -> usize {
        self.0.capacity()
    }

    pub fn as_slice(&self) -> &[(K, V)] {
        self.0.as_slice()
    }

    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        let search_res = &self.0.binary_search_by(|pair| pair.0.cmp(&key));
        let mut res = None;
        match search_res {
            Ok(idx) => {
                res = Some(mem::replace(&mut self.0[*idx].1, value));
            }
            Err(idx) => {
                if *idx > self.0.len() {
                    self.0.push((key, value));
                } else {
                    self.0.insert(*idx, (key, value))
                };
            }
        }
        res
    }

    pub fn get<Q: ?Sized>(&self, k: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Eq,
    {
        self.0
            .iter()
            .find(|(key, _)| key.borrow() == k)
            .map(|(_, v)| v)
    }

    pub fn remove<Q: ?Sized>(&mut self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Eq,
    {
        self.remove_entry(key).map(|(_, v)| v)
    }

    pub fn remove_entry<Q: ?Sized>(&mut self, key: &Q) -> Option<(K, V)>
    where
        K: Borrow<Q>,
        Q: Eq,
    {
        self.0
            .iter()
            .position(|(k, _)| k.borrow() == key)
            .map(|idx| self.0.remove(idx))
    }
}

////////////////////////////////////////////////////////////////////////////////

impl<K: Ord + Borrow<Q>, V, Q: ?Sized + Eq> Index<&Q> for FlatMap<K, V> {
    type Output = V;

    fn index(&self, index: &Q) -> &Self::Output {
        self.get(index).unwrap()
    }
}

impl<K: Ord, V> Extend<(K, V)> for FlatMap<K, V> {
    fn extend<T: IntoIterator<Item = (K, V)>>(&mut self, iter: T) {
        for (k, v) in iter {
            self.insert(k, v);
        }
    }
}

impl<K: Ord, V> From<Vec<(K, V)>> for FlatMap<K, V> {
    fn from(mut value: Vec<(K, V)>) -> Self {
        value.reverse();
        value.dedup_by(|p1, p2| p1.0 == p2.0);
        value.sort_by(|a, b| a.0.cmp(&b.0));
        FlatMap(value)
    }
}

impl<K, V> From<FlatMap<K, V>> for Vec<(K, V)> {
    fn from(value: FlatMap<K, V>) -> Self {
        value.0
    }
}

impl<K: Ord, V> FromIterator<(K, V)> for FlatMap<K, V> {
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
        let mut res = FlatMap::new();
        for (k, v) in iter {
            res.insert(k, v);
        }
        res
    }
}

pub struct FlatMapIntoIter<K, V> {
    into_iter: IntoIter<(K, V)>,
}

impl<K, V> FlatMapIntoIter<K, V> {
    fn new(vec: Vec<(K, V)>) -> Self {
        Self {
            into_iter: vec.into_iter(),
        }
    }
}

impl<K, V> IntoIterator for FlatMap<K, V> {
    type Item = (K, V);
    type IntoIter = FlatMapIntoIter<K, V>;

    fn into_iter(self) -> Self::IntoIter {
        FlatMapIntoIter::new(self.0)
    }
}

impl<K, V> Iterator for FlatMapIntoIter<K, V> {
    type Item = (K, V);

    fn next(&mut self) -> Option<Self::Item> {
        self.into_iter.next()
    }
}
