use crate::*;
// use crate::{AnyMap, CloneableAnyMap, Entry};

#[derive(Clone, Debug, PartialEq)] struct A(i32);
#[derive(Clone, Debug, PartialEq)] struct B(i32);
#[derive(Clone, Debug, PartialEq)] struct C(i32);
#[derive(Clone, Debug, PartialEq)] struct D(i32);
#[derive(Clone, Debug, PartialEq)] struct E(i32);
#[derive(Clone, Debug, PartialEq)] struct F(i32);
#[derive(Clone, Debug, PartialEq)] struct J(i32);

struct Singleton;
#[repr(transparent)]
#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq)]
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
    fn from(some: A) -> Self {
        Some { some }
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
    assert_eq!(map.get_default::<u32>().expect("").some, 42);
    assert_eq!(map.get_default::<f32>().expect("").some, 3.14159f32);
    assert_eq!(map.get_default::<u64>(), None);
}

struct MultiValued;
impl<A> EntryFamily<A> for MultiValued {
    type Result = Multiple<A>;
}

#[derive(Debug, PartialOrd, Ord, PartialEq, Eq)]
pub struct Multiple<A> { pub value: A, pub variant: u32 }
impl<A> HashEntry for Multiple<A> {
    type Key = u32;
    type Value = A;
    #[inline]
    fn split_ref(&self) -> (&u32, &A) {
        (&self.variant, &self.value)
    }
    #[inline]
    fn split_mut(&mut self) -> (&u32, &mut A) {
        (&self.variant, &mut self.value)
    }
}
impl<A> From<(u32, A)> for Multiple<A> {
    #[inline]
    fn from((variant, value): (u32, A)) -> Self {
        Multiple { variant, value }
    }
}

#[test]
fn test_multiple() {
    let mut map = AnyMap::<MultiValued>::new();
    let _ = map.insert((0, 42u32));
    let _ = map.insert((1, 1337u32));

    assert_eq!(map.len(), 2);
    assert_eq!(map.get::<u32, _>(&0).expect("").value, 42);
    assert_eq!(map.get::<u32, _>(&1).expect("").value, 1337);
    assert_eq!(map.get::<u64, _>(&0), None);
}

#[test]
fn test_default() {
    let map: AnyMap<Singleton> = Default::default();
    assert_eq!(map.len(), 0);
}

#[test]
fn test_clone() {
    let mut map: CloneableAnyMap<Singleton> = Default::default();
    let _ = map.insert(A(1));
    let _ = map.insert(B(2));
    let _ = map.insert(D(3));
    let _ = map.insert(E(4));
    let _ = map.insert(F(5));
    let _ = map.insert(J(6));
    let map2 = map.clone();
    assert_eq!(map2.len(), 6);
    assert_eq!(map2.get_default::<A>().expect("").some, A(1));
    assert_eq!(map2.get_default::<B>().expect("").some, B(2));
    assert_eq!(map2.get_default::<C>(), None::<&Some<C>>);
    assert_eq!(map2.get_default::<D>().expect("").some, D(3));
    assert_eq!(map2.get_default::<E>().expect("").some, E(4));
    assert_eq!(map2.get_default::<F>().expect("").some, F(5));
    assert_eq!(map2.get_default::<J>().expect("").some, J(6));
}


#[test]
fn test_varieties() {
    fn assert_send<T: Send>() { }
    fn assert_sync<T: Sync>() { }
    fn assert_clone<T: Clone>() { }

    type AnyMapSend = AnyMap<Singleton, hashbrown::hash_map::DefaultHashBuilder, dyn HashableAny<<hashbrown::hash_map::DefaultHashBuilder as std::hash::BuildHasher>::Hasher> + Send>;
    type AnyMapSync = AnyMap<Singleton, hashbrown::hash_map::DefaultHashBuilder, dyn HashableAny<<hashbrown::hash_map::DefaultHashBuilder as std::hash::BuildHasher>::Hasher> + Sync>;

    assert_send::<AnyMapSend>();
    let mut map: AnyMapSend = Default::default();
    let _ = map.insert(A(1));
    assert_sync::<AnyMapSync>();
    let mut map: AnyMapSync = Default::default();
    let _ = map.insert(A(1));
    assert_clone::<CloneableAnyMap<Singleton>>();
}
