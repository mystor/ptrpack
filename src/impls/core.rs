use crate::{BitStart, Packable, SubPack, RawPackedBits};
use core::mem;

unsafe impl<S: BitStart> Packable<S> for bool {
    type Packed = SubPack<S, bool>;

    const WIDTH: u32 = 1;

    #[inline]
    unsafe fn store(self, p: &mut RawPackedBits<S, Self>) {
        p.write_low_bits(self as usize);
    }

    #[inline]
    unsafe fn load(p: &RawPackedBits<S, Self>) -> Self {
        p.read_unshifted_bits() != 0
    }
}

unsafe impl<'a, T, S: BitStart> Packable<S> for &'a T {
    type Packed = SubPack<S, &'a T>;

    const WIDTH: u32 = usize::leading_zeros(mem::align_of::<T>() - 1);

    #[inline]
    unsafe fn store(self, p: &mut RawPackedBits<S, Self>) {
        p.write_high_bits(self as *const T as usize)
    }

    #[inline]
    unsafe fn load(p: &RawPackedBits<S, Self>) -> Self {
        &*(p.read_high_bits() as *const T)
    }
}
