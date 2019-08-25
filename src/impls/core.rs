use crate::{BitStart, Packable, SubPack, PackableRoot};
use core::mem;

unsafe impl<S: BitStart> Packable<S> for bool {
    type Packed = SubPack<S, bool>;

    const WIDTH: u32 = 1;

    #[inline]
    unsafe fn store(self, p: &mut SubPack<S, Self>) {
        p.set_from_low_bits(self as usize);
    }

    #[inline]
    unsafe fn load(p: &SubPack<S, Self>) -> Self {
        p.get_bits() != 0
    }
}

unsafe impl PackableRoot for bool {}

unsafe impl<'a, T, S: BitStart> Packable<S> for &'a T {
    type Packed = SubPack<S, &'a T>;

    const WIDTH: u32 = usize::leading_zeros(mem::align_of::<T>() - 1);

    #[inline]
    unsafe fn store(self, p: &mut SubPack<S, Self>) {
        p.set_from_high_bits(self as *const T as usize)
    }

    #[inline]
    unsafe fn load(p: &SubPack<S, Self>) -> Self {
        &*(p.get_as_high_bits() as *const T)
    }
}

unsafe impl<'a, T> PackableRoot for &'a T {}
