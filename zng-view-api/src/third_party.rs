//! Third-party license management and collection.

use std::fmt;

use serde::{Deserialize, Serialize};
use zng_txt::Txt;

/// Represents a license and dependencies that use it.
#[derive(Serialize, Deserialize, Clone)]
pub struct License {
    /// License SPDX id.
    pub id: Txt,
    /// License name.
    pub name: Txt,
    /// License text.
    pub text: Txt,
    /// Project or packages that use this license.
    pub used_by: Vec<LicenseUser>,
}
impl License {
    /// Compare id, name and text.
    pub fn is_same(&self, other: &Self) -> bool {
        self.id == other.id && self.name == other.name && self.text == other.text
    }
}
impl fmt::Debug for License {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("License")
            .field("id", &self.id)
            .field("used_by", &self.used_by)
            .finish_non_exhaustive()
    }
}

/// Represents a [`License`] user.
#[derive(Serialize, Deserialize, PartialEq, Eq, Hash, Debug, Clone)]
pub struct LicenseUser {
    /// Project or package name.
    pub name: Txt,
    /// Package version.
    pub version: Txt,
    /// Project or package URL.
    pub url: Txt,
}

/// Merge `licenses` into `into`.
pub fn merge_licenses(into: &mut Vec<License>, licenses: &[License]) {
    for license in licenses {
        if let Some(l) = into.iter_mut().find(|l| l.is_same(license)) {
            for user in &license.used_by {
                if !l.used_by.contains(user) {
                    l.used_by.push(user.clone());
                }
            }
        } else {
            into.push(license.clone());
        }
    }
}

/// Calls [`cargo about`] for the crate.
///
/// This method must be used in build scripts (`build.rs`).
///
/// # Panics
///
/// Panics for any error, including `cargo about` errors and JSON deserialization errors.
///
/// [`cargo about`]: https://github.com/EmbarkStudios/cargo-about
#[cfg(feature = "third_party_collect")]
pub fn collect_cargo_about(about_cfg_path: &str) -> Vec<License> {
    let mut cargo_about = std::process::Command::new("cargo");
    cargo_about
        .arg("about")
        .arg("generate")
        .arg("--format")
        .arg("json")
        .arg("--all-features");

    if !about_cfg_path.is_empty() {
        cargo_about.arg("-c").arg(about_cfg_path);
    }

    let output = cargo_about.output().expect("error calling `cargo about`");
    let error = String::from_utf8(output.stderr).unwrap();
    assert!(
        output.status.success(),
        "error code calling `cargo about`, {:?}\nstderr:\n{error}",
        output.status
    );

    let json = String::from_utf8(output.stdout).unwrap();

    parse_cargo_about(&json).expect("error parsing `cargo about` output")
}

/// Parse the output of [`cargo about`].
///
/// Example command:
///
/// ```console
/// cargo about generate -c .cargo/about.toml --format json --workspace --all-features
/// ```
///
/// See also [`collect_cargo_about`] that calls the command.
///
/// [`cargo about`]: https://github.com/EmbarkStudios/cargo-about
#[cfg(feature = "third_party_collect")]
pub fn parse_cargo_about(json: &str) -> Result<Vec<License>, serde_json::Error> {
    #[derive(Deserialize)]
    struct Output {
        licenses: Vec<License>,
    }

    serde_json::from_str::<Output>(json).map(|o| o.licenses)
}

/// Bincode serialize and deflate the licenses.
///
/// # Panics
///
/// Panics in case of any error.
#[cfg(feature = "third_party_collect")]
pub fn encode_licenses(licenses: &[License]) -> Vec<u8> {
    deflate::deflate_bytes(&bincode::serialize(licenses).expect("bincode error"))
}

/// Encode licenses and write to the output file that is included by [`include_bundle!`].
///
/// # Panics
///
/// Panics in case of any error.
#[cfg(feature = "third_party_collect")]
pub fn write_bundle(licenses: &[License]) {
    let bin = encode_licenses(licenses);
    std::fs::write(format!("{}/zng-third-licenses.bin", std::env::var("OUT_DIR").unwrap()), bin).expect("error writing file");
}

/// Includes the bundle file generated using [`write_bundle`].
///
/// This macro output is a `Vec<License>`. Note that if not built with `feature = "third_party_bundle"` this
/// macro always returns an empty vec.
#[macro_export]
#[cfg(feature = "third_party_bundle")]
macro_rules! include_bundle {
    () => {
        $crate::include_bundle!(concat!(env!("OUT_DIR"), "/zng-third-licenses.bin"))
    };
    ($custom_name:expr) => {{
        $crate::third_party::decode_licenses(include_bytes!($custom_name))
    }};
}

/// Includes the bundle file generated using [`write_bundle`].
///
/// This macro output is a `Vec<License>`. Note that if not built with `feature = "third_party_bundle"` this
/// macro always returns an empty vec.
#[macro_export]
#[cfg(not(feature = "third_party_bundle"))]
macro_rules! include_bundle {
    () => {
        $crate::include_bundle!(concat!(env!("OUT_DIR"), "/zng-third-licenses.bin"))
    };
    ($custom_name:expr) => {{
        Vec::<$crate::third_party::License>::new()
    }};
}

#[doc(inline)]
pub use crate::include_bundle;

#[cfg(feature = "third_party_bundle")]
#[doc(hidden)]
pub fn decode_licenses(bin: &[u8]) -> Vec<License> {
    let bin = inflate::inflate_bytes(bin).expect("invalid bundle deflate binary");
    bincode::deserialize(&bin).expect("invalid bundle bincode binary")
}
