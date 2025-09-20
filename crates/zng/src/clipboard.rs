#![cfg(feature = "clipboard")]

//! Clipboard service, commands and other types.
//!
//! This module provides the [`CLIPBOARD`] service and clipboard related commands and command handlers.
//! The service does not implement the commands, widgets implement the commands and optionally use the service.
//!
//! Note that the [`CLIPBOARD`] service uses the view-process the interact with the system clipboard, so it will only
//! work if a headed app or headless app with renderer is running.
//!
//! # Text
//!
//! The example below uses the service to copy text to the clipboard:
//!
//! ```
//! use zng::prelude::*;
//!
//! # fn example() {
//! let txt = var(Txt::from(""));
//! let copied = var(false);
//! # let _ =
//! Container! {
//!     child = TextInput!(txt.clone());
//!     child_end =
//!         Button! {
//!             child = Text!(copied.map(|&c| if !c { "Copy" } else { "Copied!" }.into()));
//!             on_click = async_hn!(txt, copied, |_| {
//!                 if zng::clipboard::CLIPBOARD.set_text(txt.get()).wait_rsp().await.is_ok() {
//!                     copied.set(true);
//!                 }
//!             });
//!         },
//!         4,
//!     ;
//! }
//! # ; }
//! ```
//!
//! The `TextInput` widget also implements the clipboard commands, the example below requests clipboard paste to the
//! text input, that widget uses the clipboard service to get the text.
//!
//! ```
//! use zng::prelude::*;
//!
//! # fn example() {
//! # let _ =
//! Container! {
//!     child = TextInput! {
//!         id = "input-1";
//!         txt = var(Txt::from(""));
//!     };
//!     child_end = Button!(zng::clipboard::PASTE_CMD.scoped(WidgetId::named("input-1"))), 4;
//! }
//! # ; }
//! ```
//!
//! # File List
//!
//! The example below modifies the paste button to paste file paths, the paths can be used to read or move
//! the each file, in the example they are converted to a text list.
//!
//! ```
//! use zng::prelude::*;
//!
//! # fn example() {
//! # let txt = var(Txt::from(""));
//! # let _ =
//! Button! {
//!     child = Text!("Paste");
//!     on_click = hn!(|_| {
//!         if let Ok(Some(f)) = zng::clipboard::CLIPBOARD.file_list() {
//!             txt.modify(move |txt| {
//!                 let txt = txt.to_mut();
//!                 txt.clear();
//!                 for f in f {
//!                     use std::fmt::Write as _;
//!                     let _ = writeln!(txt, "{}", f.display());
//!                 }
//!             });
//!         }
//!     });
//! }
//! # ; }
//! ```
//!
//! # Image
//!
//! The example below pastes an image from the clipboard. The example also demonstrates how to separate the
//! paste button from the paste action, the button only needs to know that the window handles the paste command,
//! the window implements the paste by setting an image variable.
//!
//! ```
//! use zng::clipboard;
//! use zng::prelude::*;
//!
//! # let mut app = APP.defaults().run_headless(false);
//! # app.doc_test_window(async {
//! let img_source = var(ImageSource::flood(layout::PxSize::splat(layout::Px(1)), colors::BLACK, None));
//! Window! {
//!     # widget::on_init = hn_once!(|_| {WINDOW.close();});
//!     child_top = Button!(clipboard::PASTE_CMD.scoped(WINDOW.id())), 0;
//!     child = Image!(img_source.clone());
//!     clipboard::on_paste = hn!(|_| {
//!         if let Ok(Some(img)) = clipboard::CLIPBOARD.image() {
//!             img_source.set(img);
//!         }
//!     });
//! }
//! # });
//! ```
//!
//! # Full API
//!
//! See [`zng_ext_clipboard`] for the full clipboard API.

pub use zng_ext_clipboard::{CLIPBOARD, COPY_CMD, CUT_CMD, ClipboardError, PASTE_CMD};
pub use zng_wgt_input::cmd::{can_copy, can_cut, can_paste, on_copy, on_cut, on_paste, on_pre_copy, on_pre_cut, on_pre_paste};
