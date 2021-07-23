use std::any::TypeId;
use core::hash::BuildHasher;
use dyn_clone::DynClone;
use hashbrown::raw::Bucket;
use hashbrown::raw::RawTable;
use std::any::Any;
use std::borrow::Borrow;
use std::hash::Hash;
use std::hash::Hasher;
use std::ops::Deref;
use std::ops::DerefMut;

/// internal trait
#[allow(missing_docs)]
pub trait HashableAny<H: Hasher>: Any {
    fn specific_hash(&self, state: &mut H);
    fn any_ref(&self) -> &dyn Any;
    fn any_mut(&mut self) -> &mut dyn Any;
    fn any_box(self: Box<Self>) -> Box<dyn Any>;
}
/// mostly internal trait, that should be implemented by the internal boxed up storage.
#[allow(missing_docs)]
pub trait HashableTrait: HashableAny<Self::Hasher> {
    type Hasher: Hasher;
}
/// mostly internal trait, that should be implemented by the internal boxed up storage.
pub trait CreateEntry<A: ?Sized, E: ?Sized + EntryFamily<A>> {
    /// Create a boxed up internal storage from an entry
    fn from_entry(e: EntryAt<E, A>) -> Box<Self>;
}
/// internal trait
pub trait CloneableHashableAny<H: Hasher>: HashableAny<H> + DynClone {}
impl<H: Hasher, T> CloneableHashableAny<H> for T where T: HashableAny<H> + DynClone {}

impl<T: Any + Hash, H: Hasher> HashableAny<H> for T {
    #[inline]
    fn specific_hash(&self, h: &mut H) {
        <T as Hash>::hash(self, h)
    }
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

/// Glue trait of entries that be stored in a hashmap-based type-dependent map.
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

/// Formal family of the entry types in the map. If `E` is such a family, the [`AnyMap`] maps,
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

//pub trait EntryFamilyClone<A: ?Sized>: EntryFamily<A, Result = Self::ResultClone> {
//    type ResultClone: Clone + HashEntry;
//}

struct InnerEntry<E: ?Sized + EntryFamily<A>, A: ?Sized> {
    entry: EntryAt<E, A>,
    _marker: std::marker::PhantomData<*const A>,
}

impl<A: ?Sized, E: ?Sized + EntryFamily<A>> Deref for InnerEntry<E, A> {
    type Target = EntryAt<E, A>;
    fn deref(&self) -> &Self::Target {
        &self.entry
    }
}

impl<A: ?Sized, E: ?Sized + EntryFamily<A>> DerefMut for InnerEntry<E, A> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.entry
    }
}

impl<A: ?Sized, E: ?Sized + EntryFamily<A>> InnerEntry<E, A> {
    fn key(&self) -> &KeyAt<E, A> {
        self.split_ref().0
    }
    fn value(&self) -> &ValueAt<E, A> {
        self.split_ref().1
    }
    fn value_mut(&mut self) -> &mut ValueAt<E, A> {
        self.split_mut().1
    }
}

impl<A: ?Sized, E: ?Sized + EntryFamily<A>> PartialEq for InnerEntry<E, A> {
    fn eq(&self, rhs: &Self) -> bool {
        self.key() == rhs.key()
    }
}
impl<A: ?Sized, E: ?Sized + EntryFamily<A>> Eq for InnerEntry<E, A> {}

impl<A: ?Sized, E: ?Sized + EntryFamily<A>> Hash for InnerEntry<E, A> {
    fn hash<H: Hasher>(&self, h: &mut H) {
        self.key().hash(h)
    }
}

struct RawEntry<E: ?Sized, I: ?Sized> {
    // Not actually *Any*, but a concrete instantiation of InnerEntry<_, E>
    inner: Box<I>,
    _marker: std::marker::PhantomData<*const E>,
}

impl<H: 'static + Hasher> HashableTrait for dyn HashableAny<H> {
    type Hasher = H;
}
impl<H: 'static + Hasher, A: 'static + ?Sized, E: 'static + ?Sized + EntryFamily<A>>
    CreateEntry<A, E> for dyn HashableAny<H>
{
    fn from_entry(entry: EntryAt<E, A>) -> std::boxed::Box<Self> {
        let inner_entry: InnerEntry<E, A> = InnerEntry {
            entry,
            _marker: std::marker::PhantomData,
        };
        Box::new(inner_entry)
    }
}

impl<H: 'static + Hasher> HashableTrait for dyn CloneableHashableAny<H> {
    type Hasher = H;
}

// E is a nominal family describing the entries
impl<E: 'static + ?Sized, I: ?Sized + HashableTrait> RawEntry<E, I> {
    fn downcast<A: 'static + ?Sized>(self) -> Option<InnerEntry<E, A>>
    where
        E: EntryFamily<A>,
    {
        match self.inner.any_box().downcast::<InnerEntry<E, A>>() {
            Result::Ok(k) => Some(*k),
            // FIXME: throwing away information probably not a good idea
            // Alas, there doesn't seem to be a way to go back to Box<dyn HashableAny<H>>.
            Result::Err(_) => None,
        }
    }

    fn downcast_ref<A: 'static + ?Sized>(&self) -> Option<&InnerEntry<E, A>>
    where
        E: EntryFamily<A>,
    {
        self.inner.any_ref().downcast_ref::<InnerEntry<E, A>>()
    }

    fn downcast_mut<A: 'static + ?Sized>(&mut self) -> Option<&mut InnerEntry<E, A>>
    where
        E: EntryFamily<A>,
    {
        self.inner.any_mut().downcast_mut::<InnerEntry<E, A>>()
    }
}

type DynStorage<S> = dyn HashableAny<<S as BuildHasher>::Hasher>;
type CloneDynStorage<S> = dyn CloneableHashableAny<<S as BuildHasher>::Hasher>;

impl<E: 'static + ?Sized, I: ?Sized> RawEntry<E, I> {
    fn new<A: 'static + ?Sized>(entry: EntryAt<E, A>) -> Self
    where
        E: EntryFamily<A>,
        I: CreateEntry<A, E>,
    {
        Self {
            inner: I::from_entry(entry),
            _marker: std::marker::PhantomData,
        }
    }
}

/// A hash map implemented with quadratic probing and SIMD lookup.
///
/// They type of the entry stored in the map depends on the type of key used.
/// For this to work, `E` - the entry family - should implement [`EntryFamily`] for each type of
/// argument you want to store in the map.
///
pub struct AnyMap<
    E: ?Sized,
    S: BuildHasher = hashbrown::hash_map::DefaultHashBuilder,
    I: ?Sized = DynStorage<S>,
> {
    hash_state: S,
    raw: RawTable<RawEntry<E, I>>,
}
/// Type-alias for an [`AnyMap`] that can be cloned.
pub type CloneableAnyMap<E, S = hashbrown::hash_map::DefaultHashBuilder> =
    AnyMap<E, S, CloneDynStorage<S>>;

/// An occupied entry in an `AnyMap`, containing the key that was used during lookup and the
/// bucket where the entry is placed in the map. Can save on repeated lookups of the same
/// key in some scenarios, but users should usually prefer the direct api of the map.
pub struct OccupiedEntry<
    'a,
    A: ?Sized,
    E: ?Sized + EntryFamily<A>,
    S: BuildHasher,
    I: ?Sized = DynStorage<S>,
> {
    _hash: u64,
    key: KeyAt<E, A>,
    elem: Bucket<RawEntry<E, I>>,
    table: &'a mut AnyMap<E, S, I>,
}

impl<
        'a,
        A: 'static + ?Sized,
        E: 'static + ?Sized + EntryFamily<A>,
        S: 'static + BuildHasher,
        I: ?Sized + HashableTrait<Hasher = S::Hasher>,
    > OccupiedEntry<'a, A, E, S, I>
{
    fn entry(&self) -> &InnerEntry<E, A> {
        // holding a ref to the table, didn't rehash or reallocate
        let inner_ref = unsafe { self.elem.as_ref() };
        match inner_ref.downcast_ref() {
            Some(r) => r,
            // invariant of how we obtained the entry
            None => unsafe { std::hint::unreachable_unchecked() },
        }
    }
    fn entry_mut(&mut self) -> &mut InnerEntry<E, A> {
        // holding a ref to the table, didn't rehash or reallocate
        let inner_ref = unsafe { self.elem.as_mut() };
        // FIXME: borrowing the whole entry can invalidate the key and hence the hash value
        // in the map. How do we handle that?
        match inner_ref.downcast_mut() {
            Some(r) => r,
            // invariant of how we obtained the entry
            None => unsafe { std::hint::unreachable_unchecked() },
        }
    }
    /// Get the key used during lookup of the entry
    pub fn key(&self) -> &KeyAt<E, A> {
        // FIXME: technically we have two keys: the one used during lookup and the one stored in the map.
        // maybe some apis want access to both?
        &self.key
    }
    /// Get the pair of (key, value) found in the map for this entry.
    pub fn hash_entry(&self) -> &EntryAt<E, A> {
        self.entry()
    }
    /// Get the pair of (key, value) found in the map for this entry.
    ///
    /// # Unsafe
    ///
    /// Unsafe if the mutable reference is used to modify the hash for the key of the entry.
    pub unsafe fn hash_entry_mut(&mut self) -> &mut EntryAt<E, A> {
        // FIXME: possibly should be unsafe, since it could modify the key (and thus the hashvalue) of the entry.
        self.entry_mut()
    }
    /// Get the value found in the map for this entry
    pub fn get(&self) -> &ValueAt<E, A> {
        self.entry().value()
    }
    /// Get the value found in the map for this entry
    pub fn get_mut(&mut self) -> &mut ValueAt<E, A> {
        self.entry_mut().value_mut()
    }
    /// Replace the value found in the map for this entry and return the old value
    pub fn insert(&mut self, value: ValueAt<E, A>) -> ValueAt<E, A> {
        let place = self.get_mut();
        std::mem::replace(place, value)
    }
    /// Remove and return the entry from the map
    pub fn remove_entry(self) -> EntryAt<E, A> {
        // holding a ref to the table, didn't rehash or reallocate
        let raw = unsafe { self.table.raw.remove(self.elem) };
        match raw.downcast() {
            Some(r) => r,
            // invariant of how we obtained the entry
            None => unsafe { std::hint::unreachable_unchecked() },
        }
        .entry
    }
}

/// A vacant entry in an [`AnyMap`].
pub struct VacantEntry<
    'a,
    A: ?Sized,
    E: ?Sized + EntryFamily<A>,
    S: BuildHasher,
    I: ?Sized = DynStorage<S>,
> {
    hash: u64,
    key: KeyAt<E, A>,
    table: &'a mut AnyMap<E, S, I>,
}

impl<
        'a,
        A: 'static + ?Sized,
        E: 'static + ?Sized + EntryFamily<A>,
        S: 'static + BuildHasher,
        I: ?Sized + HashableTrait<Hasher = S::Hasher>,
    > VacantEntry<'a, A, E, S, I>
{
    /// Get the key that was used during lookup
    pub fn key(&self) -> &KeyAt<E, A> {
        &self.key
    }
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
        let raw_entry = RawEntry::new(value(self.key));
        let hashfn = make_hasher(&self.table.hash_state);
        let ins_entry = self.table.raw.insert_entry(self.hash, raw_entry, hashfn);
        match ins_entry.downcast_mut() {
            Some(m) => m.value_mut(),
            None => unsafe { std::hint::unreachable_unchecked() },
        }
    }
    /// Insert an entry, by converting the key into an entry.
    ///
    /// Returns mutable access to inserted value.
    pub fn insert_into(
        self,
    ) -> &'a mut ValueAt<E, A>
    where
        I: CreateEntry<A, E>,
        EntryAt<E, A>: From<KeyAt<E, A>>
    {
        self.insert(|k| k.into())
    }
}

/// An entry in an [`AnyMap`]
pub enum Entry<'a, A: ?Sized, E: ?Sized + EntryFamily<A>, S: BuildHasher, I: ?Sized = DynStorage<S>>
{
    #[allow(missing_docs)]
    Occupied(OccupiedEntry<'a, A, E, S, I>),
    #[allow(missing_docs)]
    Vacant(VacantEntry<'a, A, E, S, I>),
}

impl<E: ?Sized, S: BuildHasher, I: ?Sized> AnyMap<E, S, I> {
    /// Create a new, empty, [`AnyMap`].
    pub fn new() -> Self
    where
        S: Default,
    {
        Self {
            raw: RawTable::new(),
            hash_state: S::default(),
        }
    }
    /// Create a new, empty, [`AnyMap`] with a specified initial capacity.
    pub fn with_capacity(capacity: usize) -> Self
    where
        S: Default,
    {
        Self {
            raw: RawTable::with_capacity(capacity),
            hash_state: S::default(),
        }
    }
    /// Get the capacity of the backing storage.
    pub fn capacity(&self) -> usize {
        self.raw.capacity()
    }
    /// Get the number of occupied entries in the backing storage.
    pub fn len(&self) -> usize {
        self.raw.len()
    }
    /// Check if the map is empty, i.e. `len() == 0`.
    pub fn is_empty(&self) -> bool {
        self.raw.len() == 0
    }
    /// Clear the map, but preserve the currently reserved capacity.
    pub fn clear(&mut self) {
        self.raw.clear()
    }
}

fn hash_def_entry<
    E: 'static + ?Sized,
    S: 'static + BuildHasher,
    I: ?Sized + HashableTrait<Hasher = S::Hasher>,
>(
    state: &S,
    e: &RawEntry<E, I>,
) -> u64 {
    let mut hasher = state.build_hasher();
    e.inner.any_ref().type_id().hash(&mut hasher);
    e.inner.specific_hash(&mut hasher);
    hasher.finish()
}
fn hash_def_key<
    Q: ?Sized + Hash,
    A: 'static + ?Sized,
    E: 'static + ?Sized + EntryFamily<A>,
    S: 'static + BuildHasher,
>(state: &S, key: &Q) -> u64 {
    let mut hasher = state.build_hasher();
    TypeId::of::<InnerEntry<E, A>>().hash(&mut hasher);
    key.hash(&mut hasher);
    hasher.finish()
}

fn make_hasher<
    E: 'static + ?Sized,
    S: 'static + BuildHasher,
    I: ?Sized + HashableTrait<Hasher = S::Hasher>,
>(
    state: &S,
) -> impl '_ + Fn(&RawEntry<E, I>) -> u64 {
    move |val: &RawEntry<E, I>| hash_def_entry(state, val)
}

fn equivalent_key<
    A: 'static + ?Sized,
    E: 'static + ?Sized,
    Q: ?Sized + Eq,
    I: ?Sized + HashableTrait,
>(
    key: &Q,
) -> impl '_ + FnMut(&RawEntry<E, I>) -> bool
where
    E: EntryFamily<A>,
    KeyAt<E, A>: Borrow<Q>,
{
    move |e| match e.downcast_ref() {
        Some(r) => key == r.key().borrow(),
        None => false,
    }
}

impl<
        E: 'static + ?Sized,
        S: 'static + BuildHasher,
        I: ?Sized + HashableTrait<Hasher = S::Hasher>,
    > AnyMap<E, S, I>
{
    fn hash_key<A: 'static + ?Sized, Q: ?Sized + Hash>(&self, key: &Q) -> u64
    where E: EntryFamily<A> {
        hash_def_key::<_, A, E, S>(&self.hash_state, key)
    }
    fn get_inner<A: 'static + ?Sized, Q: ?Sized>(&self, key: &Q) -> Option<&InnerEntry<E, A>>
    where
        E: EntryFamily<A>,
        KeyAt<E, A>: Borrow<Q>,
        Q: Hash + Eq,
    {
        let hash = self.hash_key(&key);
        let entry = self.raw.get(hash, equivalent_key(key));
        match entry {
            Some(e) => match e.downcast_ref() {
                r @ Some(_) => r,
                // safety: invariant for equivalent key checking downcast succeeds
                None => unsafe { std::hint::unreachable_unchecked() },
            },
            None => None,
        }
    }

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
                None => unsafe { std::hint::unreachable_unchecked() },
            },
            None => None,
        }
    }

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

    /// Reserve storage to fit at least `additional` more entries without reallocating.
    pub fn reserve(&mut self, additional: usize) {
        let hashfn = make_hasher(&self.hash_state);
        self.raw.reserve(additional, hashfn)
    }
    /// Shrink storage to fit the currently used size.
    pub fn shrink_to_fit(&mut self) {
        let hashfn = make_hasher(&self.hash_state);
        self.raw.shrink_to(0, hashfn)
    }
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
    /// Check if the map contains a value for the specified key.
    pub fn contains_key<A: 'static + ?Sized, Q: ?Sized>(&self, k: &Q) -> bool
    where
        E: EntryFamily<A>,
        KeyAt<E, A>: Borrow<Q>,
        Q: Hash + Eq,
    {
        self.get_inner(k).is_some()
    }
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
    /// Note that this differs from [`Self::insert`] where the key is not updated.
    pub fn insert<A: 'static + ?Sized, P: ?Sized>(
        &mut self,
        entry: P,
    ) -> Option<EntryAt<E, A>>
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
            let raw_entry = RawEntry::<E, I>::new(entry);
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
                None => unsafe { std::hint::unreachable_unchecked() },
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
}

impl<E: ?Sized, I: ?Sized + DynClone> Clone for RawEntry<E, I> {
    fn clone(&self) -> Self {
        Self {
            inner: dyn_clone::clone_box(&self.inner),
            _marker: std::marker::PhantomData,
        }
    }
}

impl<E: ?Sized, S: BuildHasher + Clone, I: ?Sized + DynClone> Clone for AnyMap<E, S, I> {
    fn clone(&self) -> Self {
        Self {
            hash_state: self.hash_state.clone(),
            raw: self.raw.clone(),
        }
    }
}
