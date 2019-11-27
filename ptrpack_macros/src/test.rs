use crate::packable::do_derive_packable;
use syn::{parse_quote, DeriveInput};

use std::io::Write;
use std::process::{Command, Stdio};

#[test]
#[ignore]
fn my_test() {
    let input: DeriveInput = parse_quote! {
        struct Something<T> {
            f1: &T,
            f2: bool,
        }
    };

    let output = do_derive_packable(&input).unwrap();
    println!("{}", output);

    let multiline_output = output.to_string().replace("{", "{\n");

    let mut child = Command::new("rustfmt")
        .stdin(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .unwrap()
        .write(multiline_output.as_bytes())
        .unwrap();
    child.stdin = None;
    child.wait().unwrap();

    panic!();
}
