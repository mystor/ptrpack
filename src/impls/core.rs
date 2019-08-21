use crate::{BitStart, DefaultStart, NextStart, Packable, SubPack};
use core::mem;

unsafe impl<R, S> Packable<R, S> for bool
where
    R: Packable<R, DefaultStart>,
    S: BitStart,
{
    type Packed = SubPack<R, S, bool>;

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

unsafe impl<'a, R, S, T> Packable<R, S> for &'a T
where
    R: Packable<R, DefaultStart>,
    S: BitStart,
{
    type Packed = SubPack<R, S, &'a T>;

    const WIDTH: u32 = usize::leading_zeros(mem::align_of::<T>());

    #[inline]
    unsafe fn store(self, p: &mut SubPack<R, S, Self>) {
        p.set_from_high_bits(self as *const T as usize)
    }

    #[inline]
    unsafe fn load(p: &SubPack<R, S, Self>) -> Self {
        &*(p.get_as_high_bits() as *const T)
    }
}

#[repr(transparent)]
pub struct PackedTuple<R, S, T> {
    inner: SubPack<R, S, T>,
}

impl<R, S, T, U> PackedTuple<R, S, (T, U)>
where
    R: Packable<R, DefaultStart>,
    S: BitStart,
    T: Packable<R, S>,
    U: Packable<R, NextStart<R, S, T>>,
{
    pub fn get_0(&self) -> &T::Packed {
        unsafe { mem::transmute(self) }
    }
    pub fn get_0_mut(&mut self) -> &mut T::Packed {
        unsafe { mem::transmute(self) }
    }

    pub fn get_1(&self) -> &U::Packed {
        unsafe { mem::transmute(self) }
    }
    pub fn get_1_mut(&mut self) -> &mut U::Packed {
        unsafe { mem::transmute(self) }
    }
}
