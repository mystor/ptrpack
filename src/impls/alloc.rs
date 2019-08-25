//! Helper types and impls only used if the `alloc` feature is enabled.

use crate::{Packable, SubPack, RawPackedBits};
use crate::bitstart::BitStart;

use alloc::boxed::Box;
use core::mem;
use core::ops::{Deref, DerefMut};

pub struct PackedBox<S, T> {
    inner: SubPack<S, Box<T>>,
}

impl<S: BitStart, T> PackedBox<S, T> {
    /*
    /// Get the `Box<T>` value as `&T`
    pub fn as_ref(&self) -> &T {
        unsafe { &*(self.inner.get_as_high_bits() as *const T) }
    }

    /// Get the `Box<T>` value as `&mut T`
    pub fn as_mut(&mut self) -> &mut T {
        unsafe { &mut *(self.inner.get_as_high_bits() as *mut T) }
    }
    */
}

impl<S, T> Deref for PackedBox<S, T> {
    type Target = SubPack<S, Box<T>>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<S, T> DerefMut for PackedBox<S, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

unsafe impl<S: BitStart, T> Packable<S> for Box<T> {
    type Packed = PackedBox<S, T>;

    const WIDTH: u32 = usize::leading_zeros(mem::align_of::<T>() - 1);

    #[inline]
    unsafe fn store(self, p: &mut RawPackedBits<S, Self>) {
        p.write_high_bits(Box::into_raw(self) as usize)
    }

    #[inline]
    unsafe fn load(p: &RawPackedBits<S, Self>) -> Self {
        Box::from_raw(p.read_high_bits() as *mut T)
    }
}
