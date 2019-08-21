//! Helper types and impls only used if the `alloc` feature is enabled.

use crate::{BitStart, DefaultStart, Packable, SubPack};
use alloc::boxed::Box;
use core::mem;
use core::ops::{Deref, DerefMut};

#[repr(transparent)]
pub struct PackedBox<R, S, T> {
    inner: SubPack<R, S, Box<T>>,
}

impl<R, S, T> PackedBox<R, S, T>
where
    R: Packable<R, DefaultStart>,
    S: BitStart,
{
    /// Get the `Box<T>` value as `&T`
    pub fn as_ref(&self) -> &T {
        unsafe { &*(self.inner.get_as_high_bits() as *const T) }
    }

    /// Get the `Box<T>` value as `&mut T`
    pub fn as_mut(&mut self) -> &mut T {
        unsafe { &mut *(self.inner.get_as_high_bits() as *mut T) }
    }
}

impl<R, S, T> Deref for PackedBox<R, S, T> {
    type Target = SubPack<R, S, Box<T>>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<R, S, T> DerefMut for PackedBox<R, S, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

unsafe impl<R, S, T> Packable<R, S> for Box<T>
where
    R: Packable<R, DefaultStart>,
    S: BitStart,
{
    type Packed = PackedBox<R, S, T>;

    const WIDTH: u32 = usize::leading_zeros(mem::align_of::<T>());

    #[inline]
    unsafe fn store(self, p: &mut SubPack<R, S, Self>) {
        p.set_from_high_bits(Box::into_raw(self) as usize)
    }

    #[inline]
    unsafe fn load(p: &SubPack<R, S, Self>) -> Self {
        Box::from_raw(p.get_as_high_bits() as *mut T)
    }
}
