#![no_std]

mod aligned;
pub use self::aligned::Aligned;

mod packable;
pub use self::packable::Packable;

mod either;
pub use self::either::{Either, TinyEither, TinyUnion};

mod ptrpack;
pub use self::ptrpack::PtrPack;

pub mod detail;

#[test]
fn either() {
    let p = &1i32;
    let a = TinyEither::<&i32, &i64>::from_left(p);
    assert!(a.is_left());
    assert_eq!(a.as_left().unwrap() as *const _, p as *const _);

    let q = &1i64;
    let b = TinyEither::<&i32, &i64>::from_right(q);
    assert!(b.is_right());
    assert_eq!(b.as_right().unwrap() as *const _, q as *const _);
}
