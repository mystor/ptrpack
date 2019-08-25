use core::fmt;
use crate::{Packable, SubPack, PackableRoot, BitStart};

macro_rules! tiny_decl {
    ($(
        $Uint:ident: $width:expr;
    )*) => {$(
        /// Helper integer value with a specific size.
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
        pub struct $Uint(usize);

        impl $Uint {
            pub fn new(value: usize) -> Option<Self> {
                let used_bits = 0usize.leading_zeros() - value.leading_zeros();
                if used_bits > $width {
                    return None;
                }
                Some($Uint(value))
            }

            pub unsafe fn new_unchecked(value: usize) -> Self {
                $Uint(value)
            }

            pub fn get(&self) -> usize {
                self.0
            }
        }

        impl fmt::Display for $Uint {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                self.0.fmt(f)
            }
        }

        unsafe impl<S: BitStart> Packable<S> for $Uint {
            type Packed = SubPack<S, $Uint>;

            const WIDTH: u32 = $width;

            #[inline]
            unsafe fn store(self, p: &mut SubPack<S, Self>) {
                p.set_from_low_bits(self.0);
            }

            #[inline]
            unsafe fn load(p: &SubPack<S, Self>) -> Self {
                $Uint(p.get_as_low_bits())
            }
        }

        unsafe impl PackableRoot for $Uint {}
    )*}
}

tiny_decl! {
    U0: 0;
    U1: 1;
    U2: 2;
    U3: 3;
    U4: 4;
    U5: 5;
    U6: 6;
    U7: 7;
    U8: 8;
    U9: 9;
    U10: 10;
    U11: 11;
    U12: 12;
    U13: 13;
    U14: 14;
    U15: 15;
    U16: 16;
    U17: 17;
    U18: 18;
    U19: 19;
    U20: 20;
    U21: 21;
    U22: 22;
    U23: 23;
    U24: 24;
    U25: 25;
    U26: 26;
    U27: 27;
    U28: 28;
    U29: 29;
    U30: 30;
    U31: 31;
    U32: 32;
}
