use std::fmt::Debug;
use crate::*;
use crate::families::*;
use crate::variants::*;

#[derive(Clone, Debug, PartialEq)] struct A(i32);
#[derive(Clone, Debug, PartialEq)] struct B(i32);
#[derive(Clone, Debug, PartialEq)] struct C(i32);
#[derive(Clone, Debug, PartialEq)] struct D(i32);
#[derive(Clone, Debug, PartialEq)] struct E(i32);
#[derive(Clone, Debug, PartialEq)] struct F(i32);
#[derive(Clone, Debug, PartialEq)] struct J(i32);

#[test]
fn test_some() {
    let mut map = Map::<Singleton>::new();
    let _ = map.insert(42u32);
    let _ = map.insert(3.14159f32);

    assert_eq!(map.len(), 2);
    assert_eq!(**map.get_default::<u32>().expect(""), 42);
    assert_eq!(**map.get_default::<f32>().expect(""), 3.14159f32);
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
    let mut map = Map::<MultiValued>::new();
    let _ = map.insert((0, 42u32));
    let _ = map.insert((1, 1337u32));

    assert_eq!(map.len(), 2);
    assert_eq!(map.get::<u32, _>(&0).expect("").value, 42);
    assert_eq!(map.get::<u32, _>(&1).expect("").value, 1337);
    assert_eq!(map.get::<u64, _>(&0), None);
}

#[test]
fn test_default() {
    let map: Map<Singleton> = Default::default();
    assert_eq!(map.len(), 0);
}

#[test]
fn test_clone() {
    let mut map: CloneableMap<Singleton> = Default::default();
    let _ = map.insert(A(1));
    let _ = map.insert(B(2));
    let _ = map.insert(D(3));
    let _ = map.insert(E(4));
    let _ = map.insert(F(5));
    let _ = map.insert(J(6));
    let map2 = map.clone();
    assert_eq!(map2.len(), 6);
    assert_eq!(**map2.get_default::<A>().expect(""), A(1));
    assert_eq!(**map2.get_default::<B>().expect(""), B(2));
    assert_eq!(map2.get_default::<C>(), None::<&Some<C>>);
    assert_eq!(**map2.get_default::<D>().expect(""), D(3));
    assert_eq!(**map2.get_default::<E>().expect(""), E(4));
    assert_eq!(**map2.get_default::<F>().expect(""), F(5));
    assert_eq!(**map2.get_default::<J>().expect(""), J(6));
}
#[test]
fn test_compare() {
    let mut map: ComparableMap<Singleton> = Default::default();
    let mut map2: ComparableMap<Singleton> = Default::default();
    let _ = map.insert(A(1));
    let _ = map.insert(B(2));
    let _ = map.insert(D(3));
    let _ = map2.insert(A(10));
    let _ = map2.insert(B(2));
    let _ = map2.insert(D(3));
    // NOT using assert_eq since no Debug impl here
    assert!(map == map); // test reflexivity
    assert!(map != map2); // test inequality
}

#[test]
fn test_varieties() {
    fn assert_send<T: Send>() { }
    fn assert_sync<T: Sync>() { }
    fn assert_clone<T: Clone>() { }
    fn assert_debug<T: Debug>() { }

    type MapSend = Map<Singleton, DefaultHashBuilder, dyn HashableAny<DefaultHasher> + Send>;
    type MapSync = Map<Singleton, DefaultHashBuilder, dyn HashableAny<DefaultHasher> + Sync>;

    assert_send::<MapSend>();
    #[cfg(feature = "unstable_features")]
    {
        let mut map: MapSend = Default::default();
        let _ = map.insert(A(1));
    }
    assert_sync::<MapSync>();
    #[cfg(feature = "unstable_features")]
    {
        let mut map: MapSync = Default::default();
        let _ = map.insert(A(1));
    }
    assert_clone::<CloneableMap<Singleton>>();
    {
        let mut map: DebuggableMap<MultiValued> = Default::default();
        let _ = map.insert((0, A(42)));
        let _ = map.insert((1, B(1337)));
        println!("{:?}", map);
    }
    assert_debug::<DebuggableMap<Singleton>>();
}
