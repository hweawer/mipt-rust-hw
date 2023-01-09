#![forbid(unsafe_code)]

use std::str::Chars;

pub trait ToKeyIter {
    type Item: Clone;
    type KeyIter<'a>: Iterator<Item = Self::Item>
    where
        Self: 'a;

    fn key_iter(&self) -> Self::KeyIter<'_>;
}

impl ToKeyIter for str {
    type Item = char;
    type KeyIter<'a> = Chars<'a>;

    fn key_iter(&self) -> Self::KeyIter<'_> {
        self.chars()
    }
}

impl ToKeyIter for String {
    type Item = char;
    type KeyIter<'a> = Chars<'a>;

    fn key_iter(&self) -> Self::KeyIter<'_> {
        self.as_str().key_iter()
    }
}
