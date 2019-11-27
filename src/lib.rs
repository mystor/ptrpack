#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "alloc")]
extern crate alloc;

use core::cmp;
use core::fmt;
use core::marker::PhantomData;
use core::mem::{self, ManuallyDrop};
use core::ops::{Deref, DerefMut};

use bitstart::{BitStart, DefaultStart};

pub use ptrpack_macros::Packable;

pub mod impls;
pub mod bitstart;

/// Helper constant value of the width of a pointer in bits.
const PTR_WIDTH: u32 = usize::leading_zeros(0);

/// Helper method for computing the mask field constant.
const fn const_mask(before: u32, after: u32) -> usize {
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

/// # Packable
pub unsafe trait Packable<S: BitStart>: Sized {
    type Packed;

    /// Number of bits required to represent this value.
    const WIDTH: u32;

    /// Directly store the bits for this value into the given `SubPack`.
    unsafe fn store(self, p: &mut RawPackedBits<S, Self>);

    /// Directly read the bits for this value from the given `SubPack`.
    unsafe fn load(p: &RawPackedBits<S, Self>) -> Self;
}

/// # Pack
#[repr(transparent)]
pub struct Pack<P> {
    bits: usize,
    _marker: PhantomData<P>,
}

impl<P: Packable<DefaultStart>> Pack<P> {
    pub fn new(val: P) -> Self {
        let mut bits = 0usize;
        unsafe {
            P::store(val, RawPackedBits::for_bits_mut(&mut bits));
        }
        Pack { bits, _marker: PhantomData }
    }

    pub fn into_inner(self) -> P {
        let bits = self.bits;
        mem::forget(self);
        unsafe { P::load(RawPackedBits::for_bits(&bits)) }
    }
}

impl<P: Packable<DefaultStart>> Deref for Pack<P> {
    type Target = P::Packed;

    fn deref(&self) -> &Self::Target {
        unsafe { mem::transmute(self) }
    }
}

impl<P: Packable<DefaultStart>> DerefMut for Pack<P> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { mem::transmute(self) }
    }
}

impl<P: Packable<DefaultStart>> fmt::Debug for Pack<P> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_tuple("Pack").field(&self.bits).finish()
    }
}

/// A raw reference to a slice of bits corresponding to a packed instance of
/// `P`. This type is used by implementations of [`Packable`] to read and write
/// bit subranges.
///
/// The pointer `&[mut] RawPackedBits<S, P>` must always point to a `usize`
/// containing the target unmasked bits.
pub struct RawPackedBits<S, P> {
    _marker: PhantomData<(S, P)>,
}

impl<S, P> RawPackedBits<S, P>
where
    S: BitStart,
    P: Packable<S>,
{
    /// Number of unused most signifigant bits (high bits).
    pub const BEFORE: u32 = PTR_WIDTH - S::START;
    /// Number of unused least signifigant bits (low bits).
    pub const AFTER: u32 = S::START - P::WIDTH;
    /// Mask value with a `1` for each bit in the range to be considered.
    pub const MASK: usize = const_mask(Self::BEFORE, Self::AFTER);
    /// Inverted version of `MASK` used to clear the specified range.
    pub const CLEAR_MASK: usize = !Self::MASK;

    /// View the bits in `usize` through a `RawPackedBits`.
    pub unsafe fn for_bits(bits: &usize) -> &Self {
        mem::transmute(bits)
    }

    /// Mutably view the bits in `usize` through a `RawPackedBits`.
    pub unsafe fn for_bits_mut(bits: &mut usize) -> &mut Self {
        mem::transmute(bits)
    }

    /// Read masked, but unshifted, bits for this value.
    ///
    /// See also [`RawPackedBits::read_high_bits`] and [`RawPackedBits::read_low_bits`].
    pub fn read_unshifted_bits(&self) -> usize {
        let all_bits = unsafe { *(self as *const Self as *const usize) };
        all_bits & Self::MASK
    }

    /// Read masked bits, shifted into the most significant bits.
    ///
    /// Used for pointer-like values with unused "low" bits.
    pub fn read_high_bits(&self) -> usize {
        self.read_unshifted_bits().wrapping_shl(Self::BEFORE)
    }

    /// Read masked bits, shifted into the least significant bits.
    ///
    /// Used for integer-like values with unused "high" bits.
    pub fn read_low_bits(&self) -> usize {
        self.read_unshifted_bits().wrapping_shr(Self::AFTER)
    }

    /// Write new pre-shifted bits for this value.
    ///
    /// See also [`RawPackedBits::write_high_bits`] and [`RawPackedBits::write_low_bits`].
    ///
    /// # Preconditions
    ///
    /// `bits` must be correctly shifted into the specified bitrange, and no
    /// bits outside of the range may be set.
    pub unsafe fn write_unshifted_bits(&mut self, bits: usize) {
        let all_bits = self as *mut Self as *mut usize;
        *all_bits = (*all_bits & Self::CLEAR_MASK) | bits;
    }

    /// Write new bits for this value from the high bits of `bits`.
    ///
    /// Used for pointer-like values with unused "low" bits.
    ///
    /// # Preconditions
    ///
    /// Only the least signifigant `P::WIDTH` bits of `bits` may be set.
    pub unsafe fn write_high_bits(&mut self, bits: usize) {
        self.write_unshifted_bits(bits.wrapping_shr(Self::BEFORE));
    }

    /// Write new bits for this value from the low bits of `bits`.
    ///
    /// Used for integer-like values with unused "high" bits.
    ///
    /// # Preconditions
    ///
    /// Only the least signifigant `P::WIDTH` bits of `bits` may be set.
    pub unsafe fn write_low_bits(&mut self, bits: usize) {
        self.write_unshifted_bits(bits.wrapping_shl(Self::AFTER));
    }

    /// Load value from a subfield.
    pub unsafe fn read_field<S_, P_>(&self) -> P_
    where
        S_: BitStart,
        P_: Packable<S_>,
    {
        P_::load(self.as_field::<S_, P_>())
    }

    /// Store the value of a subfield. Any existing stored value will be
    /// clobbered without invoking `Drop`.
    pub unsafe fn write_field<S_, P_>(&mut self, value: P_)
    where
        S_: BitStart,
        P_: Packable<S_>,
    {
        P_::store(value, self.as_field_mut::<S_, P_>());
    }

    /// Get a `RawPackedBits` for a subrange or subfield of this type.
    pub unsafe fn as_field<S_, P_>(&self) -> &RawPackedBits<S_, P_>
    where
        S_: BitStart,
        P_: Packable<S_>,
    {
        // XXX: Assert that we're a valid subrange. Should be a static assertion.
        assert!(<RawPackedBits<S, P>>::BEFORE <= <RawPackedBits<S_, P_>>::BEFORE,
        "Must cast to a subrange");
        assert!(<RawPackedBits<S, P>>::AFTER <= <RawPackedBits<S_, P_>>::AFTER, "Must cast to a subrange");
        mem::transmute(self)
    }

    /// Get a `RawPackedBits` for a subrange or subfield of this type.
    pub unsafe fn as_field_mut<S_, P_>(&mut self) -> &mut RawPackedBits<S_, P_>
    where
        S_: BitStart,
        P_: Packable<S_>,
    {
        // XXX: Assert that we're a valid subrange. Should be a static assertion.
        assert!(<RawPackedBits<S, P>>::BEFORE <= <RawPackedBits<S_, P_>>::BEFORE, "Must cast to a subrange");
        assert!(<RawPackedBits<S, P>>::AFTER <= <RawPackedBits<S_, P_>>::AFTER, "Must cast to a subrange");
        mem::transmute(self)
    }
}

/// # Inner Pack
pub struct SubPack<S, P> {
    __raw: RawPackedBits<S, P>,
}

impl<S, P> SubPack<S, P>
where
    S: BitStart,
    P: Packable<S>,
{
    /// Read a copy of the packed value.
    pub fn get(&self) -> P
    where
        P: Copy,
    {
        unsafe { P::load(&self.__raw) }
    }

    /// Set the packed value, dropping the previous value.
    pub fn set(&mut self, new: P) {
        self.replace(new);
    }

    /// Replace the packed value, returning the previous value.
    pub fn replace(&mut self, new: P) -> P {
        unsafe {
            let prev = ManuallyDrop::new(P::load(&self.__raw));
            P::store(new, &mut self.__raw);
            ManuallyDrop::into_inner(prev)
        }
    }

    /// Cast the reference down to a field.
    ///
    /// This method is not intended for use outside of impls.
    pub unsafe fn as_field<S2, T>(&self) -> &SubPack<S2, T>
    where
        S2: BitStart,
        T: Packable<S2>,
    {
        mem::transmute(self)
    }

    /// Cast the reference down to a field.
    ///
    /// This method is not intended for use outside of impls.
    pub unsafe fn as_field_mut<S2, T>(&mut self) -> &mut SubPack<S2, T>
    where
        S2: BitStart,
        T: Packable<S2>,
    {
        mem::transmute(self)
    }

    pub fn as_packed(&self) -> &P::Packed {
        unsafe { mem::transmute(self) }
    }

    pub fn as_packed_mut(&mut self) -> &mut P::Packed {
        unsafe { mem::transmute(self) }
    }
}

impl<S, P> fmt::Debug for SubPack<S, P>
where
    S: BitStart,
    P: Packable<S>,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_tuple("SubPack").field(&()).finish()
        // f.debug_tuple("SubPack").field(&self.get_bits()).finish()
    }
}

impl<S, P> cmp::PartialEq for SubPack<S, P>
where
    S: BitStart,
    P: Packable<S> + Copy + cmp::PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.get().eq(&other.get())
    }
}

impl<S, P> cmp::PartialEq<P> for SubPack<S, P>
where
    S: BitStart,
    P: Packable<S> + Copy + cmp::PartialEq,
{
    fn eq(&self, other: &P) -> bool {
        self.get().eq(other)
    }
}

impl<S, P> cmp::Eq for SubPack<S, P>
where
    S: BitStart,
    P: Packable<S> + Copy + cmp::Eq,
{
}

impl<S, P> cmp::PartialOrd for SubPack<S, P>
where
    S: BitStart,
    P: Packable<S> + Copy + cmp::PartialOrd,
{
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        self.get().partial_cmp(&other.get())
    }
}

impl<S, P> cmp::PartialOrd<P> for SubPack<S, P>
where
    S: BitStart,
    P: Packable<S> + Copy + cmp::PartialOrd<P>,
{
    fn partial_cmp(&self, other: &P) -> Option<cmp::Ordering> {
        self.get().partial_cmp(other)
    }
}

impl<S, P> cmp::Ord for SubPack<S, P>
where
    S: BitStart,
    P: Packable<S> + Copy + cmp::Ord,
{
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.get().cmp(&other.get())
    }
}
