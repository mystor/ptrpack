use pack_macros::Packable;
use pack::Packable;

#[derive(Packable)]
enum Either<T, U> {
    Left(T),
    Right(U),
}

