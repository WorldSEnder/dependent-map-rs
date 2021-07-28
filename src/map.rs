use core::hash::BuildHasher;
use hashbrown::raw::{Bucket, RawTable};
use std::any::Any;
use std::any::TypeId;
use std::borrow::Borrow;
use std::fmt::Debug;
use std::fmt::Formatter;
use std::hash::Hash;
use std::hash::Hasher;
use std::marker::PhantomData;
use std::ops::Deref;
use std::ops::DerefMut;

pub use dyn_clone::DynClone;

#[inline(always)]
fn unreachable_internal_invariant(_reason: &'static str) -> ! {
    #[cfg(debug_assertions)]
    {
        unreachable!(_reason)
    }
    #[cfg(not(debug_assertions))]
    unsafe {
        std::hint::unreachable_unchecked()
    }
}

/// internal trait, implemented for all `T: Sized + Any`,
/// in particular for `InnerEntry<E, A>` if `EntryAt<E, A>: Sized`.
#[allow(missing_docs)]
pub trait RefAny: Any {
    fn any_ref(&self) -> &dyn Any;
    fn any_mut(&mut self) -> &mut dyn Any;
    fn any_box(self: Box<Self>) -> Box<dyn Any>;
}
impl<T: Sized + Any> RefAny for T {
    #[inline]
    fn any_ref(&self) -> &dyn Any {
        self
    }
    #[inline]
    fn any_mut(&mut self) -> &mut dyn Any {
        self
    }
    #[inline]
    fn any_box(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}

/// internal trait, monomorphizing the specific type of hasher used for hashing entries.
#[allow(missing_docs)]
pub trait HashableAny<H: Hasher>: RefAny {
    fn specific_hash(&self, state: &mut H);
}
impl<T: Sized + Any + Hash, H: Hasher> HashableAny<H> for T {
    #[inline]
    fn specific_hash(&self, h: &mut H) {
        <T as Hash>::hash(self, h)
    }
}

/// Trait used to create internal boxed up storage from entries.
///
/// When `unstable_features` are enabled, this is implemented for a large range of traits,
/// otherwise only a select few arguments can be given to [`Map`].
/// 
/// Refer to [`create_entry_impl`] for the prefered method of writing implementations of this.
/// 
/// # Unsafety
/// 
/// Implementations promise that the returned Box contains the passed in entry in a fat pointer.
/// This must only be implemented for trait objects implying [`RefAny`], and when downcasting
/// the returned Box, the original entry must be returned.
/// 
/// [`create_entry_impl`]: crate::create_entry_impl
pub unsafe trait CreateEntry<A: ?Sized, E: ?Sized + EntryFamily<A>> {
    /// Create a boxed up internal storage from an entry
    fn from_entry(e: EntryAt<E, A>) -> Box<Self>;
}

/// Trait of entries that be stored in a hashmap-based type-dependent map.
pub trait HashEntry {
    /// The key part of the entry
    type Key: Eq + Hash;
    /// The value part of the entry
    type Value;
    /// Split the entry into key + value
    fn split_ref(&self) -> (&Self::Key, &Self::Value);
    /// Split the mutable entry into key + mutable value
    fn split_mut(&mut self) -> (&Self::Key, &mut Self::Value);
}

/// Object-safe [`PartialEq`] for comparing trait objects
pub trait DynPartialEq {
    /// Unsafe comparison: `other` is assumed to have same TypeId as Self.
    unsafe fn eq_dyn_unsafe(&self, other: &dyn Any) -> bool;
    /// Compare equality against a trait object implementing Any.
    fn eq_dyn(&self, other: &dyn Any) -> bool;
    #[inline]
    /// Compare inequality against a trait object implementing Any.
    fn ne_dyn(&self, other: &dyn Any) -> bool {
        !self.eq_dyn(other)
    }
}
impl<T: 'static + PartialEq<Self>> DynPartialEq for T {
    #[inline]
    unsafe fn eq_dyn_unsafe(&self, rhs: &dyn Any) -> bool {
        if !rhs.is::<Self>() {
            unreachable_internal_invariant("invariant for safely invoking this trait method");
        }
        // FIXME: speed this up with unsafe magic?
        Some(self) == rhs.downcast_ref::<Self>()
    }

    #[inline]
    fn eq_dyn(&self, rhs: &dyn Any) -> bool {
        if rhs.is::<Self>() {
            unsafe { self.eq_dyn_unsafe(rhs) }
        } else {
            false
        }
    }
}
/// Object-safe [`Eq`] for comparing trait objects
pub trait DynEq: DynPartialEq {}
impl<T: 'static + Eq> DynEq for T {}

/// [`Debug`] for entries
pub trait DebugEntry {
    /// Format the key of the entry
    fn fmt_key(&self, _: &mut Formatter<'_>) -> std::fmt::Result;
    /// Format the value of the entry
    fn fmt_value(&self, _: &mut Formatter<'_>) -> std::fmt::Result;
}
impl<A: 'static + ?Sized, E: ?Sized + EntryFamily<A>> DebugEntry for InnerEntry<E, A>
where
    KeyAt<E, A>: Debug,
    ValueAt<E, A>: Debug,
{
    fn fmt_key(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("")
            .field("type", &std::any::type_name::<A>())
            .field("key", self.key())
            .finish()
    }
    fn fmt_value(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.value().fmt(f)
    }
}

/// Formal family of the entry types in the map. If `E` is such a family, the [`Map`] maps,
/// at least intuitively, a pair of a type and an associated key to the associated value:
/// `(A: ?Sized, key: KeyAt<E, A>) -> value: ValueAt<E, A>`
pub trait EntryFamily<A: ?Sized> {
    /// The type of entry for the argument type `A`.
    type Result: HashEntry;
}
/// Convenience type alias for the entry at a specific argument
pub type EntryAt<E, A> = <E as EntryFamily<A>>::Result;
/// Convenience type alias for the key part of the [`HashEntry`] of an [`EntryFamily`] at a specific argument.
pub type KeyAt<E, A> = <EntryAt<E, A> as HashEntry>::Key;
/// Convenience type alias for the value part of the [`HashEntry`] of an [`EntryFamily`] at a specific argument.
pub type ValueAt<E, A> = <EntryAt<E, A> as HashEntry>::Value;

// FIXME: using Box here correct? I have no idea what the correct type to contain in PhantomData is, actually.
type NonOwningPhantomPointer<A> = PhantomData<Box<A>>;

#[repr(transparent)]
#[allow(missing_docs)]
pub struct InnerEntry<E: ?Sized + EntryFamily<A>, A: ?Sized> {
    entry: EntryAt<E, A>,
    _marker: NonOwningPhantomPointer<A>,
}

impl<A: ?Sized, E: ?Sized + EntryFamily<A>> Deref for InnerEntry<E, A> {
    type Target = EntryAt<E, A>;
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.entry
    }
}

impl<A: ?Sized, E: ?Sized + EntryFamily<A>> DerefMut for InnerEntry<E, A> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.entry
    }
}

impl<A: ?Sized, E: ?Sized + EntryFamily<A>> InnerEntry<E, A> {
    /// Create an inner entry from an instance of the entry family.
    pub fn new(entry: EntryAt<E, A>) -> Self { 
        Self {
            entry,
            _marker: PhantomData,
        }
    }

    #[inline]
    fn key(&self) -> &KeyAt<E, A> {
        self.split_ref().0
    }
    #[inline]
    fn value(&self) -> &ValueAt<E, A> {
        self.split_ref().1
    }
    #[inline]
    fn value_mut(&mut self) -> &mut ValueAt<E, A> {
        self.split_mut().1
    }
}

impl<A: ?Sized, E: ?Sized + EntryFamily<A>> PartialEq for InnerEntry<E, A>
where
    EntryAt<E, A>: PartialEq,
{
    #[inline]
    fn eq(&self, rhs: &Self) -> bool {
        self.entry == rhs.entry
    }
}

impl<A: ?Sized, E: ?Sized + EntryFamily<A>> Eq for InnerEntry<E, A> where EntryAt<E, A>: Eq {}

impl<A: ?Sized, E: ?Sized + EntryFamily<A>> Hash for InnerEntry<E, A> {
    #[inline]
    fn hash<H: Hasher>(&self, h: &mut H) {
        self.key().hash(h)
    }
}

impl<A: ?Sized, E: ?Sized + EntryFamily<A>> Clone for InnerEntry<E, A>
where
    EntryAt<E, A>: Clone,
{
    #[inline]
    fn clone(&self) -> Self {
        Self {
            entry: self.entry.clone(),
            _marker: std::marker::PhantomData,
        }
    }
}

struct RawEntry<E: ?Sized, I: ?Sized> {
    // Not actually *Any*, but a concrete instantiation of InnerEntry<_, E>
    // Implementation note: prefer to use `.inner()` or `.inner_mut()` when
    // using the Any interface, otherwise you might unintentionally work on the
    // Box instead of its content.
    inner: Box<I>,
    hash: u64,
    _marker: NonOwningPhantomPointer<E>,
}

#[cfg(feature = "unstable_features")]
unsafe impl<A: 'static + ?Sized, E: 'static + ?Sized + EntryFamily<A>, I: ?Sized> CreateEntry<A, E>
    for I
where
    InnerEntry<E, A>: std::marker::Unsize<I>,
{
    #[inline]
    fn from_entry(entry: EntryAt<E, A>) -> Box<Self>
    where
        E: EntryFamily<A>,
    {
        let boxed: Box<InnerEntry<E, A>> = Box::new(InnerEntry {
            entry,
            _marker: std::marker::PhantomData,
        });
        boxed // Unsize coercion
    }
}

mod macro_hygiene {
    /// Generate an implementation of [`CreateEntry`] for some trait object by coercing
    /// the inner entry into a box containing a `dyn`. For this to work, [`InnerEntry`] has
    /// to implement the trait.
    /// 
    /// The impl is generic in three arguments:
    /// - `H: Hasher` the hasher used for the map
    /// - `A: 'static + ?Sized` the argument to the entry family
    /// - `E: 'static + ?Sized + EntryFamily<A>` the entry family
    /// 
    /// # Example usage
    /// 
    /// ```rust
    /// # #[macro_use] extern crate dependent_map;
    /// # use dependent_map::{HashableAny, DynClone, DynEq, EntryAt};
    /// # use std::hash::Hasher;
    /// // Some trait alias we want to use as `Map<E, S, dyn SomeTraitFoo<H>>`
    /// // In this case, the Map should be cloneable and comparable for equality.
    /// trait SomeTraitFoo<H: Hasher>: HashableAny<H> + DynClone + DynEq {}
    /// impl<H: Hasher, T> SomeTraitFoo<H> for T where T: HashableAny<H> + DynClone + DynEq {}
    /// 
    /// create_entry_impl!(SomeTraitFoo<H> where EntryAt<E, A>: Clone + Eq,);
    /// ```
    /// 
    /// [`CreateEntry`]: crate::CreateEntry
    /// [`InnerEntry`]: crate::InnerEntry
    #[macro_export]
    #[cfg(not(feature = "unstable_features"))]
    macro_rules! create_entry_impl {
        ($hashable_name:path $(where $($bounded_type:ty: $bound:tt$( + $other_bounds:tt)*,)*)?) => {
            unsafe impl<H: ::std::hash::Hasher, A: 'static + ?Sized, E: 'static + ?Sized + $crate::EntryFamily<A>>
                $crate::CreateEntry<A, E> for dyn $hashable_name
            where $($($bounded_type: $bound $(+ $other_bounds)*,)*)?
            {
                #[inline]
                fn from_entry(entry: $crate::EntryAt<E, A>) -> ::std::boxed::Box<Self> {
                    let inner_entry: $crate::InnerEntry<E, A> = $crate::InnerEntry::new(entry);
                    ::std::boxed::Box::new(inner_entry)
                }
            }
        };
    }
    /// No-op for compatiblity with code generated with `unstable_features` turned off.
    /// 
    /// # Example usage
    /// 
    /// ```rust
    /// # #[macro_use] extern crate dependent_map;
    /// # use dependent_map::{HashableAny, DynClone, DynEq, EntryAt};
    /// # use std::hash::Hasher;
    /// // Some trait alias we want to use as `Map<E, S, dyn SomeTraitFoo<H>>`
    /// // In this case, the Map should be cloneable and comparable for equality.
    /// trait SomeTraitFoo<H: Hasher>: HashableAny<H> + DynClone + DynEq {}
    /// impl<H: Hasher, T> SomeTraitFoo<H> for T where T: HashableAny<H> + DynClone + DynEq {}
    /// 
    /// create_entry_impl!(SomeTraitFoo<H> where EntryAt<E, A>: Clone + Eq,);
    /// ```
    /// 
    /// [`CreateEntry`]: crate::CreateEntry
    /// [`InnerEntry`]: crate::InnerEntry
    #[macro_export]
    #[cfg(feature = "unstable_features")]
    macro_rules! create_entry_impl {
        ($hashable_name:path $(where $($bounded_type:ty: $bound:tt$( + $other_bounds:tt)*,)*)?) => { }
    }
}

impl<E: ?Sized, I: ?Sized> RawEntry<E, I> {
    #[inline]
    fn inner(&self) -> &I {
        &self.inner
    }
    #[inline]
    fn inner_mut(&mut self) -> &mut I {
        &mut self.inner
    }
}
// E is a nominal family describing the entries
impl<E: 'static + ?Sized, I: ?Sized + RefAny> RawEntry<E, I> {
    #[inline]
    fn downcast<A: 'static + ?Sized>(self) -> Option<InnerEntry<E, A>>
    where
        E: EntryFamily<A>,
    {
        let inner: Box<I> = self.inner;
        match inner.any_box().downcast::<InnerEntry<E, A>>() {
            Result::Ok(k) => Some(*k),
            // FIXME: throwing away information probably not a good idea
            // Alas, there doesn't seem to be a way to go back to Box<dyn HashableAny<H>>.
            Result::Err(_) => None,
        }
    }

    #[inline]
    fn downcast_ref<A: 'static + ?Sized>(&self) -> Option<&InnerEntry<E, A>>
    where
        E: EntryFamily<A>,
    {
        self.inner().any_ref().downcast_ref::<InnerEntry<E, A>>()
    }

    #[inline]
    fn downcast_mut<A: 'static + ?Sized>(&mut self) -> Option<&mut InnerEntry<E, A>>
    where
        E: EntryFamily<A>,
    {
        self.inner_mut()
            .any_mut()
            .downcast_mut::<InnerEntry<E, A>>()
    }
}

impl<E: ?Sized, I: ?Sized> RawEntry<E, I> {
    #[inline]
    fn new<A: ?Sized>(hash: u64, entry: EntryAt<E, A>) -> Self
    where
        E: EntryFamily<A>,
        I: CreateEntry<A, E>,
    {
        Self {
            inner: I::from_entry(entry),
            hash,
            _marker: std::marker::PhantomData,
        }
    }
}

/// The default [`BuildHasher`] used in the map, i.e. [`hashbrown::hash_map::DefaultHashBuilder`].
pub type DefaultHashBuilder = hashbrown::hash_map::DefaultHashBuilder;
/// The Hasher type corresponding to [`DefaultHashBuilder`]
pub type DefaultHasher = <DefaultHashBuilder as BuildHasher>::Hasher;

type DynStorage<S> = dyn HashableAny<<S as BuildHasher>::Hasher>;
crate::create_entry_impl!(HashableAny<H>);

/// A hash map implemented with quadratic probing and SIMD lookup.
///
/// The type of the entry stored in the map depends on the type of key used.
/// For this to work, `E` - the entry family - should implement [`EntryFamily`] for each type of
/// argument you want to store in the map.
///
pub struct Map<
    E: ?Sized,
    S: BuildHasher = DefaultHashBuilder,
    I: ?Sized + HashableAny<S::Hasher> = DynStorage<S>,
> {
    // IMPORTANT: we cache the hash in each RawEntry, so the hash_state can not change without
    // rehashing all items. Dynamic dispatch via HashableAny<S::Hasher> would then be needed,
    // which is also how PartialEq works.
    hash_state: S,
    raw: RawTable<RawEntry<E, I>>,
}

/// An occupied entry in an [`Map`], containing the key that was used during lookup and the
/// bucket where the entry is placed in the map. Can save on repeated lookups of the same
/// key in some scenarios, but users should usually prefer the direct api of the map.
pub struct OccupiedEntry<
    'a,
    A: ?Sized,
    E: ?Sized + EntryFamily<A>,
    S: BuildHasher,
    I: ?Sized + HashableAny<S::Hasher>,
> {
    _hash: u64,
    key: KeyAt<E, A>,
    elem: Bucket<RawEntry<E, I>>,
    table: &'a mut Map<E, S, I>,
}

impl<
        'a,
        A: 'static + ?Sized,
        E: 'static + ?Sized + EntryFamily<A>,
        S: BuildHasher,
        I: ?Sized + HashableAny<S::Hasher>,
    > OccupiedEntry<'a, A, E, S, I>
{
    #[inline]
    fn entry(&self) -> &InnerEntry<E, A> {
        // holding a ref to the table, didn't rehash or reallocate
        let inner_ref = unsafe { self.elem.as_ref() };
        match inner_ref.downcast_ref() {
            Some(r) => r,
            // invariant of how we obtained the entry
            None => unreachable_internal_invariant(
                "the entry is constructed pointing only at correct types",
            ),
        }
    }
    #[inline]
    fn entry_mut(&mut self) -> &mut InnerEntry<E, A> {
        // holding a ref to the table, didn't rehash or reallocate
        let inner_ref = unsafe { self.elem.as_mut() };
        // FIXME: borrowing the whole entry can invalidate the key and hence the hash value
        // in the map. How do we handle that?
        match inner_ref.downcast_mut() {
            Some(r) => r,
            // invariant of how we obtained the entry
            None => unreachable_internal_invariant(
                "the entry is constructed pointing only at correct types",
            ),
        }
    }
    #[inline]
    /// Get the key used during lookup of the entry
    pub fn key(&self) -> &KeyAt<E, A> {
        // FIXME: technically we have two keys: the one used during lookup and the one stored in the map.
        // maybe some apis want access to both?
        &self.key
    }
    #[inline]
    /// Get the pair of (key, value) found in the map for this entry.
    pub fn hash_entry(&self) -> &EntryAt<E, A> {
        self.entry()
    }
    #[inline]
    /// Get the pair of (key, value) found in the map for this entry.
    ///
    /// # Unsafe
    ///
    /// Unsafe if the mutable reference is used to modify the hash for the key of the entry.
    pub unsafe fn hash_entry_mut(&mut self) -> &mut EntryAt<E, A> {
        // FIXME: possibly should be unsafe, since it could modify the key (and thus the hashvalue) of the entry.
        self.entry_mut()
    }
    #[inline]
    /// Get the value found in the map for this entry
    pub fn get(&self) -> &ValueAt<E, A> {
        self.entry().value()
    }
    #[inline]
    /// Get the value found in the map for this entry
    pub fn get_mut(&mut self) -> &mut ValueAt<E, A> {
        self.entry_mut().value_mut()
    }
    #[inline]
    /// Replace the value found in the map for this entry and return the old value
    pub fn insert(&mut self, value: ValueAt<E, A>) -> ValueAt<E, A> {
        let place = self.get_mut();
        std::mem::replace(place, value)
    }
    #[inline]
    /// Remove and return the entry from the map
    pub fn remove_entry(self) -> EntryAt<E, A> {
        // holding a ref to the table, didn't rehash or reallocate
        let raw = unsafe { self.table.raw.remove(self.elem) };
        match raw.downcast() {
            Some(r) => r,
            // invariant of how we obtained the entry
            None => unreachable_internal_invariant(
                "the entry is constructed pointing only at correct types",
            ),
        }
        .entry
    }
}

/// A vacant entry in an [`Map`].
pub struct VacantEntry<
    'a,
    A: ?Sized,
    E: ?Sized + EntryFamily<A>,
    S: BuildHasher,
    I: ?Sized + HashableAny<S::Hasher>,
> {
    hash: u64,
    key: KeyAt<E, A>,
    table: &'a mut Map<E, S, I>,
}

impl<
        'a,
        A: 'static + ?Sized,
        E: 'static + ?Sized + EntryFamily<A>,
        S: BuildHasher,
        I: ?Sized + HashableAny<S::Hasher>,
    > VacantEntry<'a, A, E, S, I>
{
    #[inline]
    /// Get the key that was used during lookup
    pub fn key(&self) -> &KeyAt<E, A> {
        &self.key
    }
    #[inline]
    /// Take ownership of the key that was used during lookup
    pub fn into_key(self) -> KeyAt<E, A> {
        self.key
    }
    /// Insert an entry, taking ownership of the already supplied key.
    ///
    /// Returns mutable access to inserted value.
    pub fn insert(
        self,
        value: impl 'a + FnOnce(KeyAt<E, A>) -> EntryAt<E, A>,
    ) -> &'a mut ValueAt<E, A>
    where
        I: CreateEntry<A, E>,
    {
        let raw_entry = RawEntry::new(self.hash, value(self.key));
        let hashfn = make_hasher(&self.table.hash_state);
        let ins_entry = self.table.raw.insert_entry(self.hash, raw_entry, hashfn);
        match ins_entry.downcast_mut() {
            Some(m) => m.value_mut(),
            None => unreachable_internal_invariant("inserted type is correct"),
        }
    }
    #[inline]
    /// Insert an entry, by converting the key into an entry.
    ///
    /// Returns mutable access to inserted value.
    pub fn insert_into(self) -> &'a mut ValueAt<E, A>
    where
        I: CreateEntry<A, E>,
        EntryAt<E, A>: From<KeyAt<E, A>>,
    {
        self.insert(|k| k.into())
    }
}

/// An entry in an [`Map`]
pub enum Entry<
    'a,
    A: ?Sized,
    E: ?Sized + EntryFamily<A>,
    S: BuildHasher,
    I: ?Sized + HashableAny<S::Hasher>,
> {
    #[allow(missing_docs)]
    Occupied(OccupiedEntry<'a, A, E, S, I>),
    #[allow(missing_docs)]
    Vacant(VacantEntry<'a, A, E, S, I>),
}

impl<E: ?Sized, S: BuildHasher, I: ?Sized + HashableAny<S::Hasher>> Map<E, S, I> {
    #[inline]
    /// Create a new, empty, [`Map`].
    pub fn new() -> Self
    where
        S: Default,
    {
        Self {
            raw: RawTable::new(),
            hash_state: S::default(),
        }
    }
    #[inline]
    /// Create a new, empty, [`Map`] with a specified initial capacity.
    pub fn with_capacity(capacity: usize) -> Self
    where
        S: Default,
    {
        Self {
            raw: RawTable::with_capacity(capacity),
            hash_state: S::default(),
        }
    }
    #[inline]
    /// Get the capacity of the backing storage.
    pub fn capacity(&self) -> usize {
        self.raw.capacity()
    }
    #[inline]
    /// Get the number of occupied entries in the backing storage.
    pub fn len(&self) -> usize {
        self.raw.len()
    }
    #[inline]
    /// Check if the map is empty, i.e. `len() == 0`.
    pub fn is_empty(&self) -> bool {
        self.raw.len() == 0
    }
    #[inline]
    /// Clear the map, but preserve the currently reserved capacity.
    pub fn clear(&mut self) {
        self.raw.clear()
    }
}

#[inline]
fn hash_def_entry<S: BuildHasher, I: ?Sized + HashableAny<S::Hasher>>(
    state: &S,
    e: &I,
) -> u64 {
    let mut hasher = state.build_hasher();
    e.any_ref().type_id().hash(&mut hasher);
    e.specific_hash(&mut hasher);
    hasher.finish()
}

#[inline]
fn hash_def_key<
    Q: ?Sized + Hash,
    A: 'static + ?Sized,
    E: 'static + ?Sized + EntryFamily<A>,
    S: BuildHasher,
>(
    state: &S,
    key: &Q,
) -> u64 {
    let mut hasher = state.build_hasher();
    TypeId::of::<InnerEntry<E, A>>().hash(&mut hasher);
    key.hash(&mut hasher);
    hasher.finish()
}

fn equivalent_key<A: 'static + ?Sized, E: 'static + ?Sized, Q: ?Sized + Eq, I: ?Sized + RefAny>(
    key: &Q,
) -> impl '_ + FnMut(&RawEntry<E, I>) -> bool
where
    E: EntryFamily<A>,
    KeyAt<E, A>: Borrow<Q>,
{
    move |e| {
        let downcast = e.downcast_ref();
        match downcast {
            Some(r) => key == r.key().borrow(),
            None => false,
        }
    }
}

// Deep equivalence comparison, not simply comparing the key
fn equivalent_entry<E: 'static + ?Sized, I: ?Sized + RefAny + DynPartialEq>(
    lhs: &RawEntry<E, I>,
) -> impl '_ + Fn(&RawEntry<E, I>) -> bool {
    move |rhs| lhs.inner.eq_dyn((*rhs.inner).any_ref())
}

fn make_hasher<E: ?Sized, S: BuildHasher, I: ?Sized + HashableAny<S::Hasher>>(
    _state: &S,
) -> impl '_ + Fn(&RawEntry<E, I>) -> u64 {
    move |val: &RawEntry<E, I>| val.hash
}

impl<E: 'static + ?Sized, S: BuildHasher, I: ?Sized + HashableAny<S::Hasher>> Map<E, S, I> {
    #[inline]
    fn hash_key<A: 'static + ?Sized, Q: ?Sized + Hash>(&self, key: &Q) -> u64
    where
        E: EntryFamily<A>,
    {
        hash_def_key::<_, A, E, S>(&self.hash_state, key)
    }
    #[inline]
    fn get_inner<A: 'static + ?Sized, Q: ?Sized>(&self, key: &Q) -> Option<&InnerEntry<E, A>>
    where
        E: EntryFamily<A>,
        KeyAt<E, A>: Borrow<Q>,
        Q: Hash + Eq,
    {
        let hash = self.hash_key::<A, _>(&key);
        let entry = self.raw.get(hash, equivalent_key(key));
        match entry {
            Some(e) => match e.downcast_ref() {
                r @ Some(_) => r,
                // safety: invariant for equivalent key checking downcast succeeds
                None => unreachable_internal_invariant("hash+equivalent key for the correct type"),
            },
            None => None,
        }
    }

    #[inline]
    fn get_inner_mut_by_hash<A: 'static + ?Sized, Q: ?Sized>(
        &mut self,
        hash: u64,
        key: &Q,
    ) -> Option<&mut InnerEntry<E, A>>
    where
        E: EntryFamily<A>,
        KeyAt<E, A>: Borrow<Q>,
        Q: Hash + Eq,
    {
        let entry = self.raw.get_mut(hash, equivalent_key(key));
        match entry {
            Some(e) => match e.downcast_mut() {
                r @ Some(_) => r,
                // safety: invariant for equivalent key checking downcast succeeds
                None => unreachable_internal_invariant("hash+equivalent key for the correct type"),
            },
            None => None,
        }
    }

    #[inline]
    fn get_inner_mut<A: 'static + ?Sized, Q: ?Sized>(
        &mut self,
        key: &Q,
    ) -> Option<&mut InnerEntry<E, A>>
    where
        E: EntryFamily<A>,
        KeyAt<E, A>: Borrow<Q>,
        Q: Hash + Eq,
    {
        let hash = self.hash_key(key);
        self.get_inner_mut_by_hash(hash, key)
    }

    #[inline]
    /// Reserve storage to fit at least `additional` more entries without reallocating.
    pub fn reserve(&mut self, additional: usize) {
        let hashfn = make_hasher(&self.hash_state);
        self.raw.reserve(additional, hashfn)
    }
    #[inline]
    /// Shrink storage to fit the currently used size.
    pub fn shrink_to_fit(&mut self) {
        let hashfn = make_hasher(&self.hash_state);
        self.raw.shrink_to(0, hashfn)
    }
    #[inline]
    /// Lookup the entry at `key`.
    pub fn entry<A: 'static + ?Sized>(&mut self, key: KeyAt<E, A>) -> Entry<'_, A, E, S, I>
    where
        E: EntryFamily<A>,
    {
        let hash = self.hash_key(&key);
        match self.raw.find(hash, equivalent_key(&key)) {
            Some(bucket) => Entry::Occupied(OccupiedEntry {
                _hash: hash,
                key,
                elem: bucket,
                table: self,
            }),
            None => Entry::Vacant(VacantEntry {
                hash,
                key,
                table: self,
            }),
        }
    }
    #[inline]
    /// Check if the map contains a value for the specified key.
    pub fn contains_key<A: 'static + ?Sized, Q: ?Sized>(&self, k: &Q) -> bool
    where
        E: EntryFamily<A>,
        KeyAt<E, A>: Borrow<Q>,
        Q: Hash + Eq,
    {
        self.get_inner(k).is_some()
    }
    #[inline]
    /// Returns a reference to the value corresponding to the key.
    pub fn get<A: 'static + ?Sized, Q: ?Sized>(&self, k: &Q) -> Option<&EntryAt<E, A>>
    where
        E: EntryFamily<A>,
        KeyAt<E, A>: Borrow<Q>,
        Q: Hash + Eq,
    {
        // Avoid `Option::map` because it bloats LLVM IR.
        match self.get_inner::<A, Q>(k) {
            Some(inner) => Some(inner),
            None => None,
        }
    }
    #[inline]
    /// Returns a reference to the value corresponding to the default key.
    pub fn get_default<A: 'static + ?Sized>(&self) -> Option<&EntryAt<E, A>>
    where
        E: EntryFamily<A>,
        KeyAt<E, A>: Default,
    {
        self.get(&Default::default())
    }
    #[inline]
    /// Returns a mutable reference to the value corresponding to the key.
    pub fn get_mut<A: 'static + ?Sized, Q: ?Sized>(&mut self, k: &Q) -> Option<&mut ValueAt<E, A>>
    where
        E: EntryFamily<A>,
        KeyAt<E, A>: Borrow<Q>,
        Q: Hash + Eq,
    {
        // Avoid `Option::map` because it bloats LLVM IR.
        match self.get_inner_mut(k) {
            Some(inner) => Some(inner.value_mut()),
            None => None,
        }
    }
    /// Inserts an entry into the map.
    ///
    /// If the map did not have this key present, [`None`] is returned.
    ///
    /// Otherwise, the entry is fully replaced and `Some(old)` where `old` is the old entry is returned.
    pub fn insert<A: 'static + ?Sized, P: ?Sized>(&mut self, entry: P) -> Option<EntryAt<E, A>>
    where
        E: EntryFamily<A>,
        I: CreateEntry<A, E>,
        P: Into<EntryAt<E, A>>,
    {
        let entry = entry.into();
        let key = entry.split_ref().0;
        let hash = self.hash_key(key);
        if let Some(existing) = self.get_inner_mut_by_hash(hash, key) {
            Some(std::mem::replace(&mut existing.entry, entry))
        } else {
            let raw_entry = RawEntry::<E, I>::new(hash, entry);
            let hashfn = make_hasher(&self.hash_state);
            let _ = self.raw.insert(hash, raw_entry, hashfn);
            None
        }
    }
    /// Removes a key from the map, returning the value at the key if the key
    /// was previously in the map.
    pub fn remove_entry<A: 'static + ?Sized, Q: ?Sized>(&mut self, key: &Q) -> Option<EntryAt<E, A>>
    where
        E: EntryFamily<A>,
        KeyAt<E, A>: Borrow<Q>,
        Q: Hash + Eq,
    {
        let hash = self.hash_key(key);
        // Avoid `Option::map` because it bloats LLVM IR.
        match self.raw.remove_entry(hash, equivalent_key(key)) {
            Some(v) => match v.downcast() {
                Some(raw) => Some(raw.entry),
                None => unreachable_internal_invariant("hash+equivalent key for the correct type"),
            },
            None => None,
        }
    }
    /*
    // FIXME: add split method to Entry that returns values instead of references?
    //  dubious anyway, since that throws away the key
    pub fn remove<A: 'static + ?Sized, Q: ?Sized>(&mut self, k: &Q) -> Option<ValueAt<E, A>>
    where
        E: EntryFamily<A>,
        KeyAt<E, A>: Borrow<Q>,
        Q: Hash + Eq,
    {
        // Avoid `Option::map` because it bloats LLVM IR.
        match self.remove_entry(k) {
            Some(v) => Some(v),
            None => None,
        }
    }
    */
    fn iter(&self) -> impl '_ + Iterator<Item = &'_ RawEntry<E, I>> {
        // Unsafety: lifetime is captured, so map must outlive it
        let it = unsafe { self.raw.iter() };
        it.map(|b| {
            // Unsafety: borrow is active, so Bucket outlives it
            unsafe { b.as_ref() }
        })
    }
    fn _iter_mut(&mut self) -> impl '_ + Iterator<Item = &'_ mut RawEntry<E, I>> {
        // Unsafety: lifetime is captured, so map must outlive it
        let it = unsafe { self.raw.iter() };
        it.map(|b| {
            // Unsafety: borrow is active, so Bucket outlives it
            unsafe { b.as_mut() }
        })
    }
}

impl<E: ?Sized, I: ?Sized + DynClone> Clone for RawEntry<E, I> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            inner: dyn_clone::clone_box(&self.inner),
            hash: self.hash,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<E: ?Sized, S: BuildHasher + Clone, I: ?Sized + HashableAny<S::Hasher> + DynClone> Clone
    for Map<E, S, I>
{
    #[inline]
    fn clone(&self) -> Self {
        Self {
            hash_state: self.hash_state.clone(),
            raw: self.raw.clone(),
        }
    }
}

impl<E: ?Sized, S: Default + BuildHasher, I: ?Sized + HashableAny<S::Hasher>> Default
    for Map<E, S, I>
{
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<
        E: 'static + ?Sized,
        S: Default + BuildHasher,
        I: ?Sized + HashableAny<S::Hasher> + DynPartialEq,
    > PartialEq for Map<E, S, I>
{
    fn eq(&self, rhs: &Self) -> bool {
        if self.len() != rhs.len() {
            return false;
        }

        // TODO: should this care about the different hash states? Probably not
        self.iter().all(|entry| {
            let rhash = hash_def_entry(&rhs.hash_state, entry.inner());
            matches!(rhs.raw.get(rhash, equivalent_entry(entry)), Some(_))
            // rhs.get_inner(key: &Q)
        })
    }
}

impl<E: 'static + ?Sized, S: Default + BuildHasher, I: ?Sized + HashableAny<S::Hasher> + DynEq> Eq
    for Map<E, S, I>
{
}

struct SomeKey<'a, E: ?Sized, I: ?Sized>(&'a RawEntry<E, I>);
struct SomeValue<'a, E: ?Sized, I: ?Sized>(&'a RawEntry<E, I>);
impl<'a, E: ?Sized, I: ?Sized + DebugEntry> Debug for SomeKey<'a, E, I> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.inner().fmt_key(f)
    }
}
impl<'a, E: ?Sized, I: ?Sized + DebugEntry> Debug for SomeValue<'a, E, I> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.inner().fmt_value(f)
    }
}

impl<E: 'static + ?Sized, S: Default + BuildHasher, I: ?Sized + HashableAny<S::Hasher> + DebugEntry>
    Debug for Map<E, S, I>
{
    fn fmt(&self, fmt: &mut Formatter<'_>) -> std::fmt::Result {
        fmt.debug_map()
            .entries(self.iter().map(|e| (SomeKey(e), SomeValue(e))))
            .finish()
    }
}
