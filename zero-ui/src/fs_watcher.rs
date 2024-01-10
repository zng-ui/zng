//! File system watcher service and types.
//!
//! # Full API
//!
//! See [`zero_ui_ext_fs_watcher`] for the full watcher API.

pub use zero_ui_ext_fs_watcher::{
    FsChange, FsChangeNote, FsChangeNoteHandle, FsChangesArgs, WatchFile, WatcherHandle, WatcherReadStatus, WatcherSyncStatus,
    WatcherSyncWriteNote, WriteFile, FS_CHANGES_EVENT, WATCHER,
};
