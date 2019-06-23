use synstructure::decl_derive;

mod packable;

decl_derive!([Packable] => packable::derive_packable);


#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
