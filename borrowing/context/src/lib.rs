#![forbid(unsafe_code)]

use std::any::{Any, TypeId};
use std::collections::HashMap;

pub struct Context {
    container: HashMap<String, Box<dyn Any>>,
    singletons: HashMap<TypeId, Box<dyn Any>>,
}

impl Context {
    pub fn new() -> Self {
        Self {
            container: HashMap::new(),
            singletons: HashMap::new(),
        }
    }

    pub fn insert<T: 'static>(&mut self, key: impl Into<String>, obj: T) {
        self.container.insert(key.into(), Box::new(obj));
    }

    pub fn get<T: 'static>(&self, key: impl AsRef<str>) -> &T {
        self.container
            .get(key.as_ref())
            .and_then(|a| a.downcast_ref::<T>())
            .unwrap()
    }

    pub fn insert_singleton<T: 'static>(&mut self, obj: T) {
        self.singletons.insert(TypeId::of::<T>(), Box::new(obj));
    }

    pub fn get_singleton<T: 'static>(&self) -> &T {
        self.singletons
            .get(&TypeId::of::<T>())
            .and_then(|a| a.downcast_ref::<T>())
            .unwrap()
    }
}

#[cfg(test)]
mod test {
    use super::Context;
    ////////////////////////////////////////////////////////////////////////////////

    trait SayHi {
        fn say_hi(&self) -> &str;
    }

    struct Greeter {}

    impl SayHi for Greeter {
        fn say_hi(&self) -> &str {
            "hi!"
        }
    }

    ////////////////////////////////////////////////////////////////////////////////

    #[test]
    fn singletones() {
        let mut cx = Context::new();

        cx.insert_singleton(64i64);
        cx.insert_singleton(32i32);
        assert_eq!(*cx.get_singleton::<i64>(), 64);
        assert_eq!(*cx.get_singleton::<i32>(), 32);

        cx.insert_singleton(Box::new(Greeter {}) as Box<dyn SayHi>);
        assert_eq!(cx.get_singleton::<Box<dyn SayHi>>().say_hi(), "hi!");

        cx.insert_singleton::<Box<[u8]>>(Box::new(*b"binary data"));
        assert_eq!(
            cx.get_singleton::<Box<[u8]>>() as &[u8],
            b"binary data" as &[u8]
        );

        cx.insert_singleton("hello, world!");
        assert_eq!(*cx.get_singleton::<&'static str>(), "hello, world!");
        cx.insert_singleton("foo bar");
        assert_eq!(*cx.get_singleton::<&'static str>(), "foo bar");
    }

    #[test]
    fn key() {
        let mut cx = Context::new();

        cx.insert("x", 128i32);
        cx.insert("y", 255i32);
        assert_eq!(*cx.get::<i32>("x"), 128);
        assert_eq!(*cx.get::<i32>("y"), 255);

        cx.insert_singleton(372i32);
        assert_eq!(*cx.get_singleton::<i32>(), 372);

        cx.insert("z", 100i32);
        assert_eq!(*cx.get::<i32>("z"), 100);
        assert_eq!(*cx.get::<i32>("x"), 128);
        assert_eq!(*cx.get::<i32>("y"), 255);

        cx.insert("my_str", "my favourite str");
        assert_eq!(*cx.get::<&'static str>("my_str"), "my favourite str");

        assert_eq!(*cx.get_singleton::<i32>(), 372);

        let key = "foo".to_string();
        cx.insert(key.clone(), true);
        assert_eq!(*cx.get::<bool>(&key), true);
    }

    #[test]
    #[should_panic]
    fn get_missing() {
        let cx = Context::new();
        cx.get::<Greeter>("greeter");
    }

    #[test]
    #[should_panic]
    fn get_missing_singleton() {
        let cx = Context::new();
        cx.get_singleton::<Greeter>();
    }

    #[test]
    #[should_panic]
    fn wrong_type() {
        let mut cx = Context::new();
        cx.insert("greeter", Greeter {});
        cx.get::<usize>("greeter");
    }
}
