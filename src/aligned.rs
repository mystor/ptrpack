use crate::{detail, Packable};
use core::{cmp, fmt, hash, mem, ops, ptr};

/// An aligned, non-null raw pointer.
///
/// Like [`ptr::NonNull`], but also required to be correctly aligned.
#[repr(transparent)]
pub struct Aligned<T>(ptr::NonNull<T>);

impl<T> Aligned<T> {
    const FREE_BITS: u32 = mem::align_of::<T>().trailing_zeros();
    const ALIGN_MASK: usize = (1usize << Self::FREE_BITS) - 1;

    pub fn new(ptr: ptr::NonNull<T>) -> Option<Self> {
        let value = ptr.as_ptr() as usize;
        if (value & Self::ALIGN_MASK) != 0 {
            None
        } else {
            Some(unsafe { Self::new_unchecked(ptr) })
        }
    }

    pub const unsafe fn new_unchecked(ptr: ptr::NonNull<T>) -> Self {
        Aligned(ptr)
    }

    // FIXME: should be const eventually.
    pub const fn dangling() -> Self {
        let dangling = mem::align_of::<T>() as *mut T;
        unsafe { Self::new_unchecked(ptr::NonNull::new_unchecked(dangling)) }
    }

    pub const fn as_ptr(self) -> *mut T {
        self.0.as_ptr()
    }
}

impl<T> ops::Deref for Aligned<T> {
    type Target = ptr::NonNull<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// Handy trait impls to act like a pointer.
impl<T> Copy for Aligned<T> {}
impl<T> Clone for Aligned<T> {
    fn clone(&self) -> Self {
        Aligned(self.0)
    }
}
impl<T> cmp::Eq for Aligned<T> {}
impl<T> cmp::PartialEq for Aligned<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}
impl<T> cmp::Ord for Aligned<T> {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.0.cmp(&other.0)
    }
}
impl<T> cmp::PartialOrd for Aligned<T> {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl<T> hash::Hash for Aligned<T> {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state)
    }
}
impl<T> fmt::Debug for Aligned<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_tuple("Aligned").field(&self.0.as_ptr()).finish()
    }
}

unsafe impl<T> Packable for Aligned<T> {
    type BitAlign = detail::HighBits;
    type Storage = detail::NonNullStorage;

    const BITS: u32 = detail::PTR_WIDTH - Self::FREE_BITS;

    unsafe fn from_bits(bits: usize) -> Self {
        mem::transmute::<usize, Self>(bits)
    }
    fn to_bits(self) -> usize {
        unsafe { mem::transmute::<Self, usize>(self) }
    }
}
