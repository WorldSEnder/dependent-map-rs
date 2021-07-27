//! This crate provides the [`AnyMap`] type, a safe and convenient store for one value of each type.
#![cfg_attr(feature = "unstable_features", feature(unsize, coerce_unsized))]
#![warn(missing_docs, unused_results)]

mod map;
pub use map::*;
#[cfg(test)]
mod tests;

/*
use std::any::TypeId;
use std::marker::PhantomData;

use raw::RawMap;
use any::{IntoBox, Any, lateral_ref_unchecked, lateral_mut_unchecked, lateral_boxcast_unchecked};

pub mod any;
pub mod raw;

/// A collection containing zero or one values for any given type and allowing convenient,
/// type-safe access to those values.
///
/// The type parameter `A` allows you to use a different value type; normally you will want it to
/// be `anymap::any::Any`, but there are other choices:
///
/// - If you want the entire map to be cloneable, use `CloneAny` instead of `Any`.
/// - You can add on `+ Send` and/or `+ Sync` (e.g. `Map<Any + Send>`) to add those bounds.
///
/// ```rust
/// # use anymap::AnyMap;
/// let mut data = AnyMap::new();
/// assert_eq!(data.get(), None::<&i32>);
/// data.insert(42i32);
/// assert_eq!(data.get(), Some(&42i32));
/// data.remove::<i32>();
/// assert_eq!(data.get::<i32>(), None);
///
/// #[derive(Clone, PartialEq, Debug)]
/// struct Foo {
///     str: String,
/// }
///
/// assert_eq!(data.get::<Foo>(), None);
/// data.insert(Foo { str: format!("foo") });
/// assert_eq!(data.get(), Some(&Foo { str: format!("foo") }));
/// data.get_mut::<Foo>().map(|foo| foo.str.push('t'));
/// assert_eq!(&*data.get::<Foo>().unwrap().str, "foot");
/// ```
///
/// Values containing non-static references are not permitted.
#[derive(Debug)]
pub struct Map<A: ?Sized + Any = dyn Any> {
    raw: RawMap<A>,
}


#[cfg(test)]
mod tests {
    use crate::{Map, AnyMap, Entry};
    use crate::any::{Any, CloneAny};

    macro_rules! test_entry {
        ($name:ident, $init:ty) => {
            #[test]
            fn $name() {
                let mut map = <$init>::new();
                assert_eq!(map.insert(A(10)), None);
                assert_eq!(map.insert(B(20)), None);
                assert_eq!(map.insert(C(30)), None);
                assert_eq!(map.insert(D(40)), None);
                assert_eq!(map.insert(E(50)), None);
                assert_eq!(map.insert(F(60)), None);

                // Existing key (insert)
                match map.entry::<A>() {
                    Entry::Vacant(_) => unreachable!(),
                    Entry::Occupied(mut view) => {
                        assert_eq!(view.get(), &A(10));
                        assert_eq!(view.insert(A(100)), A(10));
                    }
                }
                assert_eq!(map.get::<A>().unwrap(), &A(100));
                assert_eq!(map.len(), 6);


                // Existing key (update)
                match map.entry::<B>() {
                    Entry::Vacant(_) => unreachable!(),
                    Entry::Occupied(mut view) => {
                        let v = view.get_mut();
                        let new_v = B(v.0 * 10);
                        *v = new_v;
                    }
                }
                assert_eq!(map.get::<B>().unwrap(), &B(200));
                assert_eq!(map.len(), 6);


                // Existing key (remove)
                match map.entry::<C>() {
                    Entry::Vacant(_) => unreachable!(),
                    Entry::Occupied(view) => {
                        assert_eq!(view.remove(), C(30));
                    }
                }
                assert_eq!(map.get::<C>(), None);
                assert_eq!(map.len(), 5);


                // Inexistent key (insert)
                match map.entry::<J>() {
                    Entry::Occupied(_) => unreachable!(),
                    Entry::Vacant(view) => {
                        assert_eq!(*view.insert(J(1000)), J(1000));
                    }
                }
                assert_eq!(map.get::<J>().unwrap(), &J(1000));
                assert_eq!(map.len(), 6);

                // Entry.or_insert on existing key
                map.entry::<B>().or_insert(B(71)).0 += 1;
                assert_eq!(map.get::<B>().unwrap(), &B(201));
                assert_eq!(map.len(), 6);

                // Entry.or_insert on nonexisting key
                map.entry::<C>().or_insert(C(300)).0 += 1;
                assert_eq!(map.get::<C>().unwrap(), &C(301));
                assert_eq!(map.len(), 7);
            }
        }
    }

    test_entry!(test_entry_any, AnyMap);
    test_entry!(test_entry_cloneany, Map<dyn CloneAny>);

    #[test]
    fn test_varieties() {

        assert_send::<Map<dyn Any + Send>>();
        assert_send::<Map<dyn Any + Send + Sync>>();
        assert_send::<Map<dyn CloneAny + Send>>();
        assert_send::<Map<dyn CloneAny + Send + Sync>>();

        assert_sync::<Map<dyn Any + Sync>>();
        assert_sync::<Map<dyn Any + Send + Sync>>();
        assert_sync::<Map<dyn CloneAny + Sync>>();
        assert_sync::<Map<dyn CloneAny + Send + Sync>>();

        assert_clone::<Map<dyn CloneAny + Send>>();
        assert_clone::<Map<dyn CloneAny + Send + Sync>>();
        assert_clone::<Map<dyn CloneAny + Sync>>();
        assert_clone::<Map<dyn CloneAny + Send + Sync>>();

        assert_debug::<Map<dyn Any>>();
        assert_debug::<Map<dyn Any + Send>>();
        assert_debug::<Map<dyn Any + Sync>>();
        assert_debug::<Map<dyn Any + Send + Sync>>();

        assert_debug::<Map<dyn CloneAny>>();
        assert_debug::<Map<dyn CloneAny + Send>>();
        assert_debug::<Map<dyn CloneAny + Sync>>();
        assert_debug::<Map<dyn CloneAny + Send + Sync>>();
    }
}
*/
