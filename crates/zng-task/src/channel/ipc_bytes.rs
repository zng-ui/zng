#![cfg_attr(not(ipc), allow(unused))]

use std::{
    cell::Cell,
    fmt, fs,
    io::{self, Read, Write},
    iter::FusedIterator,
    ops,
    path::{Path, PathBuf},
    pin::Pin,
    sync::{Arc, Weak},
};

use futures_lite::{AsyncReadExt, AsyncWriteExt as _};
#[cfg(ipc)]
use ipc_channel::ipc::IpcSharedMemory;
use serde::{Deserialize, Serialize};
use zng_app_context::RunOnDrop;

#[cfg(ipc)]
use crate::channel::ipc_bytes_memmap::Memmap;

/// Immutable bytes vector that can be can be shared fast over IPC.
///
/// # Memory Storage
///
/// All storage backends are held by an [`Arc`] pointer, so cloning in process is always very cheap.
///
/// The `from_*` constructor functions use different storage depending on byte length. Bytes <= 64KB are allocated in the heap
/// and are copied when shared with another process. Bytes <= 128MB are allocated in shared memory, only the system handle
/// is copied when shared with another process. Bytes > 128MB are allocated in a temporary file with restricted access and memory mapped
/// for read, only the file handle and some metadata are copied when shared with another process.
///
/// Constructor functions for creating memory maps directly are also provided.
///
/// Note that in builds without the `"ipc"` crate feature only heap backend is available, in that case all data lengths are stored in the heap.
///
/// # Serialization
///
/// When serialized inside [`with_ipc_serialization`] only the system handles and metadata of shared memory and memory maps is serialized.
/// When serializing outsize the IPC context the data is copied.
///
/// When deserializing memory map handles are reconnected and if deserializing bytes selects the best storage based on data length.
///
/// [`IpcSender`]: super::IpcSender
#[derive(Clone)]
#[repr(transparent)]
pub struct IpcBytes(pub(super) Arc<IpcBytesData>);
impl Serialize for IpcBytes {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        Serialize::serialize(&*self.0, serializer)
    }
}
impl<'de> Deserialize<'de> for IpcBytes {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let b = <IpcBytesData as Deserialize>::deserialize(deserializer)?;
        Ok(Self(Arc::new(b)))
    }
}
#[derive(Serialize, Deserialize)]
pub(super) enum IpcBytesData {
    Heap(Vec<u8>),
    #[cfg(ipc)]
    AnonMemMap(IpcSharedMemory),
    #[cfg(ipc)]
    MemMap(Memmap),
}
impl fmt::Debug for IpcBytes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "IpcBytes(<{} bytes>)", self.len())
    }
}
impl ops::Deref for IpcBytes {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        match &*self.0 {
            IpcBytesData::Heap(i) => i,
            #[cfg(ipc)]
            IpcBytesData::AnonMemMap(m) => m,
            #[cfg(ipc)]
            IpcBytesData::MemMap(f) => f,
        }
    }
}

impl IpcBytes {
    /// New empty.
    pub fn empty() -> Self {
        IpcBytes(Arc::new(IpcBytesData::Heap(vec![])))
    }
}

/// Constructors.
///
/// See also [`IpcBytesMut`](crate::channel::IpcBytesMut).
impl IpcBytes {
    /// Copy or move data from vector.
    pub async fn from_vec(data: Vec<u8>) -> io::Result<Self> {
        blocking::unblock(move || Self::from_vec_blocking(data)).await
    }

    /// Copy or move data from vector.
    pub fn from_vec_blocking(data: Vec<u8>) -> io::Result<Self> {
        #[cfg(ipc)]
        {
            if data.len() <= Self::INLINE_MAX {
                Ok(Self(Arc::new(IpcBytesData::Heap(data))))
            } else {
                Self::from_slice_blocking(&data)
            }
        }
        #[cfg(not(ipc))]
        {
            Ok(Self(Arc::new(IpcBytesData::Heap(data))))
        }
    }

    /// Copy data from slice.
    pub fn from_slice_blocking(data: &[u8]) -> io::Result<Self> {
        #[cfg(ipc)]
        {
            if data.len() <= Self::INLINE_MAX {
                Ok(Self(Arc::new(IpcBytesData::Heap(data.to_vec()))))
            } else if data.len() <= Self::UNNAMED_MAX {
                Ok(Self(Arc::new(IpcBytesData::AnonMemMap(IpcSharedMemory::from_bytes(data)))))
            } else {
                Self::new_memmap_blocking(|m| m.write_all(data))
            }
        }
        #[cfg(not(ipc))]
        {
            Ok(Self(Arc::new(IpcBytesData::Heap(data.to_vec()))))
        }
    }

    /// Copy data from the iterator.
    ///
    /// This is most efficient if the [`size_hint`] indicates an exact length (min equals max), otherwise this
    /// will collect to an [`IpcBytesWriter`] that can reallocate multiple times as the buffer grows.
    ///    
    /// Note that if the iterator gives an exact length that is the maximum taken, if it ends early the smaller length
    /// is used, if it continues after the given maximum it is clipped.
    ///
    /// [`size_hint`]: Iterator::size_hint
    /// [`IpcBytesWriter`]: crate::channel::IpcBytesWriter
    pub async fn from_iter(iter: impl Iterator<Item = u8>) -> io::Result<Self> {
        #[cfg(ipc)]
        {
            let (min, max) = iter.size_hint();
            if let Some(max) = max {
                if max <= Self::INLINE_MAX {
                    return Ok(Self(Arc::new(IpcBytesData::Heap(iter.collect()))));
                } else if max == min {
                    let mut r = super::IpcBytesMut::new(max).await?;
                    let mut actual_len = 0;
                    for (i, b) in r.iter_mut().zip(iter) {
                        *i = b;
                        actual_len += 1;
                    }
                    r.truncate(actual_len);
                    return r.finish().await;
                }
            }

            let mut writer = Self::new_writer().await;
            for b in iter {
                writer.write_all(&[b]).await?;
            }
            writer.finish().await
        }

        #[cfg(not(ipc))]
        {
            Ok(Self(Arc::new(IpcBytesData::Heap(iter.collect()))))
        }
    }

    /// Copy data from the iterator.
    ///
    /// This is most efficient if the [`size_hint`] indicates an exact length (min equals max), otherwise this
    /// will collect to an [`IpcBytesWriterBlocking`] that can reallocate multiple times as the buffer grows.
    ///
    /// Note that if the iterator gives an exact length that is the maximum taken, if it ends early the smaller length
    /// is used, if it continues after the given maximum it is clipped.
    ///
    /// [`size_hint`]: Iterator::size_hint
    ///
    /// [`IpcBytesWriterBlocking`]: crate::channel::IpcBytesWriterBlocking
    pub fn from_iter_blocking(iter: impl Iterator<Item = u8>) -> io::Result<Self> {
        #[cfg(ipc)]
        {
            let (min, max) = iter.size_hint();
            if let Some(max) = max {
                if max <= Self::INLINE_MAX {
                    return Ok(Self(Arc::new(IpcBytesData::Heap(iter.collect()))));
                } else if max == min {
                    let mut r = super::IpcBytesMut::new_blocking(max)?;
                    let mut actual_len = 0;
                    for (i, b) in r.iter_mut().zip(iter) {
                        *i = b;
                        actual_len += 1;
                    }
                    r.truncate(actual_len);
                    return r.finish_blocking();
                }
            }

            let mut writer = Self::new_writer_blocking();
            for b in iter {
                writer.write_all(&[b])?;
            }
            writer.finish()
        }
        #[cfg(not(ipc))]
        {
            Ok(Self(Arc::new(IpcBytesData::Heap(iter.collect()))))
        }
    }

    /// Read `data` into shared memory.
    pub async fn from_read(data: Pin<&mut (dyn futures_lite::AsyncRead + Send)>) -> io::Result<Self> {
        #[cfg(ipc)]
        {
            Self::from_read_ipc(data).await
        }
        #[cfg(not(ipc))]
        {
            let mut data = data;
            let mut buf = vec![];
            data.read_to_end(&mut buf).await;
            Self::from_vec(buf).await
        }
    }
    #[cfg(ipc)]
    async fn from_read_ipc(mut data: Pin<&mut (dyn futures_lite::AsyncRead + Send)>) -> io::Result<Self> {
        let mut buf = vec![0u8; Self::INLINE_MAX + 1];
        let mut len = 0;

        // INLINE_MAX read
        loop {
            match data.read(&mut buf[len..]).await {
                Ok(l) => {
                    if l == 0 {
                        // is <= INLINE_MAX
                        buf.truncate(len);
                        return Ok(Self(Arc::new(IpcBytesData::Heap(buf))));
                    } else {
                        len += l;
                        if len == Self::INLINE_MAX + 1 {
                            // goto UNNAMED_MAX read
                            break;
                        }
                    }
                }
                Err(e) => match e.kind() {
                    io::ErrorKind::WouldBlock => continue,
                    _ => return Err(e),
                },
            }
        }

        // UNNAMED_MAX read
        buf.resize(Self::UNNAMED_MAX + 1, 0);
        loop {
            match data.read(&mut buf[len..]).await {
                Ok(l) => {
                    if l == 0 {
                        // is <= UNNAMED_MAX
                        return Ok(Self(Arc::new(IpcBytesData::AnonMemMap(IpcSharedMemory::from_bytes(&buf[..len])))));
                    } else {
                        len += l;
                        if len == Self::UNNAMED_MAX + 1 {
                            // goto named file loop
                            break;
                        }
                    }
                }
                Err(e) => match e.kind() {
                    io::ErrorKind::WouldBlock => continue,
                    _ => return Err(e),
                },
            }
        }

        // named file copy
        Self::new_memmap(async |m| {
            use futures_lite::AsyncWriteExt as _;

            m.write_all(&buf).await?;
            crate::io::copy(data, m).await?;
            Ok(())
        })
        .await
    }

    /// Read `data` into shared memory.
    pub fn from_read_blocking(data: &mut dyn io::Read) -> io::Result<Self> {
        #[cfg(ipc)]
        {
            Self::from_read_blocking_ipc(data)
        }
        #[cfg(not(ipc))]
        {
            let mut buf = vec![];
            data.read_to_end(&mut buf)?;
            Self::from_vec_blocking(buf)
        }
    }
    #[cfg(ipc)]
    fn from_read_blocking_ipc(data: &mut dyn io::Read) -> io::Result<Self> {
        let mut buf = vec![0u8; Self::INLINE_MAX + 1];
        let mut len = 0;

        // INLINE_MAX read
        loop {
            match data.read(&mut buf[len..]) {
                Ok(l) => {
                    if l == 0 {
                        // is <= INLINE_MAX
                        buf.truncate(len);
                        return Ok(Self(Arc::new(IpcBytesData::Heap(buf))));
                    } else {
                        len += l;
                        if len == Self::INLINE_MAX + 1 {
                            // goto UNNAMED_MAX read
                            break;
                        }
                    }
                }
                Err(e) => match e.kind() {
                    io::ErrorKind::WouldBlock => continue,
                    _ => return Err(e),
                },
            }
        }

        // UNNAMED_MAX read
        buf.resize(Self::UNNAMED_MAX + 1, 0);
        loop {
            match data.read(&mut buf[len..]) {
                Ok(l) => {
                    if l == 0 {
                        // is <= UNNAMED_MAX
                        return Ok(Self(Arc::new(IpcBytesData::AnonMemMap(IpcSharedMemory::from_bytes(&buf[..len])))));
                    } else {
                        len += l;
                        if len == Self::UNNAMED_MAX + 1 {
                            // goto named file loop
                            break;
                        }
                    }
                }
                Err(e) => match e.kind() {
                    io::ErrorKind::WouldBlock => continue,
                    _ => return Err(e),
                },
            }
        }

        // named file copy
        Self::new_memmap_blocking(|m| {
            m.write_all(&buf)?;
            io::copy(data, m)?;
            Ok(())
        })
    }

    /// Read `path` into shared memory.
    pub async fn from_path(path: PathBuf) -> io::Result<Self> {
        let file = crate::fs::File::open(path).await?;
        Self::from_file(file).await
    }
    /// Read `file` into shared memory.
    pub async fn from_file(mut file: crate::fs::File) -> io::Result<Self> {
        #[cfg(ipc)]
        {
            let len = file.metadata().await?.len();
            if len <= Self::UNNAMED_MAX as u64 {
                let mut buf = vec![0u8; len as usize];
                file.read_exact(&mut buf).await?;
                Self::from_vec_blocking(buf)
            } else {
                Self::new_memmap(async move |m| {
                    crate::io::copy(&mut file, m).await?;
                    Ok(())
                })
                .await
            }
        }
        #[cfg(not(ipc))]
        {
            let mut buf = vec![];
            file.read_to_end(&mut buf).await?;
            Self::from_vec_blocking(buf)
        }
    }

    /// Read `path` into shared memory.
    pub fn from_path_blocking(path: &Path) -> io::Result<Self> {
        let file = fs::File::open(path)?;
        Self::from_file_blocking(file)
    }
    /// Read `file` into shared memory.
    pub fn from_file_blocking(mut file: fs::File) -> io::Result<Self> {
        #[cfg(ipc)]
        {
            let len = file.metadata()?.len();
            if len <= Self::UNNAMED_MAX as u64 {
                let mut buf = vec![0u8; len as usize];
                file.read_exact(&mut buf)?;
                Self::from_vec_blocking(buf)
            } else {
                Self::new_memmap_blocking(|m| {
                    io::copy(&mut file, m)?;
                    Ok(())
                })
            }
        }
        #[cfg(not(ipc))]
        {
            let mut buf = vec![];
            file.read_to_end(&mut buf)?;
            Self::from_vec_blocking(buf)
        }
    }

    /// Create a memory mapped file.
    ///
    /// Note that the `from_` functions select optimized backing storage depending on data length, this function
    /// always selects the slowest options, a file backed memory map.
    #[cfg(ipc)]
    pub async fn new_memmap(write: impl AsyncFnOnce(&mut crate::fs::File) -> io::Result<()>) -> io::Result<Self> {
        use crate::channel::ipc_bytes_memmap::MemmapMut;

        let file = blocking::unblock(MemmapMut::begin_write).await?;
        let mut file = crate::fs::File::from(file);
        write(&mut file).await?;

        match file.try_unwrap().await {
            Ok(f) => {
                let map = blocking::unblock(move || Memmap::end_write(f)).await?;
                Ok(Self(Arc::new(IpcBytesData::MemMap(map))))
            }
            Err(_) => Err(io::Error::new(
                io::ErrorKind::ResourceBusy,
                "no all tasks started by `write` awaited before return",
            )),
        }
    }

    /// Memory map an existing file.
    ///
    /// The `range` defines the slice of the `file` that will be mapped. Returns [`io::ErrorKind::UnexpectedEof`]
    // if the file does not have enough bytes. Returns [`io::ErrorKind::FileTooLarge`] if the range length or file length is
    // greater than `usize::MAX`.
    ///
    /// # Safety
    ///
    /// Caller must ensure the `file` is not modified while all clones of the `IpcBytes` exists in the current process and others.
    #[cfg(ipc)]
    pub async unsafe fn open_memmap(file: PathBuf, range: Option<ops::Range<u64>>) -> io::Result<Self> {
        blocking::unblock(move || {
            // SAFETY: up to the caller
            unsafe { Self::open_memmap_blocking(file, range) }
        })
        .await
    }

    /// Create a memory mapped file.
    ///
    /// Note that the `from_` functions select optimized backing storage depending on data length, this function
    /// always selects the slowest options, a file backed memory map.
    #[cfg(ipc)]
    pub fn new_memmap_blocking(write: impl FnOnce(&mut fs::File) -> io::Result<()>) -> io::Result<Self> {
        use crate::channel::ipc_bytes_memmap::MemmapMut;

        let mut file = MemmapMut::begin_write()?;
        write(&mut file)?;
        let map = Memmap::end_write(file)?;

        Ok(Self(Arc::new(IpcBytesData::MemMap(map))))
    }

    /// Memory map an existing file.
    ///
    /// The `range` defines the slice of the `file` that will be mapped. Returns [`io::ErrorKind::UnexpectedEof`]
    // if the file does not have enough bytes. Returns [`io::ErrorKind::FileTooLarge`] if the range length or file length is
    // greater than `usize::MAX`.
    ///
    /// # Safety
    ///
    /// Caller must ensure the `file` is not modified or removed while all clones of the `IpcBytes` exists in the current process and others.
    #[cfg(ipc)]
    pub unsafe fn open_memmap_blocking(file: PathBuf, range: Option<ops::Range<u64>>) -> io::Result<Self> {
        // SAFETY: up to the caller
        let map = unsafe { Memmap::read_user_file(file, range) }?;

        Ok(Self(Arc::new(IpcBytesData::MemMap(map))))
    }

    /// Gets if both point to the same memory.
    pub fn ptr_eq(&self, other: &Self) -> bool {
        let a = &self[..];
        let b = &other[..];
        (std::ptr::eq(a, b) && a.len() == b.len()) || (a.is_empty() && b.is_empty())
    }

    #[cfg(ipc)]
    pub(super) const INLINE_MAX: usize = 64 * 1024; // 64KB
    #[cfg(ipc)]
    pub(super) const UNNAMED_MAX: usize = 128 * 1024 * 1024; // 128MB
}

impl AsRef<[u8]> for IpcBytes {
    fn as_ref(&self) -> &[u8] {
        &self[..]
    }
}
impl Default for IpcBytes {
    fn default() -> Self {
        Self::empty()
    }
}
impl PartialEq for IpcBytes {
    fn eq(&self, other: &Self) -> bool {
        self.ptr_eq(other) || self[..] == other[..]
    }
}
impl Eq for IpcBytes {}

/// Enables special serialization of memory mapped files for the `serialize` call.
///
/// IPC channels like [`IpcSender`] serialize messages inside this context to support [`IpcBytes`] fast memory map sharing across processes.
///
/// You can use the [`is_ipc_serialization`] to check if inside context.
///
/// [`IpcSender`]: super::IpcSender
#[cfg(ipc)]
pub fn with_ipc_serialization<R>(serialize: impl FnOnce() -> R) -> R {
    let parent = IPC_SERIALIZATION.replace(true);
    let _clean = RunOnDrop::new(|| IPC_SERIALIZATION.set(parent));
    serialize()
}

/// Checks if is inside [`with_ipc_serialization`].
#[cfg(ipc)]
pub fn is_ipc_serialization() -> bool {
    IPC_SERIALIZATION.get()
}

#[cfg(ipc)]
thread_local! {
    static IPC_SERIALIZATION: Cell<bool> = const { Cell::new(false) };
}

impl IpcBytes {
    /// Create a weak in process reference.
    ///
    /// Note that the weak reference cannot upgrade if only another process holds a strong reference.
    pub fn downgrade(&self) -> WeakIpcBytes {
        WeakIpcBytes(Arc::downgrade(&self.0))
    }
}

/// Weak reference to an in process [`IpcBytes`].
pub struct WeakIpcBytes(Weak<IpcBytesData>);
impl WeakIpcBytes {
    /// Get strong reference if any exists in the process.
    pub fn upgrade(&self) -> Option<IpcBytes> {
        self.0.upgrade().map(IpcBytes)
    }

    /// Count of strong references in the process.
    pub fn strong_count(&self) -> usize {
        self.0.strong_count()
    }
}

// Slice iterator is very efficient, but it hold a reference, so we hold a self reference.
// The alternative to this is copying the unsafe code from std and adapting it or implementing
// a much slower index based iterator.
type SliceIter<'a> = std::slice::Iter<'a, u8>;
self_cell::self_cell! {
    struct IpcBytesIntoIterInner {
        owner: IpcBytes,
        #[covariant]
        dependent: SliceIter,
    }
}

/// An [`IpcBytes`] iterator that holds a strong reference to it.
pub struct IpcBytesIntoIter(IpcBytesIntoIterInner);
impl IpcBytesIntoIter {
    fn new(bytes: IpcBytes) -> Self {
        Self(IpcBytesIntoIterInner::new(bytes, |b| b.iter()))
    }

    /// The source bytes.
    pub fn source(&self) -> &IpcBytes {
        self.0.borrow_owner()
    }

    /// Bytes not yet iterated.
    pub fn rest(&self) -> &[u8] {
        self.0.borrow_dependent().as_slice()
    }
}
impl Iterator for IpcBytesIntoIter {
    type Item = u8;

    fn next(&mut self) -> Option<u8> {
        self.0.with_dependent_mut(|_, d| d.next().copied())
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.borrow_dependent().size_hint()
    }

    fn count(self) -> usize
    where
        Self: Sized,
    {
        self.0.borrow_dependent().as_slice().len()
    }

    fn nth(&mut self, n: usize) -> Option<u8> {
        self.0.with_dependent_mut(|_, d| d.nth(n).copied())
    }

    fn last(mut self) -> Option<Self::Item>
    where
        Self: Sized,
    {
        self.next_back()
    }
}
impl DoubleEndedIterator for IpcBytesIntoIter {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0.with_dependent_mut(|_, d| d.next_back().copied())
    }

    fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
        self.0.with_dependent_mut(|_, d| d.nth_back(n).copied())
    }
}
impl FusedIterator for IpcBytesIntoIter {}
impl Default for IpcBytesIntoIter {
    fn default() -> Self {
        IpcBytes::empty().into_iter()
    }
}
impl IntoIterator for IpcBytes {
    type Item = u8;

    type IntoIter = IpcBytesIntoIter;

    fn into_iter(self) -> Self::IntoIter {
        IpcBytesIntoIter::new(self)
    }
}
