use syn::{parse_quote, Generics, WherePredicate, Data, DataStruct, DataEnum, DeriveInput, Ident, Fields, Error};
use quote::{quote, format_ident};
use proc_macro2::TokenStream;

fn add_pred(generics: &mut Generics, pred: WherePredicate) {
    generics.make_where_clause().predicates.push(pred);
}

fn struct_data(
    name: &Ident,
    data: &DataStruct,
    helper_impls: &mut TokenStream,
    store_impl: &mut TokenStream,
    load_impl: &mut TokenStream,
    last_bitstart: &mut TokenStream,
) {
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

        // Store Impl
        store_impl.extend(quote! {
            <#ty_as_packable>::store(#fname_local, _pack.as_field_mut::<#bitstart, #ty>());
        });
        dtor_body.extend(
            match &field.ident {
                Some(name) => quote!(#name: #fname_local,),
                None => quote!(#fname_local,),
            }
        );

        // Load Impl
        load_impl.extend(quote! {
            let #fname_local = <#ty_as_packable>::load(_pack.as_field::<#bitstart, #ty>());
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
                unsafe { ::core::mem::transmute(self) }
            }

            #vis fn #fname_get_mut(&mut self) -> &mut <#ty_as_packable>::Packed {
                unsafe { ::core::mem::transmute(self) }
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
    )
}

fn enum_data(
    name: &Ident,
    data: &DataEnum,
    helper_impls: &mut TokenStream,
    store_impl: &mut TokenStream,
    load_impl: &mut TokenStream,
    last_bitstart: &mut TokenStream,
) {
    panic!();
}

pub fn do_derive_packable(input: &DeriveInput) -> Result<TokenStream, Error> {
    // Introduce two additional generics for the impl.
    let mut generics = input.generics.clone();
    generics.params.push(parse_quote!(_PackRoot));
    generics.params.push(parse_quote!(_PackStart));

    add_pred(
        &mut generics,
        parse_quote! {
            _PackRoot: pack::Packable<_PackRoot, pack::DefaultStart>
        },
    );
    add_pred(
        &mut generics,
        parse_quote! {
            _PackStart: pack::BitStart
        },
    );

    let name = &input.ident;

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
            );
        }
        Data::Enum(data) => {
            enum_data(
                name,
                data,
                &mut helper_impls,
                &mut store_impl,
                &mut load_impl,
                &mut last_bitstart,
            );
        }
        Data::Union(_) => {
            return Err(Error::new_spanned(input, "union types are unsupported"));
        }
    }

    // Get the generics required for the impl.
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();
    let (_, base_type_generics, _) = input.generics.split_for_impl();

    let helper_name = format_ident!("Packed{}", name);
    let helper_ty = quote!(#helper_name #type_generics);
    let target_ty = quote!(#name #base_type_generics);
    let subpack_ty = quote!(pack::SubPack<_PackRoot, _PackStart, #target_ty>);
    let result = quote! {
        struct #helper_name #generics #where_clause {
            inner: #subpack_ty,
        }

        impl #impl_generics #helper_ty #where_clause {
            #helper_impls
        }

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

        unsafe impl #impl_generics pack::Packable<_PackRoot, _PackStart> for #target_ty #where_clause
        {
            type Packed = #helper_ty;

            const WIDTH: u32 = {
                let last_start = <#last_bitstart as pack::BitStart>::START;
                pack::PTR_WIDTH - last_start
            };

            unsafe fn store(self, _pack: &mut #subpack_ty) {
                #store_impl
            }

            unsafe fn load(_pack: &#subpack_ty) -> Self {
                #load_impl
            }
        }
    };
    Ok(result)
}
