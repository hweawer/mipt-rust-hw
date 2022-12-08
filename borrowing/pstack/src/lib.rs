#![forbid(unsafe_code)]

use std::ops::Deref;
use std::rc::Rc;


pub struct PRef<T> {
    next: Option<Rc<PRef<T>>>,
    val: T
}

impl<T> Deref for PRef<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.val
    }
}

pub struct SimpleIter<T> {
    stack: PStack<T>
}

impl<T> Iterator for SimpleIter<T> {
    type Item = Rc<PRef<T>>;

    fn next(&mut self) -> Option<Self::Item> {
        self.stack.pop().map(|(node, stack)| {
            self.stack = stack;
            node
        })
    }
}

////////////////////////////////////////////////////////////////////////////////

pub struct PStack<T> {
    size: usize,
    head: Option<Rc<PRef<T>>>,
}

impl<T> Default for PStack<T> {
    fn default() -> Self {
        Self {
            size: 0,
            head: None,
        }
    }
}

impl<T> Clone for PStack<T> {
    fn clone(&self) -> Self {
        Self {
            size: self.size.clone(),
            head: self.head.clone(),
        }
    }
}

impl<T> PStack<T> {
    pub fn new() -> Self {
        PStack::default()
    }

    pub fn push(&self, value: T) -> Self {
        let new_cell = Rc::new(PRef {
            next: self.head.clone(),
            val: value,
        });
        Self {
            size: self.size + 1,
            head: Some(new_cell),
        }
    }

    pub fn pop(&self) -> Option<(Rc<PRef<T>>, Self)> {
        self.head.as_ref().map(|node| {
            (
                node.clone(),
                Self {
                    size: if self.is_empty() { 0 } else { self.size - 1 },
                    head: node.next.clone(),
                },
            )
        })
    }

    pub fn len(&self) -> usize {
        self.size
    }

    pub fn is_empty(&self) -> bool {
        self.head.is_none()
    }

    pub fn iter(&self) -> impl Iterator<Item = Rc<PRef<T>>> {
        SimpleIter {
            stack: self.clone(),
        }
    }
}

#[cfg(test)]
mod test {
    use super::PStack;

    #[test]
    fn simple() {
        let mut stack = PStack::new();
        assert_eq!(stack.len(), 0);
        assert!(stack.is_empty());
    
        for i in 0..10 {
            stack = stack.push(i);
            assert_eq!(stack.len(), i + 1);
        }
    
        for i in (0..10).rev() {
            let (last, stack_new) = stack.pop().unwrap();
            assert_eq!(stack_new.len(), i);
            assert_eq!(**last, i);
            stack = stack_new;
        }
    }
    
    #[test]
    fn persistence() {
        let mut stacks = vec![PStack::new()];
        for i in 0..100 {
            let st = stacks.last_mut().unwrap().push(i);
            stacks.push(st);
        }
    
        for i in (0..100).rev() {
            let (top, tail) = stacks.last().unwrap().pop().unwrap();
            assert_eq!(**top, i);
            stacks.push(tail);
        }
    
        for i in 0..100 {
            let stack = stacks[i].clone();
            assert_eq!(stack.len(), i);
    
            let mut cnt = 0;
            for (item, i) in stack.iter().zip((0..i).rev()) {
                assert_eq!(i, **item);
                cnt += 1;
            }
            assert_eq!(i, cnt);
            drop(stack);
        }
    
        for i in 100..201 {
            let stack = stacks[i].clone();
            assert_eq!(stack.len(), 200 - i);
    
            let mut cnt = 0;
            for (item, i) in stack.iter().zip((0..200 - i).rev()) {
                assert_eq!(i, **item);
                cnt += 1;
            }
            assert_eq!(200 - i, cnt);
        }
    }
    
    #[test]
    fn no_clone() {
        struct Int(i32);
    
        let mut stack = PStack::new();
        for i in 0..100 {
            stack = stack.push(Int(i));
        }
    
        for i in (0..100).rev() {
            let (top, tail) = stack.pop().unwrap();
            assert_eq!(top.0, i);
            stack = tail;
        }
    }
}
