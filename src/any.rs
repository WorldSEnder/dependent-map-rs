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
        pub trait Any: StdAny { }
        impl<T: StdAny> Any for T { }
    };
}

#[allow(missing_docs)] // Bogus warning (it’s not public outside the crate), ☹
pub trait UncheckedAnyExt: Any {
    unsafe fn downcast_ref_unchecked<T: Any>(&self) -> &T;
    unsafe fn downcast_mut_unchecked<T: Any>(&mut self) -> &mut T;
    unsafe fn downcast_unchecked<T: Any>(self: Box<Self>) -> Box<T>;
}

#[doc(hidden)]
/// A trait for the conversion of an object into a boxed trait object.
pub trait IntoBox<A: ?Sized + UncheckedAnyExt>: Any {
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

        impl UncheckedAnyExt for dyn $base $(+ $bounds)* {
            #[inline]
            unsafe fn downcast_ref_unchecked<T: 'static>(&self) -> &T {
                &*(self as *const Self as *const T)
            }

            #[inline]
            unsafe fn downcast_mut_unchecked<T: 'static>(&mut self) -> &mut T {
                &mut *(self as *mut Self as *mut T)
            }

            #[inline]
            unsafe fn downcast_unchecked<T: 'static>(self: Box<Self>) -> Box<T> {
                Box::from_raw(Box::into_raw(self) as *mut T)
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
