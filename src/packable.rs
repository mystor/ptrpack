use crate::detail;
use core::mem;

/// Integral value which may leave spare `0` bits when stored in a [`usize`].
///
/// A tuple of `Packable` values may be stored within a single pointer-sized
/// value using [`PtrPack`](crate::PtrPack).
///
/// # Requirements
///
/// 1. It is [`Copy`]-able.
///
///    Non-`Copy` types are currently unsupported due to `Drop/Copy` restrictions.
///
/// 2. It uses fewer bits than would normally fit into a [`usize`].
///
///    * [`bool`] uses only 1 bit, leaving the high 31 (or 63) bits as `0`.
///
///    * [`&i32`] uses only 30 (or 62) bits due to alignment, leaving the low 2
///      bits as `0`.
///
///    * [`*mut i32`], on the other hand, uses the full 32 (or 64) bits, as raw
///      pointers are not guaranteed to be aligned.
///
///      Use the helper [`Aligned`](crate::Aligned) type to promise raw pointer
///      alignment.
///
/// 3. Unused bits in the type's bit-representation are unconditionally `0`
///    bits.
pub unsafe trait Packable: Copy {
    /// Which end of the value significant bits are stored at.
    ///
    /// This type parameter will be either:
    ///
    /// * [`detail::HighBits`] if data is stored in the most-significant bits,
    ///   such as for pointer-like values (`&i32`, `Aligned<*mut i32>`).
    ///
    /// * [`detail::LowBits`] if data is stored in the least-significant bits,
    ///   such as for integer-like values (`bool`, `u32`, `u8`).
    type BitAlign: detail::BitAlign;

    /// Whether or not the "null" bitpattern (all `0` bits) is valid for this
    /// type.
    ///
    /// Controls whether the null-pointer optimization is supported by packed
    /// values containing this type.
    ///
    /// This type parameter will be either:
    ///
    /// * [`detail::NullableStorage`] if the "null" bitpattern is valid.
    ///   (e.g. `u32`, `Aligned<*mut i32>`, `bool`)
    ///
    /// * [`detail::NonNullStorage`] if the "null" bitpattern is not valid.
    ///   (e.g. `&i32`, `Aligned<NonNull<i32>>`)
    type Storage: detail::PointerStorage;

    /// How wide this type's "bits" memory representation is, in bits.
    ///
    /// Bits outside of the range described by [`Packable::BitAlign`] and
    /// [`Packable::Storage`] mut be `0` in this type's binary representation.
    const BITS: u32;

    /// Cast the binary representation value from `bits` to this type.
    ///
    /// This method must round-trip correctly with the [`Packable::to_bits`] method.
    unsafe fn from_bits(bits: usize) -> Self;

    /// Cast the binary representation value of this type into a `usize`.
    ///
    /// This method must round-trip correctly with the [`Packable::from_bits`]
    /// method.
    fn to_bits(self) -> usize;
}

unsafe impl Packable for () {
    type BitAlign = detail::HighBits;
    type Storage = detail::NullableStorage;

    const BITS: u32 = 0;

    unsafe fn from_bits(_: usize) -> Self {
        ()
    }
    fn to_bits(self) -> usize {
        0
    }
}

unsafe impl Packable for bool {
    type BitAlign = detail::LowBits;
    type Storage = detail::NullableStorage;

    const BITS: u32 = 1;

    unsafe fn from_bits(bits: usize) -> Self {
        bits != 0
    }
    fn to_bits(self) -> usize {
        self as usize
    }
}

unsafe impl<T> Packable for &T {
    type BitAlign = detail::HighBits;
    type Storage = detail::NonNullStorage;

    const BITS: u32 = detail::PTR_WIDTH - mem::align_of::<T>().trailing_zeros();

    unsafe fn from_bits(bits: usize) -> Self {
        mem::transmute::<usize, Self>(bits)
    }
    fn to_bits(self) -> usize {
        unsafe { mem::transmute::<Self, usize>(self) }
    }
}

// We can store `Option<T>` in some cases, thanks to the null-pointer optimization.
// This is only possible for types which are marked as `NonNull`.
unsafe impl<T> Packable for Option<T>
where
    T: Packable<Storage=detail::NonNullStorage>,
{
    type BitAlign = T::BitAlign;
    type Storage = detail::NullableStorage;

    const BITS: u32 = T::BITS;

    unsafe fn from_bits(bits: usize) -> Self {
        if bits == 0 { None } else { Some(T::from_bits(bits)) }
    }
    fn to_bits(self) -> usize {
        match self {
            Some(t) => T::to_bits(t),
            None => 0,
        }
    }
}
