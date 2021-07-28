//!
//! The variants found in this module are not exhaustive. If you want your own variant,
//! consider using [`create_entry_impl`] and your own trait. Most methods on [`Map`] are
//! guarded behind the trait extending from [`HashableAny`].
use std::hash::{BuildHasher, Hasher};
use crate::{DebugEntry, DefaultHashBuilder, DynClone, DynPartialEq, HashableAny, Map};

/// Glue trait
/// 
/// If you get an error mentioned that this is not implemented, make sure you are using
/// a storage type that captures the Clone interface of entries, such as in [`CloneableMap`].
pub trait CloneableHashableAny<H: Hasher>: HashableAny<H> + DynClone {}
impl<H: Hasher, T> CloneableHashableAny<H> for T where T: HashableAny<H> + DynClone {}
/// Glue trait
/// 
/// If you get an error mentioned that this is not implemented, make sure you are using
/// a storage type that captures the PartialEq interface of entries, such as in [`ComparableMap`].
pub trait PartialEqHashableAny<H: Hasher>: HashableAny<H> + DynPartialEq {}
impl<H: Hasher, T> PartialEqHashableAny<H> for T where T: HashableAny<H> + DynPartialEq {}
/// Glue trait
/// 
/// If you get an error mentioned that this is not implemented, make sure you are using
/// a storage type that captures the Debug interface of entries, such as in [`DebuggableMap`].
pub trait DebugHashableAny<H: Hasher>: HashableAny<H> + DebugEntry {}
impl<H: Hasher, T> DebugHashableAny<H> for T where T: HashableAny<H> + DebugEntry {}

type CloneDynStorage<S> = dyn CloneableHashableAny<<S as BuildHasher>::Hasher>;
type PartialEqDynStorage<S> = dyn PartialEqHashableAny<<S as BuildHasher>::Hasher>;
type DebugDynStorage<S> = dyn DebugHashableAny<<S as BuildHasher>::Hasher>;

/// Type-alias for a [`Map`] that can be cloned.
/// 
/// Note that this works because the trait object captures [`DynClone`]. If you want to combine
/// multiple capabilites, such as `Clone + Debug`, write a combined trait and use that as the
/// third argument to [`Map`].
pub type CloneableMap<E, S = DefaultHashBuilder> = Map<E, S, CloneDynStorage<S>>;
/// Type-alias for a [`Map`] that can be equality compared. [`PartialEq`]-only version!
/// 
/// Note that this works because the trait object captures [`DynPartialEq`]. If you want to combine
/// multiple capabilites, such as `PartialEq + Debug`, write a combined trait and use that as the
/// third argument to [`Map`].
pub type ComparableMap<E, S = DefaultHashBuilder> = Map<E, S, PartialEqDynStorage<S>>;
/// Type-alias for a [`Map`] that implements [`Debug`].
/// 
/// Note that this works because the trait object captures [`DebugEntry`]. If you want to combine
/// multiple capabilites, such as `PartialEq + Debug`, write a combined trait and use that as the
/// third argument to [`Map`].
pub type DebuggableMap<E, S = DefaultHashBuilder> = Map<E, S, DebugDynStorage<S>>;

#[allow(unused_imports)]
use std::fmt::Debug;
crate::create_entry_impl!(CloneableHashableAny<H> where crate::EntryAt<E, A>: Clone,);
crate::create_entry_impl!(PartialEqHashableAny<H> where crate::EntryAt<E, A>: PartialEq,);
crate::create_entry_impl!(DebugHashableAny<H> where crate::KeyAt<E, A>: Debug, crate::ValueAt<E, A>: Debug,);
