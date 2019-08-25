use syn::{parse_quote, Generics, WherePredicate, Data, DataStruct, DataEnum, DeriveInput, Ident, Fields, Error, Type, Lifetime};
use syn::fold::Fold;
use quote::{quote, format_ident};
use proc_macro2::TokenStream;

/// The width of a pointer on the platform which is running this proc-macro.
const HOST_PTR_WIDTH: u32 = 0usize.leading_zeros();

/*
fn add_pred(generics: &mut Generics, pred: WherePredicate) {
    generics.make_where_clause().predicates.push(pred);
}
*/

fn struct_data(
    name: &Ident,
    data: &DataStruct,
    helper_impls: &mut TokenStream,
    store_impl: &mut TokenStream,
    load_impl: &mut TokenStream,
    last_bitstart: &mut TokenStream,
) -> Result<(), Error> {
    let mut dtor_body = TokenStream::new();
    let mut ctor_body = TokenStream::new();
    for (idx, field) in data.fields.iter().enumerate() {
        let ty = &field.ty;
        let vis = &field.vis;

        let fname_s = match &field.ident{
            Some(name) => name.to_string(),
            None => idx.to_string(),
        };
        let fname_local = format_ident!("_field_{}", fname_s);
        let fname_get = format_ident!("get_{}", fname_s);
        let fname_get_mut = format_ident!("get_{}_mut", fname_s);

        let bitstart = last_bitstart.clone();
        *last_bitstart = quote!(pack::NextStart<_PackRoot, #bitstart, #ty>);

        let ty_as_packable = quote!(#ty as pack::Packable<_PackRoot, #bitstart>);

        let as_field_mut = quote!(as_field_mut::<#bitstart, #ty>());
        let as_field = quote!(as_field::<#bitstart, #ty>());

        // Store Impl
        store_impl.extend(quote! {
            <#ty_as_packable>::store(#fname_local, _pack.#as_field_mut);
        });
        dtor_body.extend(
            match &field.ident {
                Some(name) => quote!(#name: #fname_local,),
                None => quote!(#fname_local,),
            }
        );

        // Load Impl
        load_impl.extend(quote! {
            let #fname_local = <#ty_as_packable>::load(_pack.#as_field);
        });
        ctor_body.extend(
            match &field.ident {
                Some(name) => quote!(#name: #fname_local,),
                None => quote!(#fname_local,),
            }
        );

        // Helper Getter Methods
        helper_impls.extend(quote! {
            // It'd be lovely if I could use associated types here - these decls
            // can end up really long!
            #vis fn #fname_get(&self) -> &<#ty_as_packable>::Packed {
                unsafe { (self.#as_field).as_packed() }
            }

            #vis fn #fname_get_mut(&mut self) -> &mut pack::PackedType<_PackRoot, #bitstart, #ty> {
                unsafe { (self.#as_field_mut).as_packed_mut() }
            }
        });
    }

    let destruct = match &data.fields {
        Fields::Named(_) => quote!(#name { #dtor_body }),
        Fields::Unnamed(_) => quote!(#name(#dtor_body)),
        Fields::Unit => quote!(#name),
    };
    *store_impl = quote! {
        let #destruct = self;
        #store_impl
    };

    // Load Impl Ctor
    load_impl.extend(
        match &data.fields {
            Fields::Named(_) => quote!(#name { #ctor_body }),
            Fields::Unnamed(_) => quote!(#name(#ctor_body)),
            Fields::Unit => quote!(#name),
        }
    );

    Ok(())
}

fn enum_data(
    name: &Ident,
    data: &DataEnum,
    helper_impls: &mut TokenStream,
    store_impl: &mut TokenStream,
    load_impl: &mut TokenStream,
    last_bitstart: &mut TokenStream,
) -> Result<(), Error> {
    // FIXME: Should there be a behaviour for packing values with non-trivial
    // enum variants (either unit or single-element tuple variants?).
    //
    // Doing so is certainly possible, but the API might not be super great.

    // FIXME: Extra code can probably be generated for the `Option<T>`-style
    // case to support the nonzero pointer optimization. Perhaps detect the
    // value looks like `Option<T>`, and forward to it under the hood?

    // How many bits are required for the discriminant.
    let discr_bits = HOST_PTR_WIDTH - (data.variants.len() - 1).leading_zeros();
    if discr_bits > 32 {
        return Err(Error::new_spanned(data.enum_token, "Too many variants! At most 2^32 - 1 variants are supported"));
    }
    let discr_ty_id = format_ident!("U{}", discr_bits);
    let discr_ty = quote!(pack::impls::#discr_ty_id);

    let bitstart = last_bitstart.clone();
    let mut store_arms = TokenStream::new();
    let mut load_arms = TokenStream::new();
    let mut discr_bitstart: Option<TokenStream> = None;
    for (idx, variant) in data.variants.iter().enumerate() {
        let variant_name = &variant.ident;

        match &variant.fields {
            Fields::Named(_) => {
                // FIXME: Better errors
                return Err(Error::new_spanned(variant, "struct-style enum variants unsupported"));
            }
            Fields::Unnamed(fields) => {
                if fields.unnamed.len() != 1 {
                    // FIXME: Better errors
                    return Err(Error::new_spanned(variant, "multiple fields in enum variants are unsupported"));
                }

                let field = &fields.unnamed[0];
                let ty = &field.ty;

                // Update bitstart value for the discriminant.
                let after_bitstart = quote!(pack::NextStart<_PackRoot, #bitstart, #ty>);
                discr_bitstart = Some(
                    discr_bitstart
                        .map(|bs| quote!(pack::UnionStart<#bs, #after_bitstart>))
                        .unwrap_or_else(|| after_bitstart.clone())
                );

                let ty_as_packable = quote!(#ty as pack::Packable<_PackRoot, #bitstart>);
                let as_field_mut = quote!(as_field_mut::<#bitstart, #ty>());
                let as_field = quote!(as_field::<#bitstart, #ty>());

                store_arms.extend(quote! {
                    #name::#variant_name(_field) => {
                        <#bitstart as pack::BitStart>::write_raw(_pack.as_root_mut(), _field);
                        #discr_ty::new_unchecked(#idx)
                    }
                });
                load_arms.extend(quote! {
                    #idx => #name::#variant_name(<#bitstart as pack::BitStart>::read_raw(_pack.as_root())),
                });
            }
            Fields::Unit => {
                store_arms.extend(quote! {
                    #name::#variant_name => #idx,
                });
                load_arms.extend(quote! {
                    #idx => #name::#variant_name,
                });
            }
        }
    }

    let discr_bitstart = discr_bitstart.unwrap_or_else(|| bitstart.clone());
    *last_bitstart = quote!(pack::NextStart<_PackRoot, #discr_bitstart, #discr_ty>);

    store_impl.extend(quote! {
        let discr = match self {
            #store_arms
        };
        _pack.as_field_mut::<#discr_bitstart, #discr_ty>().write_raw(discr);
    });

    load_impl.extend(quote! {
        let discr = _pack.as_field::<#discr_bitstart, #discr_ty>().read_raw();
        match discr.get() {
            #load_arms
            _ => ::core::hint::unreachable_unchecked(),
        }
    });

    Ok(())
}

pub fn do_derive_packable(input: &DeriveInput) -> Result<TokenStream, Error> {
    // Introduce two additional generics for the impl.
    let mut generics = input.generics.clone();
    generics.params.push(parse_quote!(_PackRoot: pack::PackableRoot));
    generics.params.push(parse_quote!(_PackStart: pack::BitStart));

    let name = &input.ident;
    let vis = &input.vis;

    let mut helper_impls = TokenStream::new();
    let mut store_impl = TokenStream::new();
    let mut load_impl = TokenStream::new();
    let mut last_bitstart = quote!(_PackStart);

    match &input.data {
        Data::Struct(data) => {
            struct_data(
                name,
                data,
                &mut helper_impls,
                &mut store_impl,
                &mut load_impl,
                &mut last_bitstart,
            )?;
        }
        Data::Enum(data) => {
            enum_data(
                name,
                data,
                &mut helper_impls,
                &mut store_impl,
                &mut load_impl,
                &mut last_bitstart,
            )?;
        }
        Data::Union(_) => {
            return Err(Error::new_spanned(input, "union types are unsupported"));
        }
    }

    // Unfortunately, using the full types for members such as `&'a T` when
    // computing the `WIDTH` constant produces a compiler error, where the
    // compiler complains it cannot ensure `T` outlives `&'a`. This constraint
    // should be enforced already, as the type exists as a field in the struct
    // we're implementing, but the compiler appears to be unaware.
    //
    // This `fold` pass patches up the `last_bitstart` value to use inferred
    // lifetimes, which dodges this well-formedness issue.
    //
    // FIXME: This probably barfs on `for<'a> ...`-style expressions, and
    // perhaps that should be fixed?
    struct InferLifetimes;
    impl Fold for InferLifetimes {
        fn fold_lifetime(&mut self, l: Lifetime) -> Lifetime {
            if &l.ident == "static" {
                return l;
            }
            Lifetime::new("'_", l.apostrophe)
        }
    }
    let last_bitstart = InferLifetimes.fold_type(parse_quote!(#last_bitstart));

    // Get the generics required for the impl.
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();
    let (base_impl_generics, base_type_generics, _) = input.generics.split_for_impl();

    let helper_name = format_ident!("Packed{}", name);
    let helper_ty = quote!(#helper_name #type_generics);
    let target_ty = quote!(#name #base_type_generics);
    let subpack_ty = quote!(pack::SubPack<_PackRoot, _PackStart, #target_ty>);
    let result = quote! {
        #[repr(transparent)]
        #vis struct #helper_name #generics #where_clause {
            inner: #subpack_ty,
        }

        impl #impl_generics #helper_ty #where_clause {
            #helper_impls
        }

        // XXX: It's pretty gross that we're using `Deref` for a kind-of
        // "inheritance" here. I'd love to do something better, but it doesn't
        // look to be possible without losing `Copy`-only methods, due to rustc
        // erroring when seeing trivial `where` clauses.
        //
        // It may be possible to hide the fact these bounds are trivial from
        // rustc?
        impl #impl_generics ::core::ops::Deref for #helper_ty #where_clause {
            type Target = #subpack_ty;

            fn deref(&self) -> &Self::Target {
                &self.inner
            }
        }

        impl #impl_generics ::core::ops::DerefMut for #helper_ty #where_clause {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.inner
            }
        }

        impl #impl_generics ::core::fmt::Debug for #helper_ty #where_clause {
            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                self.inner.fmt(f)
            }
        }

        unsafe impl #impl_generics pack::Packable<_PackRoot, _PackStart> for #target_ty #where_clause {
            type Packed = #helper_ty;

            const WIDTH: u32 = {
                let old_start = _PackStart::START;
                let new_start = <#last_bitstart as pack::BitStart>::START;
                old_start - new_start
            };

            unsafe fn store(self, _pack: &mut #subpack_ty) {
                #store_impl
            }

            unsafe fn load(_pack: &#subpack_ty) -> Self {
                #load_impl
            }
        }

        // The `PackableRoot` trait impl is needed to avoid recursion when
        // performing trait resolution. It is used as a trait bound for the
        // first generic parameter of `Packable`, and implies
        // `pack::Packable<Self, DefaultStart>`.
        unsafe impl #base_impl_generics pack::PackableRoot for #target_ty #where_clause { }
    };
    Ok(result)
}
