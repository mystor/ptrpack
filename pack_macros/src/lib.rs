#![recursion_limit = "128"]

use synstructure::decl_derive;

mod packable;

decl_derive!([Packable] => packable::derive_packable);

#[cfg(test)]
mod test;

