//! The different types of `Any` for use in a map.
//!
//! This stuff is all based on `std::any`, but goes a little further, with `CloneAny` being a
//! cloneable `Any` and with the `Send` and `Sync` bounds possible on both `Any` and `CloneAny`.

use dyn_clone::{clone_trait_object, DynClone};
use std::any::Any as StdAny;
use std::fmt;

clone_trait_object!(CloneAny);

macro_rules! define {
    (CloneAny) => {
        define!(
            /// A type to emulate dynamic typing.
            ///
            /// Every type with no non-`'static` references implements `Any`.
            CloneAny remainder
        );
    };
    (Any) => {
        define!(
            /// A type to emulate dynamic typing with cloning.
            ///
            /// Every type with no non-`'static` references that implements `Clone` implements `Any`.
            Any remainder
        );
    };
    ($(#[doc = $doc:expr])* $t:ident remainder) => {
        define!(
            $(#[doc = $doc])*
            /// See the [`std::any` documentation](https://doc.rust-lang.org/std/any/index.html) for
            /// more details on `Any` in general.
            ///
            /// This trait is not `std::any::Any` but rather a type extending that for this library’s
            /// purposes so that it can be combined with marker traits like
            /// <code><a class=trait title=core::marker::Send
            /// href=http://doc.rust-lang.org/std/marker/trait.Send.html>Send</a></code> and
            /// <code><a class=trait title=core::marker::Sync
            /// href=http://doc.rust-lang.org/std/marker/trait.Sync.html>Sync</a></code>.
            ///
            $t trait
        );
    };
    ($(#[doc = $doc:expr])* CloneAny trait) => {
        $(#[doc = $doc])*
        /// See also [`Any`](trait.Any.html) for a version without the `Clone` requirement.
        pub trait CloneAny: Any + DynClone { }
        impl<T: StdAny + Clone> CloneAny for T { }
    };
    ($(#[doc = $doc:expr])* Any trait) => {
        $(#[doc = $doc])*
        /// See also [`CloneAny`](trait.CloneAny.html) for a cloneable version of this trait.
        pub trait Any: StdAny {
            /// Upcast to an [`std::any::Any`]
            fn as_any_ref(&self) -> &dyn StdAny;
            /// Upcast to a mut [`std::any::Any`]
            fn as_any_ref_mut(&mut self) -> &mut dyn StdAny;
            /// Upcast to a Box of [`std::any::Any`]
            fn as_any_box(self: Box<Self>) -> Box<dyn StdAny>;
        }
        impl<T: StdAny> Any for T {
            #[inline]
            fn as_any_ref(&self) -> &dyn StdAny { self }
            #[inline]
            fn as_any_ref_mut(&mut self) -> &mut dyn StdAny { self }
            #[inline]
            fn as_any_box(self: Box<Self>) -> Box<dyn StdAny> { self }
        }
    };
}

#[inline]
pub (crate) unsafe fn lateral_ref_unchecked<U: Any + ?Sized, T: Any>(any: &U) -> &T {
    let std_ref = any.as_any_ref();
    &*(std_ref as *const dyn StdAny as *const T)
    //match <dyn StdAny>::downcast_ref::<T>(any.as_any_ref()) {
    //    Some(r) => r,
    //    None => std::hint::unreachable_unchecked(),
    //}
}
#[inline]
pub (crate) unsafe fn lateral_mut_unchecked<U: Any + ?Sized, T: Any>(any: &mut U) -> &mut T {
    let std_ref = any.as_any_ref_mut();
    &mut *(std_ref as *mut dyn StdAny as *mut T)
    //match <dyn StdAny>::downcast_mut::<T>(any.as_any_ref_mut()) {
    //    Some(r) => r,
    //    None => std::hint::unreachable_unchecked(),
    //}
}
#[inline]
pub (crate) unsafe fn lateral_boxcast_unchecked<U: Any + ?Sized, T: Any>(any: Box<U>) -> Box<T> {
    let std_box = any.as_any_box();
    let raw: *mut dyn StdAny = Box::into_raw(std_box);
    Box::from_raw(raw as *mut T)
    //match std_box.downcast() {
    //    Ok(r) => r,
    //    Err(_) => std::hint::unreachable_unchecked(),
    //}
}

#[doc(hidden)]
/// A trait for the conversion of an object into a boxed trait object.
pub trait IntoBox<A: ?Sized>: Any {
    /// Convert self into the appropriate boxed form.
    fn into_box(self) -> Box<A>;
}

macro_rules! implement {
    ($base:ident, $(+ $bounds:ident)*) => {
        impl fmt::Debug for dyn $base $(+ $bounds)* {
            #[inline]
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.pad(stringify!($base $(+ $bounds)*))
            }
        }

        impl<T: $base $(+ $bounds)*> IntoBox<dyn $base $(+ $bounds)*> for T {
            #[inline]
            fn into_box(self) -> Box<dyn $base $(+ $bounds)*> {
                Box::new(self)
            }
        }
    }
}

define!(Any);
implement!(Any,);
implement!(Any, + Send);
implement!(Any, + Sync);
implement!(Any, + Send + Sync);
implement!(CloneAny,);
implement!(CloneAny, + Send);
implement!(CloneAny, + Sync);
implement!(CloneAny, + Send + Sync);

define!(CloneAny);

trait PartialEqSelf {
    fn cmp_self(&self, rhs: &Self) -> bool;
}
impl<T: PartialEq> PartialEqSelf for T {
    fn cmp_self(&self, rhs: &Self) -> bool {
        self == rhs
    }
}
