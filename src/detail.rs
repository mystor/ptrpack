//! Types requried to describe a [`Packable`] type.

use crate::*;
use core::{cmp, hash, mem, ptr};

mod private {
    pub trait Sealed {}

    impl Sealed for super::NonNullStorage {}
    impl Sealed for super::NullableStorage {}

    impl<P> Sealed for super::TupleEltBitRange<P> {}

    impl Sealed for super::LowBits {}
    impl Sealed for super::HighBits {}
}

macro_rules! static_assert {
    ($e:expr, $name:ident) => {
        #[allow(non_upper_case_globals)]
        #[allow(unused)]
        const $name: [(); {
            let cond: bool = $e;
            !cond as usize
        }] = [];
    };
}

/// Pointer width on this platform, in bits.
pub const PTR_WIDTH: u32 = 0usize.count_zeros();

/// Backing storage used by this given type.
///
/// It must be one of [`NonNullStorage`] or [`NullableStorage`], depending on
/// whether the associated value has a valid bit-representation of all `0`s.
pub unsafe trait PointerStorage:
    Copy + cmp::Eq + cmp::Ord + hash::Hash + Send + Sync + private::Sealed
{
    /// Create a `PointerStorage` from some bits.
    ///
    /// These bits must satisfy the nullability constraints of the storage type.
    unsafe fn from_bits(bits: usize) -> Self;

    /// Get the bits stored in this `PointerStorage`.
    fn to_bits(self) -> usize;
}

/// Storage for a [`usize`] which cannot be `0`.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct NonNullStorage(ptr::NonNull<()>);

unsafe impl PointerStorage for NonNullStorage {
    unsafe fn from_bits(bits: usize) -> Self {
        NonNullStorage(ptr::NonNull::new_unchecked(bits as *mut ()))
    }

    fn to_bits(self) -> usize {
        self.0.as_ptr() as usize
    }
}

unsafe impl Sync for NonNullStorage {}
unsafe impl Send for NonNullStorage {}

static_assert!(
    mem::size_of::<NonNullStorage>() == mem::size_of::<usize>(),
    size_of_nonnull_is_usize
);

/// Storage for a [`usize`] value.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct NullableStorage(usize);

unsafe impl PointerStorage for NullableStorage {
    unsafe fn from_bits(bits: usize) -> Self {
        NullableStorage(bits)
    }

    fn to_bits(self) -> usize {
        self.0
    }
}

static_assert!(
    mem::size_of::<NullableStorage>() == mem::size_of::<usize>(),
    size_of_nullable_is_usize
);

/// Internal constant-providing type used by implementations of [`BitAlign`] for
/// extra parameters and constants.
#[doc(hidden)]
pub trait BitRange: private::Sealed {
    const MASK: usize;
    const LOW_SHIFT: u32;
    const HIGH_SHIFT: u32;
}

/// Generate a mask for all bits in the non-inclusive range `low..high`.
const fn range_mask(low: u32, high: u32) -> usize {
    (u128::max_value() << low ^ u128::max_value() << high) as usize
}

static_assert!(range_mask(0, 0) == 0, mask_0_0);
static_assert!(range_mask(3, 10) == 0b1111111000, mask_3_10);
static_assert!(range_mask(32, 32) == 0, mask_32_32);

#[cfg(target_pointer_width = "64")]
static_assert!(range_mask(64, 64) == 0, mask_64_64);
#[cfg(target_pointer_width = "64")]
static_assert!(range_mask(0, 64) == usize::max_value(), mask_0_64);
#[cfg(target_pointer_width = "64")]
static_assert!(range_mask(60, 64) == 0xF000000000000000, mask_60_64);

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
        <Self::Last as Packable>::from_bits(Self::tuple_bits_to_last_bits(tuple_bits))
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
    fn to_tuple_bits(self) -> usize;

    #[doc(hidden)]
    #[inline]
    unsafe fn from_tuple_bits(bits: usize) -> Self;
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
    fn to_tuple_bits(self) -> usize {
        0
    }

    #[doc(hidden)]
    #[inline]
    unsafe fn from_tuple_bits(_: usize) -> Self {
        ()
    }
}

// NOTE: Directly include `tuples.rs` at the end to ensure that rustdoc reads
// and places the generated inherent impls below the manually written ones.
include!("tuples.rs");

// 64-bit platform specific static assertions to ensure computations are correct.
#[cfg(target_pointer_width = "64")]
mod x64_test {
    use super::*;

    static_assert!(PTR_WIDTH == 64, ptr_width_64);

    macro_rules! low_high_check {
        ($name:ident, $t:ty => ($lo:expr, $hi:expr)) => {
            #[allow(unused)]
            #[allow(non_upper_case_globals)]
            const $name: () = {
                const LOW: [(); $lo as usize] = [(); <$t as PackableTuple>::LAST_LOW_BIT as usize];
                const HIGH: [(); $hi as usize] =
                    [(); <$t as PackableTuple>::LAST_HIGH_BIT as usize];
                ()
            };
        };
    }

    low_high_check!(BOOL, (bool,) => (63, 64));
    low_high_check!(BOOL_BOOL, (bool, bool) => (62, 63));

    low_high_check!(REF_U64, (&u64,) => (3, 64));
    low_high_check!(REF_U32, (&u32,) => (2, 64));
    low_high_check!(REF_U16, (&u16,) => (1, 64));
    low_high_check!(REF_U8, (&u8,) => (0, 64));

    #[repr(align(1))]
    struct Align1(());
    #[repr(align(2))]
    struct Align2(());
    #[repr(align(4))]
    struct Align4(());
    #[repr(align(8))]
    struct Align8(());
    #[repr(align(16))]
    struct Align16(());
    #[repr(align(32))]
    struct Align32(());

    low_high_check!(ALIGN_1, (&Align1,) => (0, 64));
    low_high_check!(ALIGN_2, (&Align2,) => (1, 64));
    low_high_check!(ALIGN_4, (&Align4,) => (2, 64));
    low_high_check!(ALIGN_8, (&Align8,) => (3, 64));
    low_high_check!(ALIGN_16, (&Align16,) => (4, 64));
    low_high_check!(ALIGN_32, (&Align32,) => (5, 64));

    low_high_check!(ALIGN_8_BOOL, (&Align8, bool) => (2, 3));
    low_high_check!(ALIGN_8_BOOL_BOOL, (&Align8, bool, bool) => (1, 2));
    low_high_check!(ALIGN_8_BOOL_BOOL_BOOL, (&Align8, bool, bool, bool) => (0, 1));
}
