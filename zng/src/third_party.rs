//! Third party licenses service and types.
//!
//! Rust projects depend on many crated with a variety of licenses, some of these licenses require that they must be
//! displayed in the app binary, usually in an "about" screen. This module can be used together with the [`zng_tp_licenses`]
//! crate to collect and bundle licenses of all used crates in your project.
//!
//! The [`LICENSES`] service serves as an aggregation center for licenses of multiple sources, the [`OPEN_LICENSES_CMD`]
//! can be implemented [`on_pre_event`] to show a custom licenses screen, or it can just be used to show the default
//! screen provided by the default app.
//!
//! # Bundle Setup
//!
//! Follow these steps to configure your crate and build workflow to collect and bundle crate licenses.
//!
//! ### Install `cargo about`
//!
//! To collect and bundle licenses in your project you must have [`cargo-about`] installed:
//!
//! ```console
//! cargo install cargo-about
//! ```
//!
//! Next add file `.cargo/about.toml` in your crate or workspace root:
//!
//! ```toml
//! # cargo about generate -c .cargo/about.toml --format json --workspace --all-features
//!
//! accepted = [
//!     "Apache-2.0",
//!     "MIT",
//!     "MPL-2.0",
//!     "Unicode-DFS-2016",
//!     "BSL-1.0",
//!     "BSD-2-Clause",
//!     "BSD-3-Clause",
//!     "ISC",
//!     "Zlib",
//!     "CC0-1.0",
//! ]
//!
//! ignore-build-dependencies = true
//! ignore-dev-dependencies = true
//! filter-noassertion = true
//! private = { ignore = true }
//! ```
//!
//! Next call the command to test and modify the `accepted` config:
//!
//! ```console
//! cargo about generate -c .cargo/about.toml --format json --workspace --all-features
//! ```
//!
//! If the command prints a massive JSON dump, you are done with this step.
//!
//! ### Add `zng-tp-licenses`
//!
//! Next, add dependency to the [`zng_tp_licenses`] your crate `Cargo.toml`:
//!
//! ```toml
//! [package]
//! resolver = "2" # recommended, to not include "build" feature in the normal dependency.
//!
//! [features]
//! # Recommended, so you only bundle in release builds.
//! #
//! # Note that if you use a feature, don't forget to build with `--features bundle_licenses`.
//! bundle_licenses = ["dep:zng-tp-licenses"]
//!
//! [dependencies]
//! zng-tp-licenses = { version = "0.2.0", feature = ["bundle"], optional = true }
//!
//! [build-dependencies]
//! zng-tp-licenses = { version = "0.2.0", feature = ["build"], optional = true }
//! ```
//!
//! ### Implement Bundle
//!
//! Next, in your crates build script (`build.rs`) add:
//!
//! ```
//! fn main() {
//!     #[cfg(feature = "bundle_licenses")]
//!     {
//!         let licenses = zng_tp_licenses::collect_cargo_about("../.cargo/about.toml");
//!         zng_tp_licenses::write_bundle(&licenses);
//!     }
//! }
//! ```
//!
//! Implement a function that includes the bundle and decodes it. Register the function it in your app init code:
//!
//! ```
//! #[cfg(feature = "bundle_licenses")]
//! fn bundled_licenses() -> Vec<zng::third_party::LicenseUsed> {
//!     zng_tp_licenses::include_bundle!()
//! }
//!
//! # fn demo() {
//! # use zng::prelude::*;
//! APP.defaults().run(async {
//!     #[cfg(feature = "bundle_licenses")]
//!     zng::third_party::LICENSES.register(bundled_licenses);
//! });
//! # }
//! # fn main() { }
//! ```
//!
//! ### Review Licenses
//!
//! Call the [`OPEN_LICENSES_CMD`] in a test button, check if all the required licenses are present,
//! `cargo about` and `zng_tp_licenses` are a **best effort only** helpers, you must ensure that the generated results
//! meet yours or your company's legal obligations.
//!
//! ```
//! use zng::prelude::*;
//!
//! fn review_licenses() -> impl UiNode {
//!     // zng::third_party::LICENSES.include_view_process().set(false);
//!
//!     Button!(zng::third_party::OPEN_LICENSES_CMD)
//! }
//! ```
//!
//! #### Limitations
//!
//! Only crate licenses reachable thought cargo metadata are included. Static linked libraries in `-sys` crates may
//! have required licenses that are not included. Other bundled resources such as fonts and images may also be licensed.
//!
//! The [`LICENSES`] service accepts multiple sources, so you can implement your own custom bundle, the [`zng_tp_licenses`]
//! provides helpers for manually encoding (compressing) licenses. See the `zng-view` build script for an example of how
//! to include more licenses.
//!
//! # Full API
//!
//! See [`zng_app::third_party`] and [`zng_tp_licenses`] for the full API.
//!
//! [`zng_tp_licenses`]: https://zng-ui.github.io/doc/zng_tp_licenses/
//! [`cargo-about`]: https://github.com/EmbarkStudios/cargo-about/
//! [`on_pre_event`]: crate::event::Command::on_pre_event

pub use zng_app::third_party::{License, LicenseUsed, User, UserLicense, LICENSES, OPEN_LICENSES_CMD};
