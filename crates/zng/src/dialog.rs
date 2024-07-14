//! Modal dialog overlay widget and service.
//!
//! !!: TODO example
//!
//! # Full API
//!
//! See [`zng_wgt_dialog`] for the full view API.

pub use zng_wgt_dialog::{
    native_dialogs, AskStyle, ConfirmStyle, DefaultStyle, Dialog, DialogButtonArgs, DialogKind, ErrorStyle, FileDialogFilters,
    FileDialogResponse, InfoStyle, Response, Responses, WarnStyle, DIALOG,
};

/// Modal dialog parent widget that fills the window.
///
/// Note that the actual [`DialogBackdrop!`] widget is not included in this module because it is instantiated by the [`DIALOG`] service.
/// The backdrop can be customized by setting the [`backdrop::style_fn`].
///
/// [`DialogBackdrop!`]: struct@zng_wgt_dialog::backdrop::DialogBackdrop
/// [`backdrop::style_fn`]: fn@crate::dialog::backdrop::style_fn
/// [`DIALOG`]: fn@crate::dialog::DIALOG
///
/// !!: TODO example
///
/// # Full API
///
/// See [`zng_wgt_dialog::backdrop`] for the fill view API.
pub mod backdrop {
    pub use zng_wgt_dialog::backdrop::{close_on_click, style_fn, DefaultStyle};
}
