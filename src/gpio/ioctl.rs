mod v1;
mod v2;

// gpiochip v2 API support is a work in progress, and should only be used for testing purposes.
#[cfg(not(feature = "gpiochip_v2"))]
pub use v1::*;
#[cfg(feature = "gpiochip_v2")]
pub use v2::*;
