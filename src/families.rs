use crate::{EntryFamily, HashEntry};
use std::ops::Deref;
use std::ops::DerefMut;

/// Used as first argument to [`Map`] so that to each type, exactly one value of that type is associated.
///
/// [`Map`]: crate::Map
pub struct Singleton;
impl<A> EntryFamily<A> for Singleton {
    type Result = Some<A>;
}

/// Newtype wrapper around an arbitrary value that serves as Entry for [`Singleton`].
#[repr(transparent)]
#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub struct Some<A> {
    /// Direct access to the wrapped value
    pub some: A,
}

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

impl<A> Deref for Some<A> {
    type Target = A;
    fn deref(&self) -> &Self::Target {
        &self.some
    }
}

impl<A> DerefMut for Some<A> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.some
    }
}
