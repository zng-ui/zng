#![doc(html_favicon_url = "https://zng-ui.github.io/res/zng-logo-icon.png")]
#![doc(html_logo_url = "https://zng-ui.github.io/res/zng-logo.png")]
//!
//! Base unit types.
//!
//! # Crate
//!
#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]
#![warn(unused_extern_crates)]
#![warn(missing_docs)]

mod angle;
mod byte;
mod color;
mod corner_radius;
mod distance_key;
mod factor;
mod float_eq;
mod orientation;
mod px_dip;
mod side_offsets;
mod time;
mod transform;

use std::fmt;

#[doc(no_inline)]
pub use euclid;

pub use angle::*;
pub use byte::*;
pub use color::*;
pub use corner_radius::*;
pub use distance_key::*;
pub use factor::*;
pub use float_eq::*;
pub use orientation::*;
pub use px_dip::*;
pub use side_offsets::*;
pub use time::*;
pub use transform::*;

pub(crate) fn parse_suffix<T: std::str::FromStr>(mut s: &str, suffixes: &[&'static str]) -> Result<T, <T as std::str::FromStr>::Err> {
    for suffix in suffixes {
        if let Some(f) = s.strip_suffix(suffix) {
            s = f;
            break;
        }
    }
    s.parse()
}

/// An error which can be returned when parsing an type composed of integers.
#[derive(Debug)]
#[non_exhaustive]
pub enum ParseIntCompositeError {
    /// Color component parse error.
    Component(std::num::ParseIntError),
    /// Missing color component.
    MissingComponent,
    /// Extra color component.
    ExtraComponent,
    /// Unexpected char.
    UnknownFormat,
}
impl fmt::Display for ParseIntCompositeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseIntCompositeError::Component(e) => write!(f, "error parsing component, {e}"),
            ParseIntCompositeError::MissingComponent => write!(f, "missing component"),
            ParseIntCompositeError::ExtraComponent => write!(f, "extra component"),
            ParseIntCompositeError::UnknownFormat => write!(f, "unknown format"),
        }
    }
}
impl std::error::Error for ParseIntCompositeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        if let ParseIntCompositeError::Component(e) = self {
            Some(e)
        } else {
            None
        }
    }
}
impl From<std::num::ParseIntError> for ParseIntCompositeError {
    fn from(value: std::num::ParseIntError) -> Self {
        ParseIntCompositeError::Component(value)
    }
}
