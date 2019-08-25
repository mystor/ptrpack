#![recursion_limit="128"]

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "alloc")]
extern crate alloc;

use core::fmt;
use core::marker::PhantomData;
use core::mem::{self, ManuallyDrop};
use core::ops::{Deref, DerefMut};
use core::cmp;

pub mod impls;

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

/// The default initial bit offset for a packed value.
pub struct DefaultStart;
impl BitStart for DefaultStart {
    const START: u32 = PTR_WIDTH;
}

/// The next bit offset to use after a given value.
pub struct NextStart<S, P> {
    _marker: PhantomData<(S, P)>,
}
impl<S, P> BitStart for NextStart<S, P>
where
    S: BitStart,
    P: Packable<S>,
{
    const START: u32 = S::START - P::WIDTH;
}

pub struct UnionStart<A, B> {
    _marker: PhantomData<(A, B)>,
}
impl<A, B> BitStart for UnionStart<A, B>
where
    A: BitStart,
    B: BitStart,
{
    const START: u32 = const_min(A::START, B::START);
}

/// # Packable
pub unsafe trait Packable<S: BitStart>: Sized {
    type Packed;

    /// Number of bits required to represent this value.
    const WIDTH: u32;

    /// Directly store the bits for this value into the given `SubPack`.
    unsafe fn store(self, p: &mut SubPack<S, Self>);

    /// Directly read the bits for this value from the given `SubPack`.
    unsafe fn load(p: &SubPack<S, Self>) -> Self;
}

/// Types which may be packed as the root type within a [`Pack`].
///
/// All types implementing [`Packable`] should also implement this trait. This
/// separate implementation is needed to avoid infinite recursion when
/// evaluating trait requirements for types stored in a `Pack`.
pub unsafe trait PackableRoot : Packable<DefaultStart> { }

/// # Pack
#[repr(transparent)]
pub struct Pack<R> {
    value: usize,
    _marker: PhantomData<R>,
}

impl<R: PackableRoot> Pack<R> {
    pub fn new(x: R) -> Self {
        let mut pack = <ManuallyDrop<Self>>::new(Pack {
            value: 0,
            _marker: PhantomData,
        });

        unsafe { x.store(pack.as_inner_pack_mut()) }

        ManuallyDrop::into_inner(pack)
    }

    pub fn into_inner(self) -> R {
        let this = ManuallyDrop::new(self);
        unsafe { R::load(this.as_inner_pack()) }
    }

    pub fn get(&self) -> R
    where
        R: Copy,
    {
        self.as_inner_pack().get()
    }

    /// Directly read the stored bits.
    pub fn get_bits(&self) -> usize {
        self.value
    }

    /// Directly write the stored bits.
    pub unsafe fn set_bits(&mut self, bits: usize) {
        self.value = bits;
    }

    /// Inner helper method to downcast to `SubPack` for utility methods.
    fn as_inner_pack(&self) -> &SubPack<DefaultStart, R> {
        unsafe { mem::transmute(self) }
    }

    /// Inner helper method to downcast to `SubPack` for utility methods.
    fn as_inner_pack_mut(&mut self) -> &mut SubPack<DefaultStart, R> {
        unsafe { mem::transmute(self) }
    }
}

impl<R: PackableRoot> Deref for Pack<R> {
    type Target = R::Packed;

    fn deref(&self) -> &Self::Target {
        unsafe { mem::transmute(self) }
    }
}

impl<R: PackableRoot> DerefMut for Pack<R> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { mem::transmute(self) }
    }
}

impl<R: PackableRoot> fmt::Debug for Pack<R> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_tuple("Pack").field(&self.get_bits()).finish()
    }
}

/// # Inner Pack
pub struct SubPack<S, P> {
    _marker: PhantomData<(S, P)>,
}

impl<S, P> SubPack<S, P>
where
    S: BitStart,
    P: Packable<S>,
{
    pub const BEFORE: u32 = PTR_WIDTH - S::START;
    pub const AFTER: u32 = S::START - P::WIDTH;
    pub const MASK: usize = const_mask(Self::BEFORE, Self::AFTER);
    pub const CLEAR_MASK: usize = !Self::MASK;

    /// Read the value packed within this `SubPack`.
    pub fn get(&self) -> P
    where
        P: Copy,
    {
        unsafe { self.read_raw() }
    }

    pub fn replace(&mut self, new: P) -> P {
        unsafe {
            let prev = ManuallyDrop::new(self.read_raw());
            self.write_raw(new);
            ManuallyDrop::into_inner(prev)
        }
    }

    pub unsafe fn read_raw(&self) -> P {
        P::load(self)
    }

    pub unsafe fn write_raw(&mut self, new: P) {
        P::store(new, self)
    }

    /// Masked read of the relevant bits from the underlying type.
    pub fn get_bits(&self) -> usize {
        let bits_ptr = self as *const _ as *const usize;
        unsafe { *bits_ptr & Self::MASK }
    }

    /// Masked read of the relevant bits, shifted into the high bits of the
    /// output, like an integer.
    pub fn get_as_high_bits(&self) -> usize {
        self.get_bits().wrapping_shl(Self::BEFORE)
    }

    /// Masked read of the relevant bits, shifted into the low bits of the
    /// output, like a pointer.
    pub fn get_as_low_bits(&self) -> usize {
        self.get_bits().wrapping_shr(Self::AFTER)
    }

    /// Unsafely clears the bits corresponding to `P` in the inner `Pack<R>`,
    /// and sets them to the provided bits.
    pub unsafe fn set_bits(&mut self, bits: usize) {
        let bits_ptr = self as *mut _ as *mut usize;
        let cleared = *bits_ptr & Self::CLEAR_MASK;
        *bits_ptr = cleared | bits;
    }

    /// Like `set_bits`, but the bits are shifted into place first.
    pub unsafe fn set_from_high_bits(&mut self, bits: usize) {
        self.set_bits(bits.wrapping_shr(Self::BEFORE));
    }

    /// Like `get_bits`, but the bits are shifted into place first.
    pub unsafe fn set_from_low_bits(&mut self, bits: usize) {
        self.set_bits(bits.wrapping_shl(Self::AFTER));
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
        f.debug_tuple("SubPack").field(&self.get_bits()).finish()
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
