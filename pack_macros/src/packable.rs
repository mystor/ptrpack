use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use syn::Ident;
use synstructure::{BindingInfo, Structure, VariantInfo};

fn used_bits(val: usize) -> u32 {
    usize::leading_zeros(0) - usize::leading_zeros(val)
}

fn nullability(structure: &Structure) -> TokenStream {
    if let Some(binding_0) = structure.variants()[0].bindings().first() {
        let ty = &binding_0.ast().ty;
        quote!(<#ty as pack::Packable>::Nullability)
    } else {
        quote!(pack::NullableStorage)
    }
}

#[derive(Default)]
struct Decls {
    next_id: usize,
    decls: TokenStream,
}

impl Decls {
    fn add(&mut self, name: &str, ty: impl ToTokens, value: impl ToTokens) -> TokenStream {
        let id = Ident::new(
            &format!("PACK_{}_{}", name, self.next_id),
            Span::call_site(),
        );
        self.next_id += 1;

        self.decls.extend(quote! {
            const #id: #ty = #value;
        });
        id.into_token_stream()
    }
}

struct PackStructure<'a> {
    variants: Vec<PackVariant<'a>>,
    discr_width: TokenStream,
    before_discr: TokenStream,
    after_discr: TokenStream,
    discr_mask: TokenStream,
}

struct PackVariant<'a> {
    discr: TokenStream,
    fields: Vec<PackField<'a>>,
    after_variant: TokenStream,
    variant: &'a VariantInfo<'a>,
}

struct PackField<'a> {
    before: TokenStream,
    after: TokenStream,
    mask: TokenStream,
    binding: &'a BindingInfo<'a>,
}

fn pack_structure<'a>(structure: &'a Structure<'a>, decls: &mut Decls) -> PackStructure<'a> {
    let mut variants = Vec::new();
    for variant in structure.variants() {
        variants.push(pack_variant(variant, &mut *decls));
    }

    let mut min_after = quote!(pack::PTR_WIDTH);
    for variant in &variants {
        let after_variant = &variant.after_variant;
        min_after = quote!(pack::const_min(#min_after, #after_variant));
    }

    let after_variants = decls.add("AFTER_VARIANTS", quote!(u32), min_after);

    let discr_width = decls.add("DISCR_WIDTH", quote!(u32), used_bits(variants.len() - 1));

    let after_discr = decls.add(
        "AFTER_DISCR",
        quote!(u32),
        quote!(#after_variants - #discr_width),
    );
    let before_discr = decls.add(
        "BEFORE_DISCR",
        quote!(u32),
        quote!(pack::PTR_WIDTH - #after_discr - #discr_width),
    );
    let discr_mask = decls.add(
        "DISCR_MASK",
        quote!(usize),
        quote!(pack::const_mask(#after_discr, #discr_width)),
    );

    PackStructure {
        variants,
        discr_width,
        after_discr,
        before_discr,
        discr_mask,
    }
}

fn pack_variant<'a>(variant: &'a VariantInfo<'a>, decls: &mut Decls) -> PackVariant<'a> {
    let mut after_variant = quote!(pack::PTR_WIDTH);
    let mut fields = Vec::new();
    for binding in variant.bindings() {
        let ty = &binding.ast().ty;

        let width = quote!(<#ty as pack::Packable>::WIDTH);

        let after = decls.add("AFTER", quote!(u32), quote!(#after_variant - #width));
        let before = decls.add(
            "BEFORE",
            quote!(u32),
            quote!(pack::PTR_WIDTH - #after - #width),
        );
        let mask = decls.add(
            "MASK",
            quote!(usize),
            quote!(pack::const_mask(#after, #width)),
        );

        after_variant = after.clone();

        fields.push(PackField {
            after,
            before,
            mask,
            binding,
        });
    }

    // Discriminant name
    let name = &variant.ast().ident;
    let discr = quote!(Pack__Discriminant::#name);

    PackVariant {
        discr,
        fields,
        after_variant,
        variant,
    }
}

fn gen_pack(packed: &PackStructure) -> TokenStream {
    let mut arms = TokenStream::new();
    for variant in &packed.variants {
        let mut body = TokenStream::new();

        // XXX: Is it worthwhile to skip packing the `0` discriminant?
        let discr = &variant.discr;
        let before_discr = &packed.before_discr;
        let after_discr = &packed.after_discr;
        body.extend(quote! {
            bits |= pack::Packable::pack(
                #discr,
                #before_discr,
                #after_discr,
            );
        });

        // Pack each field, and bit-or them into the final bits
        for field in &variant.fields {
            let binding = &field.binding;
            let before = &field.before;
            let after = &field.after;
            body.extend(quote! {
                bits |= pack::Packable::pack(#binding, #before, #after);
            })
        }

        // Build the match arm.
        let pat = variant.variant.pat();
        arms.extend(quote! {
            #pat => {
                #body
            }
        });
    }

    quote! {
        let mut bits = 0usize;
        match self {
            #arms
        }
        bits.wrapping_shr(before)
    }
}

fn gen_unpack(packed: &PackStructure) -> TokenStream {
    // Unpack the discriminant to match on it.
    let discr_mask = &packed.discr_mask;
    let before_discr = &packed.before_discr;
    let after_discr = &packed.after_discr;
    let discr = quote! {
        <Pack__Discriminant as pack::Packable>::unpack(
            shifted & #discr_mask,
            #before_discr,
            #after_discr,
        )
    };

    let mut arms = TokenStream::new();
    for variant in &packed.variants {
        // Build up constructor for each variant.
        let ctor = variant.variant.construct(|_, idx| {
            let field = &variant.fields[idx];
            let mask = &field.mask;
            let before = &field.before;
            let after = &field.after;
            let ty = &field.binding.ast().ty;

            quote! {
                <#ty as pack::Packable>::unpack(
                    shifted & #mask,
                    #before,
                    #after,
                )
            }
        });

        let variant_discr = &variant.discr;
        arms.extend(quote! {
            #variant_discr => #ctor,
        });
    }

    quote! {
        let shifted = packed.wrapping_shl(before);
        match #discr {
            #arms
        }
    }
}

fn gen_discriminant(packed: &PackStructure) -> TokenStream {
    let mut discriminants = TokenStream::new();
    for (idx, variant) in packed.variants.iter().enumerate() {
        let name = &variant.variant.ast().ident;
        discriminants.extend(quote!(#name = #idx,));
    }

    let discr_width = &packed.discr_width;
    quote! {
        #[repr(usize)]
        pub enum Pack__Discriminant {
            #discriminants
        }

        unsafe impl pack::Packable for Pack__Discriminant {
            const WIDTH: u32 = #discr_width;
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
    }
}

pub fn derive_packable(structure: Structure) -> TokenStream {
    let mut decls = Decls::default();
    let packed = pack_structure(&structure, &mut decls);

    let discriminant = gen_discriminant(&packed);

    let impl_pack = gen_pack(&packed);
    let impl_unpack = gen_unpack(&packed);

    let decls_tokens = &decls.decls;
    let after_discr = &packed.after_discr;
    let storage = nullability(&structure);
    structure.gen_impl(quote! {
        extern crate pack;

        #discriminant

        #decls_tokens

        gen unsafe impl pack::Packable for @Self {
            const WIDTH: u32 = pack::PTR_WIDTH - #after_discr;
            type Storage = #storage;
            type Discriminant = Pack__Discriminant;

            #[inline]
            fn pack(&self, before: u32, _after: u32) -> usize {
                #impl_pack
            }

            #[inline]
            unsafe fn unpack(packed: usize, before: u32, _after: u32) -> Self {
                #impl_unpack
            }
        }
    })
}
