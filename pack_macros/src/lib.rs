#![recursion_limit = "128"]

extern crate proc_macro;

use syn::{parse_macro_input, DeriveInput};

mod packable2;

#[proc_macro_derive(Packable)]
pub fn derive_packable(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let stream: proc_macro::TokenStream = match packable2::do_derive_packable(&input) {
        Ok(expanded) => expanded.into(),
        Err(error) => error.to_compile_error().into(),
    };

    if true {
        use std::process::{Command, Stdio};
        use std::io::Write;

        let multiline_output = stream.to_string().replace("{ ", "{\n");
        let mut child = Command::new("rustfmt")
            .stdin(Stdio::piped())
            .spawn()
            .unwrap();
        child.stdin.unwrap().write(multiline_output.as_bytes()).unwrap();
        child.stdin = None;
        child.wait().unwrap();
    }

    stream
}

#[cfg(test)]
mod test;
