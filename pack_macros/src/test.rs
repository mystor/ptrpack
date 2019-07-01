use crate::packable::derive_packable;
use synstructure::test_derive;

#[test]
fn empty() {
    test_derive! {
        derive_packable {
            struct Empty;
        }
        expands to {
            #[allow(non_upper_case_globals)]
            const _DERIVE_pack_Packable_FOR_Empty: () = {
                extern crate pack;

                #[repr(usize)]
                pub enum Pack__Discriminant {
                    Empty = 0usize,
                }

                // pack::unsafe_impl_discriminant_pack!(Pack__Discriminant, PACK_DISCR_WIDTH_1);

                unsafe impl pack::Packable for Pack__Discriminant {
                    const WIDTH: u32 = PACK_DISCR_WIDTH_1;
                    type Storage = pack::NullableStorage;
                    type Discriminant = Pack__Discriminant;

                    #[inline]
                    fn pack(&self, _before: u32, after: u32) -> usize {
                        (self as usize).wrapping_shl(after)
                    }

                    #[inline]
                    unsafe fn unpack(packed: usize, _before: u32, after: u32) -> usize {
                        ::core::mem::transmute::<usize, Self>(packed.wrapping_shr(after))
                    }
                }

                const PACK_AFTER_VARIANTS_0: u32 = pack::const_min(pack::PTR_WIDTH, pack::PTR_WIDTH);
                const PACK_DISCR_WIDTH_1: u32 = 0u32;
                const PACK_AFTER_DISCR_2: u32 = PACK_AFTER_VARIANTS_0 - PACK_DISCR_WIDTH_1;
                const PACK_BEFORE_DISCR_3: u32 = pack::PTR_WIDTH - PACK_AFTER_DISCR_2 - PACK_DISCR_WIDTH_1;
                const PACK_DISCR_MASK_4: usize = pack::const_mask(PACK_AFTER_DISCR_2, PACK_DISCR_WIDTH_1);

                unsafe impl pack::Packable for Empty {
                    const WIDTH: u32 = pack::PTR_WIDTH - PACK_AFTER_DISCR_2;
                    type Storage = pack::NullableStorage;
                    type Discriminant = Pack__Discriminant;

                    #[inline]
                    fn pack(&self, before: u32, _after: u32) -> usize {
                        let mut bits = 0usize;
                        match self {
                            Empty => {
                                bits |= pack::Packable::pack(
                                    Pack__Discriminant::Empty,
                                    PACK_BEFORE_DISCR_3,
                                    PACK_AFTER_DISCR_2,
                                );
                            }
                        }
                        bits.wrapping_shr(before)
                    }

                    #[inline]
                    unsafe fn unpack(packed: usize, before: u32, _after: u32) -> Self {
                        let shifted = packed.wrapping_shl(before);
                        match <Pack__Discriminant as pack::Packable>::unpack(
                            shifted & PACK_DISCR_MASK_4,
                            PACK_BEFORE_DISCR_3,
                            PACK_AFTER_DISCR_2,
                        ) {
                            Pack__Discriminant::Empty => Empty,
                        }
                    }
                }
            };
        }
        no_build
    }
}