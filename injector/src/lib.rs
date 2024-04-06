#[cfg(feature = "nightly")]
mod implementation;

#[cfg(feature = "nightly")]
pub use implementation::*;

#[cfg(not(feature = "nightly"))]
pub const THIS_CRATE_ONLY_SUPPORTS_NIGHTLY: () = ();
