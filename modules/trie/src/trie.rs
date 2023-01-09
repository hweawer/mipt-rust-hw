#![forbid(unsafe_code)]
use crate::trie_key::ToKeyIter;
use std::borrow::Cow::Owned;
use std::{borrow::Borrow, collections::HashMap, hash::Hash, ops::Index};

struct TrieNode<K: ToKeyIter, V> {
    value: Option<V>,
    counter: u64,
    children: HashMap<K::Item, TrieNode<K, V>>,
}

impl<K: ToKeyIter, V> TrieNode<K, V> {
    fn new() -> Self {
        TrieNode {
            value: None,
            counter: 0,
            children: HashMap::new(),
        }
    }
}

////////////////////////////////////////////////////////////////////////////////

pub struct Trie<K: ToKeyIter, V> {
    size: usize,
    root: TrieNode<K, V>,
}

impl<K, V> Trie<K, V>
where
    K: ToKeyIter,
    K::Item: Eq + Hash,
{
    pub fn new() -> Self {
        Self {
            size: 0,
            root: TrieNode::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.size
    }

    pub fn is_empty(&self) -> bool {
        self.root.children.is_empty()
    }

    pub fn insert<Q: ?Sized + ToKeyIter<Item = K::Item>>(
        &mut self,
        key: &Q,
        value: V,
    ) -> Option<V> {
        let contains = self.contains(key);
        //todo: optimize it
        let mut current = &mut self.root;
        for symbol in key.to_owned().key_iter() {
            if !current.children.contains_key(&symbol) {
                current.children.insert(symbol.clone(), TrieNode::new());
            }
            current = current.children.get_mut(&symbol).unwrap();
            if !contains {
                current.counter += 1;
            }
        }
        let res = current.value.take();
        if res.is_none() {
            self.size += 1;
        }
        current.value = Some(value);
        return res;
    }

    pub fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        Q: ?Sized + ToKeyIter<Item = K::Item>,
    {
        let mut current = &self.root;
        for symbol in key.key_iter() {
            if !current.children.contains_key(&symbol) {
                return None;
            }
            current = current.children.get(&symbol).unwrap();
        }
        current.value.as_ref()
    }

    pub fn get_mut<Q: ?Sized + ToKeyIter<Item = K::Item>>(&mut self, key: &Q) -> Option<&mut V> {
        let mut current = &mut self.root;
        for symbol in key.to_owned().key_iter() {
            if !current.children.contains_key(&symbol) {
                return None;
            }
            current = current.children.get_mut(&symbol).unwrap();
        }
        current.value.as_mut()
    }

    pub fn contains<Q: ?Sized + ToKeyIter<Item = K::Item>>(&self, key: &Q) -> bool {
        self.get(key).is_some()
    }

    pub fn starts_with<Q: ?Sized + ToKeyIter<Item = K::Item>>(&self, key: &Q) -> bool {
        let mut current = &self.root;
        for symbol in key.key_iter() {
            if !current.children.contains_key(&symbol) {
                return false;
            }
            current = current.children.get(&symbol).unwrap();
        }
        true
    }

    pub fn remove<Q: ?Sized + ToKeyIter<Item = K::Item>>(&mut self, key: &Q) -> Option<V> {
        if self.contains(key) {
            let mut current = &mut self.root;
            let mut moved;
            for symbol in key.to_owned().key_iter() {
                if !current.children.contains_key(&symbol) {
                    return None;
                }
                let mut temp = current.children.get_mut(&symbol).unwrap();
                temp.counter -= 1;
                current = if temp.counter == 0 {
                    moved = current.children.remove(&symbol).unwrap();
                    &mut moved
                } else {
                    current.children.get_mut(&symbol).unwrap()
                }
            }
            self.size -= 1;
            current.value.take()
        } else {
            None
        }
    }
}

////////////////////////////////////////////////////////////////////////////////

impl<K, V, Q> Index<Q> for Trie<K, V>
where
    K: ToKeyIter,
    K::Item: Eq + Hash,
    Q: Into<K>,
{
    type Output = V;

    fn index(&self, index: Q) -> &Self::Output {
        self.get(&index.into()).unwrap()
    }
}
