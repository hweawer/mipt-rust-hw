#![forbid(unsafe_code)]
#![feature(fn_traits)]

use std::{
    cell::RefCell,
    collections::VecDeque,
    iter::{from_fn, repeat_with},
    rc::Rc,
};

pub fn count() -> impl Iterator<Item = u64> {
    let mut cur: u64 = 0;
    from_fn(move || {
        let res = Some(cur.clone());
        cur = cur.saturating_add(1);
        res
    })
}

struct CycleIterator<I: Iterator> {
    iter: I,
    deq: VecDeque<I::Item>,
    exhausted: bool,
}

impl<I> Iterator for CycleIterator<I>
where
    I: Iterator,
    I::Item: Clone,
{
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        if !self.exhausted {
            match self.iter.next() {
                None => {
                    self.exhausted = true;
                    let res = self.deq.pop_front();
                    if let Some(ref e) = res {
                        self.deq.push_back(e.clone());
                    }
                    res
                }
                Some(e) => {
                    self.deq.push_back(e.clone());
                    Some(e)
                }
            }
        } else {
            let res = self.deq.pop_front();
            if let Some(ref e) = res {
                self.deq.push_back(e.clone());
            }
            res
        }
    }
}

pub fn cycle<T>(into_iter: T) -> impl Iterator<Item = T::Item>
where
    T: IntoIterator,
    T::Item: Clone,
{
    CycleIterator {
        iter: into_iter.into_iter(),
        deq: VecDeque::new(),
        exhausted: false,
    }
}

pub fn extract<T: IntoIterator>(
    into_iter: T,
    index: usize,
) -> (Option<T::Item>, impl Iterator<Item = T::Item>) {
    let mut deq = VecDeque::new();
    let mut it = into_iter.into_iter();
    let mut i: usize = 0;
    while i < index {
        match it.next() {
            None => break,
            Some(e) => {
                deq.push_back(e);
            }
        }
        i += 1;
    }
    (it.next(), deq.into_iter().chain(it))
}

struct TeeBuf<I: Iterator>
where
    I::Item: Clone,
{
    deq: VecDeque<I::Item>,
    iter: I,
    owner: bool,
    exhausted: bool,
}

struct Tee<I: Iterator>
where
    I::Item: Clone,
{
    buf: Rc<RefCell<TeeBuf<I>>>,
    id: bool,
}

impl<I: Iterator> Iterator for Tee<I>
where
    I::Item: Clone,
{
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        let mut buf = self.buf.borrow_mut();
        if self.id == buf.owner {
            if let Some(e) = buf.deq.pop_front() {
                return Some(e);
            }
        }
        if !buf.exhausted {
            if let Some(e) = buf.iter.next() {
                buf.deq.push_back(e.clone());
                buf.owner = !self.id;
                return Some(e);
            } else {
                buf.exhausted = true;
            }
        }
        None
    }
}

pub fn tee<T>(into_iter: T) -> (impl Iterator<Item = T::Item>, impl Iterator<Item = T::Item>)
where
    T: IntoIterator,
    T::Item: Clone,
{
    let buffer = Rc::new(RefCell::new(TeeBuf {
        deq: VecDeque::new(),
        iter: into_iter.into_iter(),
        owner: false,
        exhausted: false,
    }));
    let t1 = Tee {
        buf: buffer.clone(),
        id: true,
    };
    let t2 = Tee {
        buf: buffer,
        id: false,
    };
    (t1, t2)
}

pub fn group_by<T, F, V>(into_iter: T, mut f: F) -> impl Iterator<Item = (V, Vec<T::Item>)>
where
    T: IntoIterator,
    F: FnMut(&T::Item) -> V,
    V: Eq,
{
    let mut deq: VecDeque<(V, Vec<T::Item>)> = VecDeque::new();
    for e in into_iter.into_iter() {
        let val = f.call_mut((&e,));
        if let Some((k, ref mut v)) = deq.back_mut() {
            if *k == val {
                v.push(e);
                continue;
            }
        }
        deq.push_back((val, vec![e]));
    }
    deq.into_iter()
}
