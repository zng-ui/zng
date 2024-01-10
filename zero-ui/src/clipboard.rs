//! Clipboard service, commands and types.
//!
//! # Full API
//!
//! See [`zero_ui_ext_clipboard`] for the full clipboard API.

pub use zero_ui_ext_clipboard::{ClipboardError, CLIPBOARD, COPY_CMD, CUT_CMD, PASTE_CMD};
pub use zero_ui_wgt_input::cmd::{on_copy, on_cut, on_paste, on_pre_copy, on_pre_cut, on_pre_paste};
