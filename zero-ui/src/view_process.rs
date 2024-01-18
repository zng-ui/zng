//! View process implementations.

/// Default view-process implementation.
///
/// # Full API
///
/// See [`zero_ui_view`] for the full API.
#[cfg(feature = "view")]
pub mod default {
    pub use zero_ui_view::{init, run_same_process};
}

/// Default view-process implementation as an embedded precompiled binary.
///
/// # Full API
///
/// See [`zero_ui_view_prebuilt`] and [`zero_ui_view`] for the full API.
#[cfg(feature = "view_prebuilt")]
pub mod prebuilt {
    pub use zero_ui_view_prebuilt::{init, run_same_process};
}
