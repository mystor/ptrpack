use crate::{detail, Packable};
use core::{cmp, fmt, hash, marker};

/// A tuple value packed into a single pointer-sized value.
///
/// # Template Parameters
///
/// The type parameter, `T`, must be a tuple containing values which implement
/// [`Packable`]. Currently tuple sizes of up to 16 are supported. See
/// [`Packable`] for more details on what types of values may be packed into a
/// pointer.
#[repr(transparent)]
pub struct PtrPack<T: detail::PackableTuple> {
    data: T::Storage,
    _marker: marker::PhantomData<T>,
}

impl<T: detail::PackableTuple> PtrPack<T> {
    /// Pack the given tuple into a pointer-sized value.
    pub fn new(tuple: T) -> Self {
        let bits = T::to_tuple_bits(tuple);
        unsafe { Self::from_bits(bits) }
    }

    /// Extract the tuple value from this [`PtrPack`].
    pub fn to_tuple(self) -> T {
        unsafe { T::from_tuple_bits(self.to_bits()) }
    }

    /// Unsafely construct one of these values from bits.
    pub unsafe fn from_bits(bits: usize) -> Self {
        PtrPack {
            data: detail::PointerStorage::from_bits(bits),
            _marker: marker::PhantomData,
        }
    }

    /// Unsafely convert one of these values to bits.
    pub fn to_bits(self) -> usize {
        detail::PointerStorage::to_bits(self.data)
    }
}

impl<T: detail::PackableTuple> Copy for PtrPack<T> {}
impl<T: detail::PackableTuple> Clone for PtrPack<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: detail::PackableTuple> cmp::Eq for PtrPack<T> {}
impl<T: detail::PackableTuple> cmp::PartialEq for PtrPack<T> {
    fn eq(&self, other: &Self) -> bool {
        self.data == other.data
    }
}
impl<T: detail::PackableTuple> cmp::Ord for PtrPack<T> {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.data.cmp(&other.data)
    }
}
impl<T: detail::PackableTuple> cmp::PartialOrd for PtrPack<T> {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl<T: detail::PackableTuple> hash::Hash for PtrPack<T> {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.data.hash(state)
    }
}

impl<T: detail::PackableTuple + fmt::Debug> fmt::Debug for PtrPack<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_tuple("PtrPack").field(&self.to_tuple()).finish()
    }
}

// PtrPack-ed values are also packable themselves.
unsafe impl<T> Packable for PtrPack<T>
where
    T: detail::PackableTuple,
{
    type BitAlign = detail::HighBits;
    type Storage = T::Storage;

    const BITS: u32 = detail::PTR_WIDTH - T::LAST_LOW_BIT;

    unsafe fn from_bits(bits: usize) -> Self {
        Self::from_bits(bits)
    }
    fn to_bits(self) -> usize {
        self.to_bits()
    }
}

#[test]
fn test_packable() {
    let x = 5u32;
    let pack1 = <PtrPack<(&u32, bool, bool)>>::new((&x, false, true));
    let pack2 = <PtrPack<(&u32, bool, bool)>>::new((&x, false, true));
    assert_eq!(pack1, pack2);

    assert_eq!(pack1.get_0() as *const _, &x as *const _);
    assert_eq!(pack2.get_0() as *const _, &x as *const _);

    assert_eq!(pack1.get_1(), false);
    assert_eq!(pack2.get_1(), false);

    assert_eq!(pack1.get_2(), true);
    assert_eq!(pack2.get_2(), true);
}
