#![cfg(feature = "shortcut_text")]

//! Keyboard shortcut display widget.
//!
//! !!: TODO
//!
//! # Full API
//!
//! See [`zng_wgt_shortcut`] for the full widget API.

pub use zng_wgt_shortcut::{
    ShortcutText, chord_separator_fn, first_n, key_fn, key_gesture_fn, key_gesture_separator_fn, modifier_fn, panel_fn, shortcut_fn,
    shortcuts_separator_fn,
};
