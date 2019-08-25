use crate::{BitStart, NextStart, Packable, SubPack, PackableRoot};
use super::PackedCopy;
use core::mem;

unsafe impl<R, S> Packable<R, S> for bool
where
    R: PackableRoot,
    S: BitStart,
{
    type Packed = PackedCopy<R, S, bool>;

    const WIDTH: u32 = 1;

    #[inline]
    unsafe fn store(self, p: &mut SubPack<R, S, Self>) {
        p.set_from_low_bits(self as usize);
    }

    #[inline]
    unsafe fn load(p: &SubPack<R, S, Self>) -> Self {
        p.get_bits() != 0
    }
}

unsafe impl PackableRoot for bool {}

unsafe impl<'a, R, S, T> Packable<R, S> for &'a T
where
    R: PackableRoot,
    S: BitStart,
{
    type Packed = PackedCopy<R, S, &'a T>;

    const WIDTH: u32 = usize::leading_zeros(mem::align_of::<T>() - 1);

    #[inline]
    unsafe fn store(self, p: &mut SubPack<R, S, Self>) {
        p.set_from_high_bits(self as *const T as usize)
    }

    #[inline]
    unsafe fn load(p: &SubPack<R, S, Self>) -> Self {
        &*(p.get_as_high_bits() as *const T)
    }
}

unsafe impl<'a, T> PackableRoot for &'a T {}
