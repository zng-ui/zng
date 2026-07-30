#![doc(html_favicon_url = "https://zng-ui.github.io/res/zng-logo-icon.png")]
#![doc(html_logo_url = "https://zng-ui.github.io/res/zng-logo.png")]
//!
//! Types and service for implementing custom setup apps.
//!
//! The primary use for this crate is creating a Windows setup executable that is compressed
//! and packaged using `cargo zng res --tool sfx`.
//!
//! # Self Setup
//!
//! For [`zng`] apps the installer/uninstaller logic can be build as an alternate mode of the app main executable, avoiding
//! the cost of packaging a separate executable for install/uninstall and sharing the app look and feel.
//!
//! [`zng`]: https://crates.io/crates/zng
//!
//! # Crate
//!
#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]
#![warn(unused_extern_crates)]
#![warn(missing_docs)]

mod service;
pub mod tasks;
pub mod view;
