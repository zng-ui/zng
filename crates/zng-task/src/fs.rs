//! Async filesystem primitives.
//!
//! This module is the [async-fs](https://docs.rs/async-fs) crate re-exported for convenience.
//!

#[doc(inline)]
pub use async_fs::*;

//
// Module contains patched version of File
// TODO(breaking) replace with reexport again after  https://github.com/smol-rs/async-fs/pull/55 is released
//

use std::fmt;
use std::future::Future;
use std::io::{self, SeekFrom};
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt as _;

#[cfg(windows)]
use std::os::windows::fs::OpenOptionsExt as _;

use async_lock::Mutex;
use blocking::{Unblock, unblock};
use futures_lite::io::{AsyncRead, AsyncSeek, AsyncWrite, AsyncWriteExt};
use futures_lite::ready;

#[doc(no_inline)]
pub use std::fs::{FileType, Metadata, Permissions};

/// An open file on the filesystem.
///
/// Depending on what options the file was opened with, this type can be used for reading and/or
/// writing.
///
/// Files are automatically closed when they get dropped and any errors detected on closing are
/// ignored. Use the [`sync_all()`][`File::sync_all()`] method before dropping a file if such
/// errors need to be handled.
///
/// **NOTE:** If writing to a file, make sure to call
/// [`flush()`][`futures_lite::io::AsyncWriteExt::flush()`], [`sync_data()`][`File::sync_data()`],
/// or [`sync_all()`][`File::sync_all()`] before dropping the file or else some written data
/// might get lost!
///
/// # Examples
///
/// Create a new file and write some bytes to it:
///
/// ```no_run
/// use futures_lite::io::AsyncWriteExt;
/// use zng_task::fs::File;
///
/// # futures_lite::future::block_on(async {
/// let mut file = File::create("a.txt").await?;
///
/// file.write_all(b"Hello, world!").await?;
/// file.flush().await?;
/// # std::io::Result::Ok(()) });
/// ```
///
/// Read the contents of a file into a vector of bytes:
///
/// ```no_run
/// use futures_lite::io::AsyncReadExt;
/// use zng_task::fs::File;
///
/// # futures_lite::future::block_on(async {
/// let mut file = File::open("a.txt").await?;
///
/// let mut contents = Vec::new();
/// file.read_to_end(&mut contents).await?;
/// # std::io::Result::Ok(()) });
/// ```
pub struct File {
    /// Always accessible reference to the file.
    file: Arc<std::fs::File>,

    /// Performs blocking I/O operations on a thread pool.
    unblock: Mutex<Unblock<ArcFile>>,

    /// Logical file cursor, tracked when reading from the file.
    ///
    /// This will be set to an error if the file is not seekable.
    read_pos: Option<io::Result<u64>>,

    /// Set to `true` if the file needs flushing.
    is_dirty: bool,
}

impl File {
    /// Creates an async file from a blocking file.
    fn new(inner: std::fs::File, is_dirty: bool) -> File {
        let file = Arc::new(inner);
        let unblock = Mutex::new(Unblock::new(ArcFile(file.clone())));
        let read_pos = None;
        File {
            file,
            unblock,
            read_pos,
            is_dirty,
        }
    }

    /// Opens a file in read-only mode.
    ///
    /// See the [`OpenOptions::open()`] function for more options.
    ///
    /// # Errors
    ///
    /// An error will be returned in the following situations:
    ///
    /// * `path` does not point to an existing file.
    /// * The current process lacks permissions to read the file.
    /// * Some other I/O error occurred.
    ///
    /// For more details, see the list of errors documented by [`OpenOptions::open()`].
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use zng_task::fs::File;
    ///
    /// # futures_lite::future::block_on(async {
    /// let file = File::open("a.txt").await?;
    /// # std::io::Result::Ok(()) });
    /// ```
    pub async fn open<P: AsRef<Path>>(path: P) -> io::Result<File> {
        let path = path.as_ref().to_owned();
        let file = unblock(move || std::fs::File::open(path)).await?;
        Ok(File::new(file, false))
    }

    /// Opens a file in write-only mode.
    ///
    /// This method will create a file if it does not exist, and will truncate it if it does.
    ///
    /// See the [`OpenOptions::open`] function for more options.
    ///
    /// # Errors
    ///
    /// An error will be returned in the following situations:
    ///
    /// * The file's parent directory does not exist.
    /// * The current process lacks permissions to write to the file.
    /// * Some other I/O error occurred.
    ///
    /// For more details, see the list of errors documented by [`OpenOptions::open()`].
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use zng_task::fs::File;
    ///
    /// # futures_lite::future::block_on(async {
    /// let file = File::create("a.txt").await?;
    /// # std::io::Result::Ok(()) });
    /// ```
    pub async fn create<P: AsRef<Path>>(path: P) -> io::Result<File> {
        let path = path.as_ref().to_owned();
        let file = unblock(move || std::fs::File::create(path)).await?;
        Ok(File::new(file, false))
    }

    /// Synchronizes OS-internal buffered contents and metadata to disk.
    ///
    /// This function will ensure that all in-memory data reaches the filesystem.
    ///
    /// This can be used to handle errors that would otherwise only be caught by closing the file.
    /// When a file is dropped, errors in synchronizing this in-memory data are ignored.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use futures_lite::io::AsyncWriteExt;
    /// use zng_task::fs::File;
    ///
    /// # futures_lite::future::block_on(async {
    /// let mut file = File::create("a.txt").await?;
    ///
    /// file.write_all(b"Hello, world!").await?;
    /// file.sync_all().await?;
    /// # std::io::Result::Ok(()) });
    /// ```
    pub async fn sync_all(&self) -> io::Result<()> {
        let mut inner = self.unblock.lock().await;
        inner.flush().await?;
        let file = self.file.clone();
        unblock(move || file.sync_all()).await
    }

    /// Synchronizes OS-internal buffered contents to disk.
    ///
    /// This is similar to [`sync_all()`][`File::sync_all()`], except that file metadata may not
    /// be synchronized.
    ///
    /// This is intended for use cases that must synchronize the contents of the file, but don't
    /// need the file metadata synchronized to disk.
    ///
    /// Note that some platforms may simply implement this in terms of
    /// [`sync_all()`][`File::sync_all()`].
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use futures_lite::io::AsyncWriteExt;
    /// use zng_task::fs::File;
    ///
    /// # futures_lite::future::block_on(async {
    /// let mut file = File::create("a.txt").await?;
    ///
    /// file.write_all(b"Hello, world!").await?;
    /// file.sync_data().await?;
    /// # std::io::Result::Ok(()) });
    /// ```
    pub async fn sync_data(&self) -> io::Result<()> {
        let mut inner = self.unblock.lock().await;
        inner.flush().await?;
        let file = self.file.clone();
        unblock(move || file.sync_data()).await
    }

    /// Truncates or extends the file.
    ///
    /// If `size` is less than the current file size, then the file will be truncated. If it is
    /// greater than the current file size, then the file will be extended to `size` and have all
    /// intermediate data filled with zeros.
    ///
    /// The file's cursor stays at the same position, even if the cursor ends up being past the end
    /// of the file after this operation.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use zng_task::fs::File;
    ///
    /// # futures_lite::future::block_on(async {
    /// let mut file = File::create("a.txt").await?;
    /// file.set_len(10).await?;
    /// # std::io::Result::Ok(()) });
    /// ```
    pub async fn set_len(&self, size: u64) -> io::Result<()> {
        let mut inner = self.unblock.lock().await;
        inner.flush().await?;
        let file = self.file.clone();
        unblock(move || file.set_len(size)).await
    }

    /// Reads the file's metadata.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use zng_task::fs::File;
    ///
    /// # futures_lite::future::block_on(async {
    /// let file = File::open("a.txt").await?;
    /// let metadata = file.metadata().await?;
    /// # std::io::Result::Ok(()) });
    /// ```
    pub async fn metadata(&self) -> io::Result<Metadata> {
        let file = self.file.clone();
        unblock(move || file.metadata()).await
    }

    /// Changes the permissions on the file.
    ///
    /// # Errors
    ///
    /// An error will be returned in the following situations:
    ///
    /// * The current process lacks permissions to change attributes on the file.
    /// * Some other I/O error occurred.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use zng_task::fs::File;
    ///
    /// # futures_lite::future::block_on(async {
    /// let file = File::create("a.txt").await?;
    ///
    /// let mut perms = file.metadata().await?.permissions();
    /// perms.set_readonly(true);
    /// file.set_permissions(perms).await?;
    /// # std::io::Result::Ok(()) });
    /// ```
    pub async fn set_permissions(&self, perm: Permissions) -> io::Result<()> {
        let file = self.file.clone();
        unblock(move || file.set_permissions(perm)).await
    }

    /// Repositions the cursor after reading.
    ///
    /// When reading from a file, actual file reads run asynchronously in the background, which
    /// means the real file cursor is usually ahead of the logical cursor, and the data between
    /// them is buffered in memory. This kind of buffering is an important optimization.
    ///
    /// After reading ends, if we decide to perform a write or a seek operation, the real file
    /// cursor must first be repositioned back to the correct logical position.
    fn poll_reposition(&mut self, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        if let Some(Ok(read_pos)) = self.read_pos {
            ready!(Pin::new(self.unblock.get_mut()).poll_seek(cx, SeekFrom::Start(read_pos)))?;
        }
        self.read_pos = None;
        Poll::Ready(Ok(()))
    }

    /// Returns the inner blocking file, if no task is running.
    ///
    /// This will flush any pending data I/O tasks before attempting to unwrap, it will fail
    /// if there are pending metadata tasks. Note that dropping futures does not cancel file
    /// tasks, you must await all pending futures for this conversion to succeed.
    pub async fn try_unwrap(self) -> Result<std::fs::File, Self> {
        // flush Unblock and drop its reference
        let _ = self.unblock.into_inner().into_inner().await;

        match Arc::try_unwrap(self.file) {
            Ok(ready) => Ok(ready),
            Err(pending) => {
                // task associated with dropped future is still running
                Err(Self {
                    file: pending.clone(),
                    unblock: Mutex::new(Unblock::new(ArcFile(pending))),
                    is_dirty: false,
                    read_pos: self.read_pos,
                })
            }
        }
    }
}

impl fmt::Debug for File {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.file.fmt(f)
    }
}

impl From<std::fs::File> for File {
    fn from(inner: std::fs::File) -> File {
        File::new(inner, true)
    }
}

#[cfg(unix)]
impl std::os::unix::io::AsRawFd for File {
    fn as_raw_fd(&self) -> std::os::unix::io::RawFd {
        self.file.as_raw_fd()
    }
}

#[cfg(windows)]
impl std::os::windows::io::AsRawHandle for File {
    fn as_raw_handle(&self) -> std::os::windows::io::RawHandle {
        self.file.as_raw_handle()
    }
}

#[cfg(unix)]
impl From<std::os::unix::io::OwnedFd> for File {
    fn from(fd: std::os::unix::io::OwnedFd) -> Self {
        File::from(std::fs::File::from(fd))
    }
}

#[cfg(windows)]
impl From<std::os::windows::io::OwnedHandle> for File {
    fn from(fd: std::os::windows::io::OwnedHandle) -> Self {
        File::from(std::fs::File::from(fd))
    }
}

#[cfg(unix)]
impl std::os::unix::io::AsFd for File {
    fn as_fd(&self) -> std::os::unix::io::BorrowedFd<'_> {
        self.file.as_fd()
    }
}

#[cfg(windows)]
impl std::os::windows::io::AsHandle for File {
    fn as_handle(&self) -> std::os::windows::io::BorrowedHandle<'_> {
        self.file.as_handle()
    }
}

impl AsyncRead for File {
    fn poll_read(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<io::Result<usize>> {
        // Before reading begins, remember the current cursor position.
        if self.read_pos.is_none() {
            // Initialize the logical cursor to the current position in the file.
            self.read_pos = Some(ready!(self.as_mut().poll_seek(cx, SeekFrom::Current(0))));
        }

        let n = ready!(Pin::new(self.unblock.get_mut()).poll_read(cx, buf))?;

        // Update the logical cursor if the file is seekable.
        if let Some(Ok(pos)) = self.read_pos.as_mut() {
            *pos += n as u64;
        }

        Poll::Ready(Ok(n))
    }
}

impl AsyncWrite for File {
    fn poll_write(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
        ready!(self.poll_reposition(cx))?;
        self.is_dirty = true;
        Pin::new(self.unblock.get_mut()).poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        if self.is_dirty {
            ready!(Pin::new(self.unblock.get_mut()).poll_flush(cx))?;
            self.is_dirty = false;
        }
        Poll::Ready(Ok(()))
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(self.unblock.get_mut()).poll_close(cx)
    }
}

impl AsyncSeek for File {
    fn poll_seek(mut self: Pin<&mut Self>, cx: &mut Context<'_>, pos: SeekFrom) -> Poll<io::Result<u64>> {
        ready!(self.poll_reposition(cx))?;
        Pin::new(self.unblock.get_mut()).poll_seek(cx, pos)
    }
}

/// A wrapper around `Arc<std::fs::File>` that implements `Read`, `Write`, and `Seek`.
struct ArcFile(Arc<std::fs::File>);

impl io::Read for ArcFile {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        (&*self.0).read(buf)
    }
}

impl io::Write for ArcFile {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        (&*self.0).write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        (&*self.0).flush()
    }
}

impl io::Seek for ArcFile {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        (&*self.0).seek(pos)
    }
}

/// A builder for opening files with configurable options.
///
/// Files can be opened in [`read`][`OpenOptions::read()`] and/or
/// [`write`][`OpenOptions::write()`] mode.
///
/// The [`append`][`OpenOptions::append()`] option opens files in a special writing mode that
/// moves the file cursor to the end of file before every write operation.
///
/// It is also possible to [`truncate`][`OpenOptions::truncate()`] the file right after opening,
/// to [`create`][`OpenOptions::create()`] a file if it doesn't exist yet, or to always create a
/// new file with [`create_new`][`OpenOptions::create_new()`].
///
/// # Examples
///
/// Open a file for reading:
///
/// ```no_run
/// use zng_task::fs::OpenOptions;
///
/// # futures_lite::future::block_on(async {
/// let file = OpenOptions::new().read(true).open("a.txt").await?;
/// # std::io::Result::Ok(()) });
/// ```
///
/// Open a file for both reading and writing, and create it if it doesn't exist yet:
///
/// ```no_run
/// use zng_task::fs::OpenOptions;
///
/// # futures_lite::future::block_on(async {
/// let file = OpenOptions::new().read(true).write(true).create(true).open("a.txt").await?;
/// # std::io::Result::Ok(()) });
/// ```
#[derive(Clone, Debug)]
pub struct OpenOptions(std::fs::OpenOptions);

impl OpenOptions {
    /// Creates a blank set of options.
    ///
    /// All options are initially set to `false`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use zng_task::fs::OpenOptions;
    ///
    /// # futures_lite::future::block_on(async {
    /// let file = OpenOptions::new().read(true).open("a.txt").await?;
    /// # std::io::Result::Ok(()) });
    /// ```
    pub fn new() -> OpenOptions {
        OpenOptions(std::fs::OpenOptions::new())
    }

    /// Configures the option for read mode.
    ///
    /// When set to `true`, this option means the file will be readable after opening.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use zng_task::fs::OpenOptions;
    ///
    /// # futures_lite::future::block_on(async {
    /// let file = OpenOptions::new().read(true).open("a.txt").await?;
    /// # std::io::Result::Ok(()) });
    /// ```
    pub fn read(&mut self, read: bool) -> &mut OpenOptions {
        self.0.read(read);
        self
    }

    /// Configures the option for write mode.
    ///
    /// When set to `true`, this option means the file will be writable after opening.
    ///
    /// If the file already exists, write calls on it will overwrite the previous contents without
    /// truncating it.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use zng_task::fs::OpenOptions;
    ///
    /// # futures_lite::future::block_on(async {
    /// let file = OpenOptions::new().write(true).open("a.txt").await?;
    /// # std::io::Result::Ok(()) });
    /// ```
    pub fn write(&mut self, write: bool) -> &mut OpenOptions {
        self.0.write(write);
        self
    }

    /// Configures the option for append mode.
    ///
    /// When set to `true`, this option means the file will be writable after opening and the file
    /// cursor will be moved to the end of file before every write operation.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use zng_task::fs::OpenOptions;
    ///
    /// # futures_lite::future::block_on(async {
    /// let file = OpenOptions::new().append(true).open("a.txt").await?;
    /// # std::io::Result::Ok(()) });
    /// ```
    pub fn append(&mut self, append: bool) -> &mut OpenOptions {
        self.0.append(append);
        self
    }

    /// Configures the option for truncating the previous file.
    ///
    /// When set to `true`, the file will be truncated to the length of 0 bytes.
    ///
    /// The file must be opened in [`write`][`OpenOptions::write()`] or
    /// [`append`][`OpenOptions::append()`] mode for truncation to work.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use zng_task::fs::OpenOptions;
    ///
    /// # futures_lite::future::block_on(async {
    /// let file = OpenOptions::new().write(true).truncate(true).open("a.txt").await?;
    /// # std::io::Result::Ok(()) });
    /// ```
    pub fn truncate(&mut self, truncate: bool) -> &mut OpenOptions {
        self.0.truncate(truncate);
        self
    }

    /// Configures the option for creating a new file if it doesn't exist.
    ///
    /// When set to `true`, this option means a new file will be created if it doesn't exist.
    ///
    /// The file must be opened in [`write`][`OpenOptions::write()`] or
    /// [`append`][`OpenOptions::append()`] mode for file creation to work.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use zng_task::fs::OpenOptions;
    ///
    /// # futures_lite::future::block_on(async {
    /// let file = OpenOptions::new().write(true).create(true).open("a.txt").await?;
    /// # std::io::Result::Ok(()) });
    /// ```
    pub fn create(&mut self, create: bool) -> &mut OpenOptions {
        self.0.create(create);
        self
    }

    /// Configures the option for creating a new file or failing if it already exists.
    ///
    /// When set to `true`, this option means a new file will be created, or the open operation
    /// will fail if the file already exists.
    ///
    /// The file must be opened in [`write`][`OpenOptions::write()`] or
    /// [`append`][`OpenOptions::append()`] mode for file creation to work.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use zng_task::fs::OpenOptions;
    ///
    /// # futures_lite::future::block_on(async {
    /// let file = OpenOptions::new().write(true).create_new(true).open("a.txt").await?;
    /// # std::io::Result::Ok(()) });
    /// ```
    pub fn create_new(&mut self, create_new: bool) -> &mut OpenOptions {
        self.0.create_new(create_new);
        self
    }

    /// Opens a file with the configured options.
    ///
    /// # Errors
    ///
    /// An error will be returned in the following situations:
    ///
    /// * The file does not exist and neither [`create`] nor [`create_new`] were set.
    /// * The file's parent directory does not exist.
    /// * The current process lacks permissions to open the file in the configured mode.
    /// * The file already exists and [`create_new`] was set.
    /// * Invalid combination of options was used, like [`truncate`] was set but [`write`] wasn't,
    ///   or none of [`read`], [`write`], and [`append`] modes was set.
    /// * An OS-level occurred, like too many files are open or the file name is too long.
    /// * Some other I/O error occurred.
    ///
    /// [`read`]: `OpenOptions::read()`
    /// [`write`]: `OpenOptions::write()`
    /// [`append`]: `OpenOptions::append()`
    /// [`truncate`]: `OpenOptions::truncate()`
    /// [`create`]: `OpenOptions::create()`
    /// [`create_new`]: `OpenOptions::create_new()`
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use zng_task::fs::OpenOptions;
    ///
    /// # futures_lite::future::block_on(async {
    /// let file = OpenOptions::new().read(true).open("a.txt").await?;
    /// # std::io::Result::Ok(()) });
    /// ```
    pub fn open<P: AsRef<Path>>(&self, path: P) -> impl Future<Output = io::Result<File>> {
        let path = path.as_ref().to_owned();
        let options = self.0.clone();
        async move {
            let file = unblock(move || options.open(path)).await?;
            Ok(File::new(file, false))
        }
    }
}

impl Default for OpenOptions {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(unix)]
impl unix::OpenOptionsExt for OpenOptions {
    fn mode(&mut self, mode: u32) -> &mut Self {
        self.0.mode(mode);
        self
    }

    fn custom_flags(&mut self, flags: i32) -> &mut Self {
        self.0.custom_flags(flags);
        self
    }
}

#[cfg(windows)]
impl windows::OpenOptionsExt for OpenOptions {
    fn access_mode(&mut self, access: u32) -> &mut Self {
        self.0.access_mode(access);
        self
    }

    fn share_mode(&mut self, val: u32) -> &mut Self {
        self.0.share_mode(val);
        self
    }

    fn custom_flags(&mut self, flags: u32) -> &mut Self {
        self.0.custom_flags(flags);
        self
    }

    fn attributes(&mut self, val: u32) -> &mut Self {
        self.0.attributes(val);
        self
    }

    fn security_qos_flags(&mut self, flags: u32) -> &mut Self {
        self.0.security_qos_flags(flags);
        self
    }
}

#[cfg_attr(not(any(windows, unix)), allow(dead_code))]
mod __private {
    #[doc(hidden)]
    pub trait Sealed {}

    impl Sealed for super::OpenOptions {}
    impl Sealed for super::File {}
}

/// Unix-specific extensions.
#[cfg(unix)]
pub mod unix {
    use super::__private::Sealed;

    #[doc(inline)]
    pub use async_fs::unix::*;

    /// Unix-specific extensions to [`OpenOptions`].
    ///
    /// [`OpenOptions`]: crate::fs::OpenOptions
    pub trait OpenOptionsExt: Sealed {
        /// Sets the mode bits that a new file will be created with.
        ///
        /// If a new file is created as part of an [`OpenOptions::open()`] call then this
        /// specified `mode` will be used as the permission bits for the new file.
        ///
        /// If no `mode` is set, the default of `0o666` will be used.
        /// The operating system masks out bits with the system's `umask`, to produce
        /// the final permissions.
        ///
        /// [`OpenOptions::open()`]: crate::fs::OpenOptions::open
        ///
        /// # Examples
        ///
        /// ```no_run
        /// use zng_task::fs::{OpenOptions, unix::OpenOptionsExt};
        ///
        /// # futures_lite::future::block_on(async {
        /// let mut options = OpenOptions::new();
        /// // Read/write permissions for owner and read permissions for others.
        /// options.mode(0o644);
        /// let file = options.open("foo.txt").await?;
        /// # std::io::Result::Ok(()) });
        /// ```
        fn mode(&mut self, mode: u32) -> &mut Self;

        /// Passes custom flags to the `flags` argument of `open`.
        ///
        /// The bits that define the access mode are masked out with `O_ACCMODE`, to
        /// ensure they do not interfere with the access mode set by Rust's options.
        ///
        /// Custom flags can only set flags, not remove flags set by Rust's options.
        /// This options overwrites any previously set custom flags.
        ///
        /// # Examples
        ///
        /// ```no_run
        /// # mod libc { pub const O_NOFOLLOW: i32 = 0x40000; }
        /// use zng_task::fs::{OpenOptions, unix::OpenOptionsExt};
        ///
        /// # futures_lite::future::block_on(async {
        /// let mut options = OpenOptions::new();
        /// options.write(true);
        /// options.custom_flags(libc::O_NOFOLLOW);
        /// let file = options.open("foo.txt").await?;
        /// # std::io::Result::Ok(()) });
        /// ```
        fn custom_flags(&mut self, flags: i32) -> &mut Self;
    }
}

/// Windows-specific extensions.
#[cfg(windows)]
pub mod windows {
    use super::__private::Sealed;

    #[doc(inline)]
    pub use async_fs::windows::*;

    /// Windows-specific extensions to [`OpenOptions`].
    ///
    /// [`OpenOptions`]: crate::fs::OpenOptions
    pub trait OpenOptionsExt: Sealed {
        /// Overrides the `dwDesiredAccess` argument to the call to [`CreateFile`]
        /// with the specified value.
        ///
        /// This will override the `read`, `write`, and `append` flags on the
        /// [`OpenOptions`] structure. This method provides fine-grained control over
        /// the permissions to read, write and append data, attributes (like hidden
        /// and system), and extended attributes.
        ///
        /// # Examples
        ///
        /// ```no_run
        /// use zng_task::fs::{OpenOptions, windows::OpenOptionsExt};
        ///
        /// # futures_lite::future::block_on(async {
        /// // Open without read and write permission, for example if you only need
        /// // to call `stat` on the file
        /// let file = OpenOptions::new().access_mode(0).open("foo.txt").await?;
        /// # std::io::Result::Ok(()) });
        /// ```
        ///
        /// [`CreateFile`]: https://docs.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createfilea
        /// [`OpenOptions`]: crate::fs::OpenOptions
        fn access_mode(&mut self, access: u32) -> &mut Self;

        /// Overrides the `dwShareMode` argument to the call to [`CreateFile`] with
        /// the specified value.
        ///
        /// By default `share_mode` is set to
        /// `FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE`. This allows
        /// other processes to read, write, and delete/rename the same file
        /// while it is open. Removing any of the flags will prevent other
        /// processes from performing the corresponding operation until the file
        /// handle is closed.
        ///
        /// # Examples
        ///
        /// ```no_run
        /// use zng_task::fs::{OpenOptions, windows::OpenOptionsExt};
        ///
        /// # futures_lite::future::block_on(async {
        /// // Do not allow others to read or modify this file while we have it open
        /// // for writing.
        /// let file = OpenOptions::new().write(true).share_mode(0).open("foo.txt").await?;
        /// # std::io::Result::Ok(()) });
        /// ```
        ///
        /// [`CreateFile`]: https://docs.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createfilea
        fn share_mode(&mut self, val: u32) -> &mut Self;

        /// Sets extra flags for the `dwFileFlags` argument to the call to
        /// [`CreateFile2`] to the specified value (or combines it with
        /// `attributes` and `security_qos_flags` to set the `dwFlagsAndAttributes`
        /// for [`CreateFile`]).
        ///
        /// Custom flags can only set flags, not remove flags set by Rust's options.
        /// This option overwrites any previously set custom flags.
        ///
        /// # Examples
        ///
        /// ```no_run
        /// use zng_task::fs::{OpenOptions, windows::OpenOptionsExt};
        ///
        /// # futures_lite::future::block_on(async {
        /// let file = OpenOptions::new()
        ///     .create(true)
        ///     .write(true)
        ///     .custom_flags(windows_sys::Win32::Storage::FileSystem::FILE_FLAG_DELETE_ON_CLOSE)
        ///     .open("foo.txt")
        ///     .await?;
        /// # std::io::Result::Ok(()) });
        /// ```
        ///
        /// [`CreateFile`]: https://docs.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createfilea
        /// [`CreateFile2`]: https://docs.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createfile2
        fn custom_flags(&mut self, flags: u32) -> &mut Self;

        /// Sets the `dwFileAttributes` argument to the call to [`CreateFile2`] to
        /// the specified value (or combines it with `custom_flags` and
        /// `security_qos_flags` to set the `dwFlagsAndAttributes` for
        /// [`CreateFile`]).
        ///
        /// If a _new_ file is created because it does not yet exist and
        /// `.create(true)` or `.create_new(true)` are specified, the new file is
        /// given the attributes declared with `.attributes()`.
        ///
        /// If an _existing_ file is opened with `.create(true).truncate(true)`, its
        /// existing attributes are preserved and combined with the ones declared
        /// with `.attributes()`.
        ///
        /// In all other cases the attributes get ignored.
        ///
        /// # Examples
        ///
        /// ```no_run
        /// use zng_task::fs::{OpenOptions, windows::OpenOptionsExt};
        ///
        /// # futures_lite::future::block_on(async {
        /// let file = OpenOptions::new()
        ///     .write(true)
        ///     .create(true)
        ///     .attributes(windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_HIDDEN)
        ///     .open("foo.txt")
        ///     .await?;
        /// # std::io::Result::Ok(()) });
        /// ```
        ///
        /// [`CreateFile`]: https://docs.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createfilea
        /// [`CreateFile2`]: https://docs.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createfile2
        fn attributes(&mut self, val: u32) -> &mut Self;

        /// Sets the `dwSecurityQosFlags` argument to the call to [`CreateFile2`] to
        /// the specified value (or combines it with `custom_flags` and `attributes`
        /// to set the `dwFlagsAndAttributes` for [`CreateFile`]).
        ///
        /// By default `security_qos_flags` is not set. It should be specified when
        /// opening a named pipe, to control to which degree a server process can
        /// act on behalf of a client process (security impersonation level).
        ///
        /// When `security_qos_flags` is not set, a malicious program can gain the
        /// elevated privileges of a privileged Rust process when it allows opening
        /// user-specified paths, by tricking it into opening a named pipe. So
        /// arguably `security_qos_flags` should also be set when opening arbitrary
        /// paths. However the bits can then conflict with other flags, specifically
        /// `FILE_FLAG_OPEN_NO_RECALL`.
        ///
        /// For information about possible values, see [Impersonation Levels] on the
        /// Windows Dev Center site. The `SECURITY_SQOS_PRESENT` flag is set
        /// automatically when using this method.
        ///
        /// # Examples
        ///
        /// ```no_run
        /// use zng_task::fs::{OpenOptions, windows::OpenOptionsExt};
        ///
        /// # futures_lite::future::block_on(async {
        /// let file = OpenOptions::new()
        ///     .write(true)
        ///     .create(true)
        ///     .security_qos_flags(windows_sys::Win32::Storage::FileSystem::SECURITY_IDENTIFICATION)
        ///     .open(r"\\.\pipe\MyPipe")
        ///     .await?;
        /// # std::io::Result::Ok(()) });
        /// ```
        ///
        /// [`CreateFile`]: https://docs.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createfilea
        /// [`CreateFile2`]: https://docs.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createfile2
        /// [Impersonation Levels]: https://docs.microsoft.com/en-us/windows/win32/api/winnt/ne-winnt-security_impersonation_level
        fn security_qos_flags(&mut self, flags: u32) -> &mut Self;
    }
}
