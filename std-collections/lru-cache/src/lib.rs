#![forbid(unsafe_code)]

#![feature(map_first_last)]

use std::collections::{BTreeMap, HashMap, VecDeque};
use std::hash::Hash;

#[derive(Debug)]
pub struct LRUCache<K, V> {
    capacity: usize,
    size: usize,
    map: HashMap<K, V>,
    key_time: HashMap<K, u64>,
    btree: BTreeMap<u64, K>,
    cur_time: u64,
}

impl<K: Clone + Hash + Ord, V> LRUCache<K, V> {
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0);
        Self {
            capacity,
            size: 0,
            map: HashMap::new(),
            key_time: HashMap::new(),
            btree: BTreeMap::new(),
            cur_time: 0,
        }
    }

    pub fn get(&mut self, key: &K) -> Option<&V> {
        let val = self.map.get(key);
        if val.is_some() {
            let time = self.key_time.remove(key).unwrap();
            self.btree.remove(&time);
            self.cur_time += 1;
            self.key_time.insert(key.clone(), self.cur_time);
            self.btree.insert(self.cur_time, key.clone());
        }
        val
    }

    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        let res = self.map.remove(&key);
        if res.is_some() {
            let time = self.key_time.remove(&key).unwrap();
            self.btree.remove(&time);
            self.cur_time += 1;
            self.key_time.insert(key.clone(), self.cur_time);
            self.btree.insert(self.cur_time, key.clone());
        } else {
            if self.size == self.capacity {
                if let Some((_, ref key)) = self.btree.pop_first() {
                    let time = self.key_time.remove(key).unwrap();
                    self.btree.remove(&time);
                    self.map.remove(key);
                }
            } else {
                self.size += 1;
            }
            self.cur_time += 1;
            self.key_time.insert(key.clone(), self.cur_time);
            self.btree.insert(self.cur_time, key.clone());
        }
        self.map.insert(key, value);
        res
    }
}
