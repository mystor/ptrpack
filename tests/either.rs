extern crate std;

use pack::Pack;
use pack_macros::Packable;

#[derive(Packable)]
struct Something<'a, T> {
    apple: &'a T,
    pear: bool,
}

#[test]
fn test_something() {
    let apple = &15u32;
    let pear = true;

    let something = Something {apple, pear};
    let packed = Pack::new(something);

    let apple_ = packed.get_apple();
    let pear_ = packed.get_pear();
    assert_eq!(apple_, &apple);
    assert_eq!(pear_, &pear);
}

#[derive(Packable, Debug, Eq, PartialEq, Copy, Clone)]
pub enum EitherRef<'a, T, U> {
    Left(&'a T),
    Right(&'a U),
}

// enum Either<T, U> {
//     Left(T),
//     Right(U),
// }

#[test]
fn test_either_ref() {
    let a = 5;
    let b = 10;

    let left = EitherRef::Left(&a);
    println!("{:?}", left);

    let right = EitherRef::Right(&b);
    println!("{:?}", right);

    assert_ne!(left, right);

    let packed_l = Pack::new(left);
    println!("{:#x?} {:#x}", packed_l, &a as *const _ as usize);
    println!("{:?}", packed_l.get());

    let packed_r = Pack::new(right);
    println!("{:#x?} {:#x}", packed_r, &b as *const _ as usize);
    println!("{:?}", packed_r.get());

    assert_eq!(packed_l.get(), left);
    assert_eq!(packed_r.get(), right);
}
