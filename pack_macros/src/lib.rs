#![recursion_limit = "128"]

extern crate proc_macro;

use syn::{parse_macro_input, DeriveInput};

mod packable2;

#[proc_macro_derive(Packable)]
pub fn derive_packable(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match packable2::do_derive_packable(&input) {
        Ok(expanded) => expanded.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

#[cfg(test)]
mod test;
