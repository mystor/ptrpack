mod core;
pub use self::core::*;

#[cfg(feature = "alloc")]
mod alloc;
#[cfg(feature = "alloc")]
pub use self::core::*;
