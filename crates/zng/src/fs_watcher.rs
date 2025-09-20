#![cfg(feature = "fs_watcher")]

//! File system watcher service and other types.
//!
//! The [`WATCHER`] service can be used to get notifications when a file or directory is modified. It also provides
//! ways to bind a file to a variable, automatically synchronizing both.
//!
//! The example below binds the current content of a text file to at text variable using [`WATCHER.read`](WATCHER::read).
//! Any external change made to the text file updates the UI text.
//!
//! ```
//! use zng::{fs_watcher::WATCHER, prelude::*};
//!
//! # fn main() { }
//! # fn demo() {
//! # let _ =
//! Text!(WATCHER.read("dump.log", Txt::from(""), |f| f.ok()?.text().ok()))
//! # ; }
//! ```
//!
//! The next example created a read-write binding with the text file, any external change made to the text file updates the
//! `TextInput!` and any change made using the `TextInput!` updates the file contents.
//!
//! ```
//! use zng::{fs_watcher::WATCHER, prelude::*};
//!
//! # fn main() { }
//! # fn demo() {
//! # let _ =
//! TextInput!(zng::fs_watcher::WATCHER.sync(
//!     "dump.log",
//!     // initial value
//!     Txt::from(""),
//!     // read, only updates txt if returns Some
//!     |f| f.ok()?.text().ok(),
//!     // write, only change file if commit called.
//!     |txt, f| {
//!         if let Ok(mut f) = f {
//!             if f.write_text(&txt).is_ok() {
//!                 // replace actual file with temp that was successfully written.
//!                 let _ = f.commit();
//!             } else {
//!                 f.cancel();
//!             }
//!         }
//!     },
//! ))
//! # ; }
//! ```
//!
//! The [`WATCHER`] service abstracts away most of the headache of interacting with the file system. This service
//! is used internally by the implementations of [`CONFIG`] and [`L10N`].
//!
//! [`CONFIG`]: crate::config::CONFIG
//! [`L10N`]: crate::l10n::L10N
//!
//! # Full API
//!
//! See [`zng_ext_fs_watcher`] for the full watcher API.

pub use zng_ext_fs_watcher::{
    FS_CHANGES_EVENT, FsChange, FsChangeNote, FsChangeNoteHandle, FsChangesArgs, WATCHER, WatchFile, WatcherHandle, WatcherReadStatus,
    WatcherSyncStatus, WriteFile, fs_event,
};
