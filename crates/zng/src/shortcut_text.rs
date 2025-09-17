#![cfg(feature = "shortcut_text")]

//! Keyboard shortcut display widget.
//!
//! The [`ShortcutText!`] is composite widget that generates localized and styled shortcut *text*, it
//! can handle all shortcut variations, multiple shortcuts and partially invalid shortcuts.
//! Extensive configuration is possible using contextual properties that can override.
//!
//! The example below demonstrates a basic *key binding editor* that uses the [`ShortcutText!`] widget in multiple places.
//!
//! ```
//! # use zng::focus::{focus_on_init, focusable};
//! # use zng::gesture::Shortcuts;
//! # use zng::keyboard::{Key, KeyInputArgs, on_pre_key_down};
//! # use zng::layout::{align, min_height};
//! # use zng::prelude::*;
//! # use zng::shortcut_text::ShortcutText;
//! #
//! pub fn shortcut_input(shortcut: Var<Shortcuts>) -> UiNode {
//!     Button! {
//!         // display the shortcut, or the `none_fn` content if there is no shortcut.
//!         child = ShortcutText! {
//!             shortcut = shortcut.clone();
//!             none_fn = wgt_fn!(|_| Text!("no shortcut"));
//!         };
//!         on_click = hn!(|_| {
//!             DIALOG.custom(shortcut_input_dialog(shortcut.clone()));
//!         });
//!     }
//! }
//! fn shortcut_input_dialog(output: Var<gesture::Shortcuts>) -> UiNode {
//!     let pressed = var(Shortcuts::new());
//!     Container! {
//!         child_top =
//!             Wrap!(ui_vec![
//!                 Text!("Press the new shortcut and then press "),
//!                 ShortcutText!(shortcut!(Enter)), // shortcut text supports inlining
//!             ]),
//!             20,
//!         ;
//!         // default style is derived from the `font_size` and `font_color` values.
//!         child = ShortcutText! {
//!             shortcut = pressed.clone();
//!             font_size = 3.em();
//!             align = Align::TOP;
//!         };
//!
//!         on_pre_key_down = hn!(|args: &KeyInputArgs| {
//!             args.propagation().stop();
//!             match &args.key {
//!                 Key::Enter => {
//!                     let shortcut = pressed.get();
//!                     if shortcut.is_empty() || shortcut[0].is_valid() {
//!                         is_valid.set(true);
//!                         output.set(shortcut);
//!                         DIALOG.respond(dialog::Response::ok());
//!                     } else {
//!                         is_valid.set(false);
//!                     }
//!                 }
//!                 Key::Escape => {
//!                     DIALOG.respond(dialog::Response::cancel());
//!                 }
//!                 _ => {
//!                     is_valid.set(true); // clear
//!                     pressed.set(args.editing_shortcut().unwrap());
//!                 }
//!             }
//!         });
//!         align = Align::CENTER;
//!         min_height = 200;
//!         focusable = true;
//!         focus_on_init = true;
//!     }
//! }
//! ```
//!
//! [`ShortcutText!`]: struct@ShortcutText
//!
//! # Full API
//!
//! See [`zng_wgt_shortcut`] for the full widget API.

pub use zng_wgt_shortcut::{
    ShortcutText, chord_separator_fn, first_n, key_fn, key_gesture_fn, key_gesture_separator_fn, key_txt, keycap, modifier_fn,
    modifier_txt, panel_fn, shortcut_fn, shortcuts_separator_fn,
};
