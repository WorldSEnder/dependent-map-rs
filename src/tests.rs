use crate::*;

struct Singleton;
#[repr(transparent)]
#[derive(Debug)]
pub struct Some<A> { pub some: A }
impl<A> HashEntry for Some<A> {
    type Key = ();
    type Value = A;
    #[inline]
    fn split_ref(&self) -> (&(), &A) {
        (&(), &self.some)
    }
    #[inline]
    fn split_mut(&mut self) -> (&(), &mut A) {
        (&(), &mut self.some)
    }
}
impl<A> From<A> for Some<A> {
    #[inline]
    fn from(a: A) -> Self {
        Some { some: a }
    }
}

impl<A> EntryFamily<A> for Singleton {
    type Result = Some<A>;
}

#[test]
fn test_some() {
    let mut map = AnyMap::<Singleton>::new();
    let _ = map.insert(42u32);
    let _ = map.insert(3.14159f32);

    assert_eq!(map.len(), 2);
    assert_eq!(map.get::<u32, _>(&()).expect("expected to contain a u32").some, 42);
    let _ = map.get::<f32, _>(&()).expect("expected to contain an f32");
    match map.get::<u64, _>(&()) { None => {}, Some(_) => assert!(false, "not expected to contain an u64") };
}
