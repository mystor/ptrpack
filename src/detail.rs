//! Types requried to describe a [`Packable`] type.

use crate::*;
use core::{cmp, hash, ptr};

mod private {
    pub trait Sealed {}

    impl Sealed for super::NonNullStorage {}
    impl Sealed for super::NullableStorage {}

    impl<P> Sealed for super::TupleEltBitRange<P> {}

    impl Sealed for super::LowBits {}
    impl Sealed for super::HighBits {}
}

/// Pointer width on this platform, in bits.
pub const PTR_WIDTH: u32 = 0usize.count_zeros();

/// Backing storage used by this given type.
///
/// It must be one of [`NonNullStorage`] or [`NullableStorage`], depending on
/// whether the associated value has a valid bit-representation of all `0`s.
pub unsafe trait PointerStorage:
    Copy + cmp::Eq + cmp::Ord + hash::Hash + private::Sealed
{
    /// Create a `PointerStorage` from some bits.
    ///
    /// These bits must satisfy the nullability constraints of the storage type.
    unsafe fn from_bits_unchecked(bits: usize) -> Self;

    /// Get the bits stored in this `PointerStorage`.
    fn to_bits(self) -> usize;
}

/// Storage for a [`usize`] which cannot be `0`.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct NonNullStorage(ptr::NonNull<()>);

unsafe impl PointerStorage for NonNullStorage {
    unsafe fn from_bits_unchecked(bits: usize) -> Self {
        NonNullStorage(ptr::NonNull::new_unchecked(bits as *mut ()))
    }

    fn to_bits(self) -> usize {
        self.0.as_ptr() as usize
    }
}

unsafe impl Sync for NonNullStorage {}
unsafe impl Send for NonNullStorage {}

/// Storage for a [`usize`] value.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct NullableStorage(usize);

unsafe impl PointerStorage for NullableStorage {
    unsafe fn from_bits_unchecked(bits: usize) -> Self {
        NullableStorage(bits)
    }

    fn to_bits(self) -> usize {
        self.0
    }
}

/// Internal constant-providing type used by implementations of [`BitAlign`] for
/// extra parameters and constants.
#[doc(hidden)]
pub trait BitRange: private::Sealed {
    const MASK: usize;
    const LOW_SHIFT: u32;
    const HIGH_SHIFT: u32;
}

/// Mask for all bits up to, but not including, `high`.
const fn low_mask(high: u32) -> usize {
    (1usize << high) - 1
}

/// Mask with all bits set starting at `low`, up to, but not including, `high`.
const fn range_mask(low: u32, high: u32) -> usize {
    low_mask(high) ^ low_mask(low)
}

/// [`BitRange`] for the last element in the tuple `P`.
#[doc(hidden)]
pub struct TupleEltBitRange<P>(P);

impl<P: PackableTuple> BitRange for TupleEltBitRange<P> {
    const MASK: usize = range_mask(P::LAST_LOW_BIT, P::LAST_HIGH_BIT);
    const LOW_SHIFT: u32 = P::LAST_LOW_BIT;
    const HIGH_SHIFT: u32 = PTR_WIDTH - P::LAST_HIGH_BIT;
}

/// Whether the type stores its data in the least significant bits (the
/// [`LowBits`]), or most significant bits (the [`HighBits`]).
///
/// This type's operations are very quick, usually just a shift and/or mask, and
/// care is taken to ensure more expensive constant computation is done at
/// compile time.
pub trait BitAlign: private::Sealed {
    /// Extract data from a subrange of `bits`.
    ///
    /// The `bits` parameter is unmasked. Extract the value from the bits
    /// specified by `R`, putting it in the correct bits of the resulting
    /// `usize`. Bits outside of the range are cleared to `0`.
    #[inline]
    fn from_bit_range<R: BitRange>(bits: usize) -> usize;

    /// Store `bits` into the subrange `R` of a `usize`.
    ///
    /// The resulting value must have `0` bits in all positions not included in
    /// the range `R`.
    #[inline]
    fn to_bit_range<R: BitRange>(bits: usize) -> usize;
}

/// An implementation of [`BitAlign`] for values which only use the low `BITS`
/// bits of their representation for data. All other bits must be `0`.
pub struct LowBits;
impl BitAlign for LowBits {
    #[inline]
    fn from_bit_range<R: BitRange>(bits: usize) -> usize {
        ((bits & R::MASK) >> R::LOW_SHIFT)
    }

    #[inline]
    fn to_bit_range<R: BitRange>(bits: usize) -> usize {
        bits << R::LOW_SHIFT
    }
}

/// An implementation of [`BitAlign`] for values which only use the high `BITS`
/// bits of their representation for data. All other bits must be `0`.
pub struct HighBits;
impl BitAlign for HighBits {
    #[inline]
    fn from_bit_range<R: BitRange>(bits: usize) -> usize {
        ((bits & R::MASK) << R::HIGH_SHIFT)
    }

    #[inline]
    fn to_bit_range<R: BitRange>(bits: usize) -> usize {
        bits >> R::HIGH_SHIFT
    }
}

/// A tuple which valid as a type parameter to [`PtrPack`].
///
/// This trait is implemented for all tuples (of sizes 0-16) containing
/// exclusively [`Packable`] elements.
///
/// See the [`PtrPack`] documentation for details.
pub unsafe trait PackableTuple: Copy {
    // Are packed values of this type nullable?
    // detail::{Nullable,NonNull}Storage depending.
    #[doc(hidden)]
    type Storage: detail::PointerStorage;

    // Type of the last element of this tuple, and which bits of the pointer
    // will be used to store it. Used to implement bit-twiddling.
    #[doc(hidden)]
    type Last: Packable;
    #[doc(hidden)]
    const LAST_HIGH_BIT: u32;
    #[doc(hidden)]
    const LAST_LOW_BIT: u32;
    #[doc(hidden)]
    type LastBitRange: detail::BitRange;

    // bit-twiddling helper methods
    #[doc(hidden)]
    #[inline]
    fn tuple_bits_to_last_bits(tuple_bits: usize) -> usize {
        <Self::Last as Packable>::BitAlign::from_bit_range::<Self::LastBitRange>(tuple_bits)
    }

    #[doc(hidden)]
    #[inline]
    unsafe fn tuple_bits_to_last(tuple_bits: usize) -> Self::Last {
        <Self::Last as Packable>::from_bits_unchecked(Self::tuple_bits_to_last_bits(tuple_bits))
    }

    #[doc(hidden)]
    #[inline]
    fn last_bits_to_tuple_bits(last_bits: usize) -> usize {
        <Self::Last as Packable>::BitAlign::to_bit_range::<Self::LastBitRange>(last_bits)
    }

    #[doc(hidden)]
    #[inline]
    fn last_to_tuple_bits(last: Self::Last) -> usize {
        Self::last_bits_to_tuple_bits(<Self::Last as Packable>::to_bits(last))
    }

    #[doc(hidden)]
    #[inline]
    fn update_tuple_bits_with_last(tuple_bits: usize, last: Self::Last) -> usize {
        let mask = Self::LastBitRange::MASK;
        (tuple_bits & !mask) | Self::last_to_tuple_bits(last)
    }

    #[doc(hidden)]
    #[inline]
    fn tuple_to_tuple_bits(self) -> usize;

    #[doc(hidden)]
    #[inline]
    unsafe fn tuple_bits_to_tuple(bits: usize) -> Self;
}

// FIXME: Is it even worth supporting packing the empty tuple?
unsafe impl PackableTuple for () {
    #[doc(hidden)]
    type Storage = detail::NullableStorage;

    #[doc(hidden)]
    type Last = ();

    #[doc(hidden)]
    const LAST_HIGH_BIT: u32 = detail::PTR_WIDTH;
    #[doc(hidden)]
    const LAST_LOW_BIT: u32 = detail::PTR_WIDTH;

    #[doc(hidden)]
    type LastBitRange = detail::TupleEltBitRange<Self>;

    #[doc(hidden)]
    #[inline]
    fn tuple_to_tuple_bits(self) -> usize {
        0
    }

    #[doc(hidden)]
    #[inline]
    unsafe fn tuple_bits_to_tuple(_: usize) -> Self {
        ()
    }
}

// NOTE: Directly include `tuples.rs` at the end to ensure that rustdoc reads
// and places the generated inherent impls below the manually written ones.
include!("tuples.rs");
