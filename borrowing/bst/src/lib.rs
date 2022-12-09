#![forbid(unsafe_code)]

use std::cmp::Ordering;

struct Node {
    key: i64,
    left_ptr: Option<Box<Node>>,
    right_ptr: Option<Box<Node>>,
}

#[derive(Default)]
pub struct BstSet {
    root: Option<Box<Node>>,
    size: usize,
}

impl BstSet {
    pub fn new() -> Self {
        Self { root: None, size: 0 }
    }

    pub fn len(&self) -> usize {
        self.size
    }

    pub fn is_empty(&self) -> bool {
        self.root.is_none()
    }

    pub fn contains(&self, key: i64) -> bool {
        let mut cur = &self.root;
        while let Some(node) = cur {
            match node.key.cmp(&key) {
                Ordering::Less => cur = &node.right_ptr,
                Ordering::Equal => return true,
                Ordering::Greater => cur = &node.left_ptr,
            }
        }
        false
    }

    pub fn insert(&mut self, key: i64) -> bool {
        let mut cur = &mut self.root;
        while let Some(ref mut node) = cur {
            match node.key.cmp(&key) {
                Ordering::Less => cur = &mut node.right_ptr,
                Ordering::Equal => return false,
                Ordering::Greater => cur = &mut node.left_ptr,
            }
        }
        *cur = Some(Box::new(Node {
            key,
            left_ptr: None,
            right_ptr: None,
        }));
        self.size += 1;
        true
    }

    pub fn remove(&mut self, key: i64) -> bool {
        let mut cur = &mut self.root;
        while let Some(ref mut node) = cur {
            match node.key.cmp(&key) {
                Ordering::Less => cur = &mut cur.as_mut().unwrap().right_ptr,
                Ordering::Equal => {
                    match (node.left_ptr.as_mut(), node.right_ptr.as_mut()) {
                        (None, None) => *cur = None,
                        (Some(_), None) => *cur = node.left_ptr.take(),
                        (None, Some(_)) => *cur = node.right_ptr.take(),
                        (Some(_), Some(_)) =>
                            node.key = BstSet::delete_min(&mut node.right_ptr).unwrap().key
                    };
                    self.size -= 1;
                    return true;
                }
                Ordering::Greater => cur = &mut cur.as_mut().unwrap().left_ptr,
            }
        }
        false
    }

    fn delete_min(root: &mut Option<Box<Node>>) -> Option<Box<Node>> {
        let mut cur = root;
        let mut res = None;
        if cur.is_some() {
            while !cur.as_ref().unwrap().left_ptr.is_none() {
                cur = &mut cur.as_mut().unwrap().left_ptr;
            }
            res = cur.take();
            *cur = res.as_mut().and_then(|node| node.right_ptr.take());
        }
        res
    }
}
