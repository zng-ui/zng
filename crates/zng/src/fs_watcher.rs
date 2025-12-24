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

#[cfg(feature = "image")]
mod images_ext {
    use std::path::PathBuf;

    use zng_app::hn;
    use zng_ext_fs_watcher::WATCHER;
    use zng_ext_image::*;

    /// File watcher extensions for [`IMAGES`] service.
    #[expect(non_camel_case_types)]
    pub trait IMAGES_Ext {
        /// Like [`IMAGES.read`] with automatic reload when the file at `path` is modified.
        ///
        /// [`IMAGES.read`]: IMAGES::read
        fn watch(&self, path: impl Into<PathBuf>) -> ImageVar;

        /// Like [`IMAGES.image`] with automatic cache reload when the file at `path` is modified.
        ///
        /// [`IMAGES.image`]: IMAGES::image
        fn watch_image(
            &self,
            path: impl Into<PathBuf>,
            limits: Option<ImageLimits>,
            downscale: Option<ImageDownscaleMode>,
            mask: Option<ImageMaskMode>,
            entries: ImageEntriesMode,
        ) -> ImageVar;
    }
    impl IMAGES_Ext for IMAGES {
        fn watch(&self, path: impl Into<PathBuf>) -> ImageVar {
            watch(path.into())
        }

        fn watch_image(
            &self,
            path: impl Into<PathBuf>,
            limits: Option<ImageLimits>,
            downscale: Option<ImageDownscaleMode>,
            mask: Option<ImageMaskMode>,
            entries: ImageEntriesMode,
        ) -> ImageVar {
            watch_image(path.into(), limits, downscale, mask, entries)
        }
    }

    fn watch(path: PathBuf) -> ImageVar {
        let img = IMAGES.read(path.clone());
        let handle = WATCHER.on_file_changed(
            path.clone(),
            hn!(|_| {
                let _ = IMAGES.reload(path.clone());
            }),
        );
        img.hold(handle).perm();
        img
    }

    fn watch_image(
        path: PathBuf,
        limits: Option<ImageLimits>,
        downscale: Option<ImageDownscaleMode>,
        mask: Option<ImageMaskMode>,
        entries: ImageEntriesMode,
    ) -> ImageVar {
        let img = IMAGES.image(
            ImageSource::Read(path.clone()),
            ImageCacheMode::Cache,
            limits.clone(),
            downscale.clone(),
            mask,
            entries,
        );
        let handle = WATCHER.on_file_changed(
            path.clone(),
            hn!(|_| {
                let _ = IMAGES.image(
                    ImageSource::Read(path.clone()),
                    ImageCacheMode::Reload,
                    limits.clone(),
                    downscale.clone(),
                    mask,
                    entries,
                );
            }),
        );
        img.hold(handle).perm();
        img
    }
}
#[cfg(feature = "image")]
pub use images_ext::*;
