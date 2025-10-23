#![doc(html_favicon_url = "https://zng-ui.github.io/res/zng-logo-icon.png")]
#![doc(html_logo_url = "https://zng-ui.github.io/res/zng-logo.png")]
//! Properties that define accessibility metadata.
//!
//! The properties in this crate should only be used by widget implementers, they only
//! define metadata for accessibility, this metadata signals the availability of behaviors
//! that are not implemented by these properties, for example an [`AccessRole::Button`] widget
//! must also be focusable and handle click events, an [`AccessRole::TabList`] must contain widgets
//! marked [`AccessRole::Tab`].
//!
//! [`AccessRole::Button`]: zng_app::widget::info::access::AccessRole::Button
//! [`AccessRole::TabList`]: zng_app::widget::info::access::AccessRole::TabList
//! [`AccessRole::Tab`]: zng_app::widget::info::access::AccessRole::Tab
//!
//! # Crate
//!
#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]
#![warn(unused_extern_crates)]
#![warn(missing_docs)]

mod events;
mod meta;
pub use events::*;
pub use meta::*;
