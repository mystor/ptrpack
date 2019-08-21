extern crate std;

use pack::Pack;
use pack_macros::Packable;

#[derive(Packable)]
struct Something<'a> {
    apple: &'a u32,
    pear: bool,
}

#[test]
fn test_something() {
    let apple = &15u32;
    let pear = true;

    let something = Something {apple, pear};
    let packed = Pack::new(something);
}

/*
#[derive(Packable)]
enum Either<T, U> {
    Left(T),
    Right(U),
}
*/
