use crate::{detail, Packable, PtrPack};
use core::marker;

const fn const_max(x: u32, y: u32) -> u32 {
    // FIXME: This is gross, but we can't branch.
    let use_x = x > y;
    let x2 = x * (use_x as u32);
    let y2 = y * (!use_x as u32);
    x2 + y2
}

/// Untagged [`Packable`] union of two [`Packable`] types.
///
/// Values of this type are exactly pointer sized, and high bits are used to
/// store significant packed data. Additional data may be used to pack into low
/// bits using [`PtrPack`].
///
/// See also [`TinyEither`] for a tagged `Packable` union.
#[derive(Copy, Clone)]
pub struct TinyUnion<L: Packable, R: Packable> {
    bits: usize,
    _marker: marker::PhantomData<(L, R)>,
}

impl<L: Packable, R: Packable> TinyUnion<L, R> {
    pub fn from_left(left: L) -> Self {
        TinyUnion {
            bits: PtrPack::<(L,)>::new((left,)).to_bits(),
            _marker: marker::PhantomData,
        }
    }

    pub fn from_right(right: R) -> Self {
        TinyUnion {
            bits: PtrPack::<(R,)>::new((right,)).to_bits(),
            _marker: marker::PhantomData,
        }
    }

    pub unsafe fn as_left(self) -> L {
        PtrPack::<(L,)>::from_bits(self.bits).get_0()
        // self.left.get_0()
    }

    pub unsafe fn as_right(self) -> R {
        PtrPack::<(R,)>::from_bits(self.bits).get_0()
        //self.right.get_0()
    }
}

unsafe impl<L: Packable, R: Packable> Packable for TinyUnion<L, R> {
    type BitAlign = detail::HighBits;

    // FIXME: If both `L` and `R` are `NonNullStorage`, we should be able to
    // make this `NonNullStorage`. This may require specialization or
    // higher-order types.
    type Storage = detail::NullableStorage;

    // Whichever of `L` and `R` uses the most bits is the one we need to use.
    const BITS: u32 = const_max(
        <PtrPack<(L,)> as Packable>::BITS,
        <PtrPack<(R,)> as Packable>::BITS,
    );

    unsafe fn from_bits(bits: usize) -> Self {
        TinyUnion { bits, _marker: marker::PhantomData }
    }
    fn to_bits(self) -> usize {
        self.bits
    }
}

/// Basic, unpacked, 2-element enum.
///
/// This type is like [`Result`], but with a different name and intended
/// semantics.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum Either<L, R> {
    Left(L),
    Right(R),
}

/// [`Packable`] Enum of two `Packable` types.
pub struct TinyEither<L: Packable, R: Packable> {
    // FIXME: If `L` uses `NonNullStorage`, then our `TinyUnion` should be able
    // to use it. We don't care about `R`, as even if it's nullable, the value
    // will be non-null due to the bool being `True`.
    value: PtrPack<(TinyUnion<L, R>, bool)>,
}

impl<L: Packable + Copy, R: Packable + Copy> TinyEither<L, R> {
    pub fn new(either: Either<L, R>) -> Self {
        match either {
            Either::Left(left) => Self::from_left(left),
            Either::Right(right) => Self::from_right(right),
        }
    }

    pub fn from_left(left: L) -> Self {
        TinyEither {
            value: PtrPack::new((TinyUnion::from_left(left), false)),
        }
    }

    pub fn from_right(right: R) -> Self {
        TinyEither {
            value: PtrPack::new((TinyUnion::from_right(right), true)),
        }
    }

    pub fn is_left(&self) -> bool {
        !self.is_right()
    }

    pub fn is_right(&self) -> bool {
        self.value.get_1()
    }

    pub fn as_union(self) -> TinyUnion<L, R> {
        self.value.get_0()
    }

    pub unsafe fn as_left_unchecked(self) -> L {
        self.as_union().as_left()
    }

    pub unsafe fn as_right_unchecked(self) -> R {
        self.as_union().as_right()
    }

    pub fn as_left(self) -> Option<L> {
        if self.is_left() {
            Some(unsafe { self.as_left_unchecked() })
        } else {
            None
        }
    }

    pub fn as_right(self) -> Option<R> {
        if self.is_right() {
            Some(unsafe { self.as_right_unchecked() })
        } else {
            None
        }
    }

    pub fn as_either(self) -> Either<L, R> {
        if self.is_left() {
            Either::Left(unsafe { self.as_left_unchecked() })
        } else {
            Either::Right(unsafe { self.as_right_unchecked() })
        }
    }
}

impl<L: Packable + Copy, R: Packable + Copy> Copy for TinyEither<L, R> { }
impl<L: Packable + Copy, R: Packable + Copy> Clone for TinyEither<L, R> {
    fn clone(&self) -> Self {
        *self
    }
}

unsafe impl<L: Packable, R: Packable> Packable for TinyEither<L, R> {
    type BitAlign = <PtrPack<(TinyUnion<L, R>, bool)> as Packable>::BitAlign;

    type Storage = <PtrPack<(TinyUnion<L, R>, bool)> as Packable>::Storage;

    const BITS: u32 = <PtrPack<(TinyUnion<L, R>, bool)> as Packable>::BITS;

    unsafe fn from_bits(bits: usize) -> Self {
        TinyEither {
            value: PtrPack::from_bits(bits),
        }
    }
    fn to_bits(self) -> usize {
        self.value.to_bits()
    }
}
