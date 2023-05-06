//! File system events and service.

use std::{fs::File, io, path::PathBuf};

use crate::{handler::AppHandler, var::*};

/// File system watcher service.
pub struct FS;

impl FS {
    /// Enable file change events for files inside `dir`, also include inner directories if `recursive` is `true`.
    ///
    /// Returns a handle that will stop the dir watch when dropped, if there is no other active handler for the same directory.
    ///
    /// The directory will be watched using an OS specific efficient watcher provided by the [`notify`] crate. If there is
    /// any error creating the watcher, such as if the directory does not exist yet a slower polling watcher will retry periodically
    /// until the efficient watcher can be created or the handle is dropped.
    pub fn watch(&self, dir: impl Into<PathBuf>, recursive: bool) {
        todo!("!!: HANDLE")
    }

    /// Read a file into a variable, the `init` value will start the variable and the `read` closure will be called
    /// every time the file changes, if the closure returns `Some(O)` the variable updates with the new value.
    ///
    /// Dropping the variable drops the read watch. The `read` closure is non-blocking, it is called in a [`task::wait`]
    /// background thread, the result is FIFO, the output of a slow read is ignored if a more recent output is already
    /// set in the variable.
    pub fn read_file<O: VarValue>(
        &self,
        file: impl Into<PathBuf>,
        init: O,
        read: impl FnMut(io::Result<File>) -> Option<O> + Send + 'static,
    ) -> ReadOnlyArcVar<O> {
        todo!("!!: impl")
    }

    /// Watch a `dir` and calls `handler` when any change in the dir is detected.
    ///
    /// Note that the `handler` is blocking, use an async handler like [`async_app_hn!`] and offload IO to [`task::wait`] to
    /// avoid blocking the app.
    pub fn on_dir_changed(&self, dir: impl Into<PathBuf>, recursive: bool, handler: impl AppHandler<()>) {
        let handle = self.watch(dir, recursive);
    }

    /// Watch a `file` and calls `handler` when any change in the file is detected.
    ///
    /// Note that the `handler` is blocking, use an async handler like [`async_app_hn!`] and offload IO to [`task::wait`] to
    /// avoid blocking the app.
    pub fn on_file_changed(&self, file: impl Into<PathBuf>, handler: impl AppHandler<()>) {
        todo!("!!: impl");
    }
}
