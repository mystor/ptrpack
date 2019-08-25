use crate::{PackableRoot, Packable, SubPack, BitStart};
use ::core::cmp;
use ::core::fmt;
use ::core::ops;

#[repr(transparent)]
pub struct PackedCopy<R, S, P> {
    inner: SubPack<R, S, P>,
}

impl<R, S, P> PackedCopy<R, S, P>
where
    R: PackableRoot,
    S: BitStart,
    P: Packable<R, S> + Copy,
{
    pub fn get(&self) -> P {
        self.inner.get()
    }

    pub fn set(&mut self, new: P) {
        self.inner.replace(new);
    }

    pub fn replace(&mut self, new: P) -> P {
        self.inner.replace(new)
    }
}

impl<R, S, P> cmp::PartialEq for PackedCopy<R, S, P>
where
    R: PackableRoot,
    S: BitStart,
    P: Packable<R, S> + Copy + cmp::PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.get().eq(&other.get())
    }
}

impl<R, S, P> cmp::PartialEq<P> for PackedCopy<R, S, P>
where
    R: PackableRoot,
    S: BitStart,
    P: Packable<R, S> + Copy + cmp::PartialEq,
{
    fn eq(&self, other: &P) -> bool {
        self.get().eq(other)
    }
}

impl<R, S, P> cmp::Eq for PackedCopy<R, S, P>
where
    R: PackableRoot,
    S: BitStart,
    P: Packable<R, S> + Copy + cmp::Eq,
{
}

impl<R, S, P> cmp::PartialOrd for PackedCopy<R, S, P>
where
    R: PackableRoot,
    S: BitStart,
    P: Packable<R, S> + Copy + cmp::PartialOrd,
{
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        self.get().partial_cmp(&other.get())
    }
}

impl<R, S, P> cmp::PartialOrd<P> for PackedCopy<R, S, P>
where
    R: PackableRoot,
    S: BitStart,
    P: Packable<R, S> + Copy + cmp::PartialOrd<P>,
{
    fn partial_cmp(&self, other: &P) -> Option<cmp::Ordering> {
        self.get().partial_cmp(other)
    }
}

impl<R, S, P> cmp::Ord for PackedCopy<R, S, P>
where
    R: PackableRoot,
    S: BitStart,
    P: Packable<R, S> + Copy + cmp::Ord,
{
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.get().cmp(&other.get())
    }
}

impl<R, S, P> fmt::Debug for PackedCopy<R, S, P>
where
    R: PackableRoot,
    S: BitStart,
    P: Packable<R, S> + Copy + fmt::Debug
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_tuple("Packed")
            .field(&self.get())
            .finish()
    }
}

mod core;
pub use self::core::*;

#[cfg(feature = "alloc")]
mod alloc;
#[cfg(feature = "alloc")]
pub use self::alloc::*;

mod tinyuint;
pub use self::tinyuint::*;
