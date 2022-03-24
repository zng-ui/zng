#![cfg(feature = "post")]

//! Static post-processing.
//!
//! Implements the same transformations that the dynamic scripts in `/js` do, but applied
//! directly to the docs directory as a custom post-process.

mod macro_;
mod property;
mod script;
mod util;
mod widget;

/// Transform HTML of all crates in the docs directory.
pub fn transform(docs_root: impl AsRef<std::path::Path>) {
    let docs_root = docs_root.as_ref();

    macro_::transform(docs_root);
    property::transform(docs_root);
    script::transform(docs_root);
}
