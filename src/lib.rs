//! This crate provides the [`Map`] type, a safe and convenient store for one value of each type.
#![cfg_attr(feature = "unstable_features", feature(unsize, coerce_unsized))]
#![warn(missing_docs, unused_results)]

mod map;
pub use map::*;
#[cfg(test)]
mod tests;
/// Structures implementing [`EntryFamily`]
pub mod families;
