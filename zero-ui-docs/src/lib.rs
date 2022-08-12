//! Rust-doc extensions that customize `zero-ui` widget and property pages.
//!
//! # Usage
//!
//! Modify the `Cargo.toml` of the target crate:
//!
//! ```toml
//! [build-dependencies]
//! zero-ui-docs = "0.1"
//!
//! [package.metadata.docs.rs]
//! rustdoc-args = ["--html-in-header", "target/doc/zero-ui-extensions.html"]
//! ```
//!
//! In the crate build script (`build.rs`) add:
//!
//! ```
//! zero_ui_docs::html_in_header();
//! ```
//!
//! This enables the custom widget and property items.
//!
//! # Widget & Property Helpers
//!
//! These helpers are enabled automatically, new sections are created for widget modules and property functions, the
//! item pages are also modified to
//!
//! # Macro Placement Helper
//!
//! Macros declared with `macro_rules!` are placed at the root of the crate, you can re-export and inline docs in a module
//! but it will still show at the docs front page. To remove a macro item from the front page add `<span data-del-macro-root></span>`
//! at the start of the macro documentation.

use std::{env, fs, path::PathBuf};

mod rs;

#[cfg(feature = "post")]
pub use rs::transform;

macro_rules! include_js {
    ($name:tt) => {
        concat!(
            "<script data-zero-ui-dyn='",
            $name,
            "'>",
            include_str!(concat!(env!("OUT_DIR"), "/js_min/", $name)),
            "</script>"
        )
    };
}
macro_rules! include_css {
    ($name:tt) => {
        concat!(
            "<style data-zero-ui-dyn='",
            $name,
            "'>",
            include_str!(concat!(env!("OUT_DIR"), "/css_min/", $name)),
            "</style>"
        )
    };
}

/// Aggregate all dynamic customization scripts in a HTML snippet that can be written to a file
/// and used as the `--html-in-header`.
pub fn html() -> &'static str {
    concat!(
        include_js!("macro.js"),
        include_js!("property.js"),
        include_js!("widget.js"),
        include_js!("sidebar.js"),
        include_css!("widget.css"),
    )
}

/// Writes the [`html`] to "target/doc/zero-ui-extensions.html".
///
/// Returns the path.
pub fn html_in_header() -> PathBuf {
    let file = doc_dir().join("zero-ui-extensions.html");
    fs::write(&file, html()).unwrap();
    file
}

fn doc_dir() -> PathBuf {
    let out_dir = PathBuf::from(env!("OUT_DIR")).canonicalize().unwrap();

    let mut dir = out_dir.parent().unwrap();
    while dir.file_name().unwrap() != "target" {
        dir = dir.parent().expect("failed to get 'target' dir from `OUT_DIR`");
    }
    let dir = dir.join("doc");
    fs::create_dir_all(&dir).unwrap();

    dir
}
