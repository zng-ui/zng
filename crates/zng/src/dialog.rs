#![cfg(feature = "dialog")]

//! Message dialogs, file and folder dialogs, notifications.
//!
//! The [`DIALOG`] service provides custom and modal native dialogs.
//!
//! ```
//! use zng::prelude::*;
//!
//! # fn example() {
//! # let _ =
//! Button! {
//!     child = Text!("Info, Warn, Error");
//!     on_click = async_hn!(|_| {
//!         DIALOG.info("Info", "Information message.").wait_rsp().await;
//!         DIALOG.warn("Warn", "Warning message.").wait_rsp().await;
//!         DIALOG.error("Error", "Error message.").wait_rsp().await;
//!     });
//!     // dialog::native_dialogs = true;
//! }
//! # ; }
//! ```
//!
//! The example above shows 3 custom dialogs in sequence, info, warn and error. If `dialog::native_dialogs = true` is uncommented
//! the example shows 3 native dialogs.
//!
//! Custom dialogs modal widgets, rendered in the window content, instantiated using the [`Dialog!`] widget.
//!
//! ```
//! use zng::prelude::*;
//!
//! # async fn _demo() {
//! let r = DIALOG
//!     .custom(dialog::Dialog! {
//!         style_fn = dialog::WarnStyle!();
//!         title = Text!(l10n!("save-dlg.title", "Save File?"));
//!         content = SelectableText!(l10n!(
//!             "save-dlg.msg",
//!             "Save file? All unsaved changes will be lost."
//!         ));
//!         responses = vec![
//!             dialog::Response::cancel(),
//!             dialog::Response::new("discard", l10n!("save-dlg.discard", "Discard")),
//!             dialog::Response::new("save", l10n!("save-dlg.save", "Save")),
//!         ];
//!     })
//!     .wait_rsp()
//!     .await;
//! if r.name == "save" {
//!     // save
//! }
//! # }
//! ```
//!
//! The example above creates a custom dialog based on the warning dialog (`WarnStyle!`), it uses custom responses that are
//! identified by name.
//!
//! Some of the dialogs provided are native by default (and only native on this release), the example below shows a native save file dialog:
//!
//! ```
//! use zng::prelude::*;
//!
//! # async fn _demo() {
//! let mut f = dialog::FileDialogFilters::default();
//! f.push_filter("Text Files", ["txt", "md"]);
//! f.push_filter("Text File", ["txt"]);
//! f.push_filter("Markdown File", ["md"]);
//! f.push_filter("All Files", ["*"]);
//! let filters = f;
//!
//! let r = DIALOG
//!     .save_file("Save Text", "last/save/dir", "last-name.txt", filters)
//!     .wait_rsp()
//!     .await
//!     .into_path();
//!
//! if let Ok(Some(path)) = r {
//!     std::fs::write(path, "contents".as_bytes()).unwrap();
//! }
//! # }
//! ```
//!
//! # Local Notifications
//!
//! The service can also insert notifications in the system notification list.
//!
//! ```
//! use zng::prelude::*;
//!
//! # async fn _demo(){
//! let mut note = dialog::Notification::new("Test", "Test notification.");
//! // optional *dismiss actions*
//! note.push_action("action-1", "Action 1");
//! note.push_action("action-2", "Action 2");
//! // optional system cleanup timeout suggestion.
//! note.timeout = Some(3.hours());
//!
//! // as explicit var to support updates
//! let note = var(note);
//!
//! // insert the notification
//! let r = DIALOG.notification(note.clone());
//!
//! // set a handler in case the notification is responded.
//! r.on_rsp(hn!(|response| {
//!     println!("Notification response: {response:?}");
//! })).perm();
//!
//! // await for an explicit response up to 10 minutes.
//! task::with_deadline(r.wait_done(), 10.minutes()).await;
//! if r.is_waiting() {
//!     // update the notification with a close request.
//!     note.set(dialog::Notification::close());
//! }
//! # }
//! ```
//!
//! The example above inserts a notification with two dismiss actions and sets a handler in case the notification is ever responded.
//!
//! Note that the notification is a variable, it can be updated and closed. Also note that care is taken to not keep awaiting for a response
//! forever, notifications are not intrusive like other dialogs, users may never respond, the operating system may also ignore the timeout suggestion.
//!
//! [`Dialog!`]: struct@Dialog
//!
//! # Full API
//!
//! See [`zng_wgt_dialog`] for the full view API.

pub use zng_wgt_dialog::{
    AskStyle, ConfirmStyle, DIALOG, DefaultStyle, Dialog, DialogButtonArgs, DialogKind, ErrorStyle, FileDialogFilters, FileDialogResponse,
    InfoStyle, Notification, NotificationAction, NotificationResponse, Response, Responses, WarnStyle, ask_style_fn, confirm_style_fn,
    error_style_fn, info_style_fn, native_dialogs, warn_style_fn,
};

/// Modal dialog parent widget that fills the window.
///
/// Note that the actual [`DialogBackdrop!`] widget is not included in this module because it is instantiated by the [`DIALOG`] service.
/// The backdrop can be customized by setting the [`backdrop::style_fn`].
///
/// ```
/// use zng::prelude::*;
///
/// # fn example() {
/// # let _ =
/// Window! {
///     dialog::backdrop::style_fn = Style! {
///         replace = true;
///         color::filter::backdrop_blur = 2;
///     };
/// }
/// # ; }
/// ```
///
/// The example above configures the backdrop to blur the window content when any dialog is open.
///
/// [`DialogBackdrop!`]: struct@zng_wgt_dialog::backdrop::DialogBackdrop
/// [`backdrop::style_fn`]: fn@crate::dialog::backdrop::style_fn
/// [`DIALOG`]: crate::dialog::DIALOG
///
/// # Full API
///
/// See [`zng_wgt_dialog::backdrop`] for the fill view API.
pub mod backdrop {
    pub use zng_wgt_dialog::backdrop::{DefaultStyle, style_fn};
}
