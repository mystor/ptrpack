//! The [`BitStart`] trait, and its implementations.
//!
//! It is usually unnecessary to use the types defined in this module directly.

use crate::{Packable, PTR_WIDTH};

mod sealed {
    pub trait Sealed {}
}

/// Where in the packed value to start reading bits. Types implementing this
/// trait are generally obtained using [`NextStart`] and [`UnionStart`].
pub trait BitStart: sealed::Sealed {
    /// The most significant bit of a range.
    ///
    /// The `START` value of the most significant bit of a `usize` is the
    /// pointer width on the target platform, usually either `32` or `64`.
    ///
    /// The least significant bit has an `START` of `1`. A value of `0` marks
    /// the start of the empty range of bits at the least significant end of the
    /// usize.
    const START: u32;
}

/// The initial starting point used when creating a [`Pack`](`crate::Pack`).
pub struct DefaultStart(());
impl BitStart for DefaultStart {
    const START: u32 = PTR_WIDTH;
}
impl sealed::Sealed for DefaultStart {}

/// Advance bitstart `S` over a value `P`.
pub struct NextStart<S, P>(S, P);
impl<S, P> BitStart for NextStart<S, P>
where
    S: BitStart,
    P: Packable<S>,
{
    const START: u32 = S::START - P::WIDTH;
}
impl<S, P> sealed::Sealed for NextStart<S, P> {}

const fn const_min(a: u32, b: u32) -> u32 {
    [a, b][(b < a) as usize]
}

/// Select the more advanced of the bitstarts `A` and `B`.
pub struct UnionStart<A, B>(A, B);
impl<A, B> BitStart for UnionStart<A, B>
where
    A: BitStart,
    B: BitStart,
{
    const START: u32 = const_min(A::START, B::START);
}
impl<A, B> sealed::Sealed for UnionStart<A, B> {}
