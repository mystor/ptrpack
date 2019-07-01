#![no_std]

use core::hash::Hash;
use core::num::NonZeroUsize;
use core::marker::PhantomData;
use core::fmt::Debug;
use core::mem;
use core::ops::{Deref, DerefMut};

mod internal {
    pub trait Sealed {}
    impl Sealed for usize {}
    impl Sealed for core::num::NonZeroUsize {}
}

/// Helper constant value of the width of a pointer in bits.
#[doc(hidden)]
pub const PTR_WIDTH: u32 = usize::leading_zeros(0);

/// Helper method for computing the minimum of two `u32` values.
#[doc(hidden)]
pub const fn const_min(a: u32, b: u32) -> u32 {
    let a_lt_b = (a < b) as u32;
    let a_ge_b = (a >= b) as u32;

    (a * a_lt_b) + (b * a_ge_b)
}

/// Helper method for computing the mask field constant.
#[doc(hidden)]
pub const fn const_mask(before: u32, after: u32) -> usize {
    // 1 if the specified range is non-empty. This is used to zero out the mask
    // if the range is empty, as `wrapping_shr` and `wrapping_shl` won't produce
    // a 0 value if the shift overflows.
    //
    // This would be much easier if rust supported conditionals within constant
    // context, but that is currently unsupported.
    let nonempty = ((before + after) < PTR_WIDTH) as usize;

    // Compute the mask with all bits set except those before the range in
    // question, and those after the range in question. If either `before` or
    // `after` is `PTR_WIDTH`, these shifts will overflow, wrapping around. This
    // case is caught by the multiplication with `nonempty` below.
    let not_before = usize::max_value().wrapping_shr(before);
    let not_after = usize::max_value().wrapping_shl(after);

    (not_before & not_after) * nonempty
}

/// # BitStart
pub trait BitStart {
    const START: u32;
}

pub struct DefaultStart;
impl BitStart for DefaultStart {
    const START: u32 = PTR_WIDTH;
}

/// # Packed
pub unsafe trait Packed<S: BitStart> {
    type Packable: Packable<S>;
}

/// # Packable
pub unsafe trait Packable<S: BitStart> {
    type Packed: Packed<S>;

    const WIDTH: u32;

    #[inline]
    fn pack(&self) -> usize;

    #[inline]
    unsafe fn unpack(packed: usize) -> Self;
}

/// # Pack
#[repr(transparent)]
pub struct Pack<P: Packable<DefaultStart>> {
    value: usize,
    marker: PhantomData<P>,
}

impl<P: Packable<DefaultStart>> Pack<P> {
    
}

/// # Pack
#[repr(transparent)]
pub struct Pack<P, S = DefaultStart>
where
    P: Packable<S>,
    S: BitStart,
{
    value: usize,
    marker: PhantomData<(P, S)>,
}

impl<P> Pack<P, DefaultStart>
where
    P: Packable<DefaultStart>,
{
    /// Construct a packed version of the given type
    pub fn new(p: P) -> Self {
        unimplemented!()
    }
}

impl<P, S> Pack<P, S>
where
    P: Packable<S>,
    S: BitStart,
{
    // --
}

impl<P, S> Deref for Pack<P, S>
where
    P: Packable<S>,
    S: BitStart,
{
    type Target = P::Packed;

    fn deref(&self) -> &Self::Target {
        unsafe { mem::transmute(self) }
    }
}

impl<P, S> DerefMut for Pack<P, S>
where
    P: Packable<S>,
    S: BitStart,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { mem::transmute(self) }
    }
}



/// Storage type used by `Packable`. This is either `usize` or `NonZeroUsize`.
pub trait MaybeNonZeroUsize:
    Copy + Clone + Send + Sync + Ord + PartialOrd + Hash + Eq + PartialEq + Debug + internal::Sealed
{
    unsafe fn new_unchecked(value: usize) -> Self;
    fn get(&self) -> usize;
}

impl MaybeNonZeroUsize for usize {
    unsafe fn new_unchecked(packed: usize) -> Self {
        packed
    }
    fn get(&self) -> usize {
        *self
    }
}

impl MaybeNonZeroUsize for NonZeroUsize {
    unsafe fn new_unchecked(packed: usize) -> Self {
        NonZeroUsize::new_unchecked(packed)
    }
    fn get(&self) -> usize {
        NonZeroUsize::get(*self)
    }
}

/// Helper trait used internally by `Packable`.
pub trait BitRange {
    /// Number of bits before the field.
    const BEFORE: u32;

    /// Number of bits after the field.
    const AFTER: u32;

    /// Mask for bits within the specified range.
    const MASK: usize;
}

/// Compound bit range offset such that the inner `BitRange` is located inside
/// of the outer `BitRange, left-aligned.`
pub struct CompoundBitRange<Outer: BitRange, Inner: BitRange> {
    marker: PhantomData<(Outer, Inner)>
}

impl<Outer: BitRange, Inner: BitRange> CompoundBitRange<Outer, Inner> {
    const BEFORE: u32 = Inner::BEFORE + Outer::BEFORE;

    const AFTER: u32 = Inner::AFTER - Outer::BEFORE;

    const MASK: usize = const_mask(Self::BEFORE, Self::AFTER);
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
