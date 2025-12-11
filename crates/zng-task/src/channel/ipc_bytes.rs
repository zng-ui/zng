#![cfg_attr(not(ipc), allow(unused))]

use std::{
    cell::Cell,
    fmt, fs,
    io::{self, Read, Write},
    marker::PhantomData,
    mem::MaybeUninit,
    ops,
    path::{Path, PathBuf},
    pin::Pin,
    sync::{Arc, Weak},
};

use futures_lite::AsyncReadExt;
#[cfg(ipc)]
use ipc_channel::ipc::IpcSharedMemory;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize, de::VariantAccess};
use zng_app_context::RunOnDrop;

/// Immutable bytes vector that can be can be shared fast over IPC.
///
/// # Memory Storage
///
/// All storage backends are held by a [`Arc`] pointer, so cloning in process is always very cheap.
///
/// The `from_*` constructor functions use different storage depending on byte length. Bytes <= 64KB are allocated in the heap
/// and are copied when shared with another process. Bytes <= 128MB are allocated in an anonymous memory map, only the system handle
/// is copied when shared with another process. Bytes > 128MB are allocated in a temporary file with restricted access and memory mapped
/// for read, only the file path and some metadata are copied when shared with another process.
///
/// Constructor functions for creating memory maps directly are also provided.
///
/// Note that in builds without the `"ipc"` crate feature only heap backend is available, in that case all data lengths are stored in the heap.
///
/// # Serialization
///
/// When serialized inside [`with_ipc_serialization`] the memory map bytes are not copied, only the system handle and metadata is serialized.
/// When serialized in other contexts all bytes are copied.
///
/// When deserializing memory map handles are reconnected and if deserializing bytes selects the best storage based on data length.
///
/// [`IpcSender`]: super::IpcSender
#[derive(Clone)]
#[repr(transparent)]
pub struct IpcBytes(Arc<IpcBytesData>);
enum IpcBytesData {
    Heap(Vec<u8>),
    #[cfg(ipc)]
    AnonMemMap(IpcSharedMemory),
    #[cfg(ipc)]
    MemMap(IpcMemMap),
}
#[cfg(ipc)]
struct IpcMemMap {
    name: PathBuf,
    range: ops::Range<usize>,
    is_custom: bool,
    map: Option<memmap2::Mmap>,
    read_handle: Option<fs::File>,
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
            IpcBytesData::MemMap(f) => f.map.as_ref().unwrap(),
        }
    }
}

impl IpcBytes {
    /// New empty.
    pub fn empty() -> Self {
        IpcBytes(Arc::new(IpcBytesData::Heap(vec![])))
    }
}
/// Async constructors.
impl IpcBytes {
    /// Start a memory efficient async writer for creating a `IpcBytes` with unknown length.
    pub async fn new_writer() -> IpcBytesWriter {
        IpcBytesWriter {
            inner: blocking::Unblock::new(Self::new_writer_blocking()),
        }
    }

    /// Allocate zeroed mutable memory that can be written to and then converted to `IpcBytes` fast.
    pub async fn new_mut(len: usize) -> io::Result<IpcBytesMut> {
        #[cfg(ipc)]
        if len <= Self::INLINE_MAX {
            Ok(IpcBytesMut {
                len,
                inner: IpcBytesMutInner::Heap(vec![0; len]),
            })
        } else if len <= Self::UNNAMED_MAX {
            Ok(IpcBytesMut {
                len,
                inner: IpcBytesMutInner::AnonMemMap(IpcSharedMemory::from_byte(0, len)),
            })
        } else {
            blocking::unblock(move || Self::new_mut_blocking(len)).await
        }

        #[cfg(not(ipc))]
        {
            Ok(IpcBytesMut {
                len,
                inner: IpcBytesMutInner::Heap(vec![0; len]),
            })
        }
    }

    /// Copy or move data from vector.
    pub async fn from_vec(data: Vec<u8>) -> io::Result<Self> {
        blocking::unblock(move || Self::from_vec_blocking(data)).await
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

    /// Read `file` into shared memory.
    pub async fn from_file(file: PathBuf) -> io::Result<Self> {
        #[cfg(ipc)]
        {
            let mut file = crate::fs::File::open(file).await?;
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
            let mut file = crate::fs::File::open(file).await?;
            let mut buf = vec![];
            file.read_to_end(&mut buf).await?;
            Self::from_vec_blocking(buf)
        }
    }

    /// Create a memory mapped file.
    ///
    /// Note that the `from_` functions select optimized backing storage depending on data length, this function
    /// always selects the slowest options, a file backed memory map.
    #[cfg(ipc)]
    pub async fn new_memmap(write: impl AsyncFnOnce(&mut crate::fs::File) -> io::Result<()>) -> io::Result<Self> {
        let (name, file) = blocking::unblock(Self::create_memmap).await?;
        let mut file = crate::fs::File::from(file);
        write(&mut file).await?;

        let mut permissions = file.metadata().await?.permissions();
        permissions.set_readonly(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            permissions.set_mode(0o400);
        }
        file.set_permissions(permissions).await?;

        blocking::unblock(move || {
            drop(file);
            let map = IpcMemMap::read(name, None)?;
            Ok(Self(Arc::new(IpcBytesData::MemMap(map))))
        })
        .await
    }

    /// Memory map an existing file.
    ///
    /// The `range` defines the slice of the `file` that will be mapped. Returns [`io::ErrorKind::UnexpectedEof`] if the file does not have enough bytes.
    ///
    /// # Safety
    ///
    /// Caller must ensure the `file` is not modified while all clones of the `IpcBytes` exists in the current process and others.
    ///
    /// Note that the safe [`new_memmap`] function assures safety by retaining a read lock (Windows) and restricting access rights (Unix)
    /// so that the file data is as read-only as the static data in the current executable file.
    ///
    /// [`new_memmap`]: Self::new_memmap
    #[cfg(ipc)]
    pub async unsafe fn open_memmap(file: PathBuf, range: Option<ops::Range<usize>>) -> io::Result<Self> {
        blocking::unblock(move || {
            // SAFETY: up to the caller
            unsafe { Self::open_memmap_blocking(file, range) }
        })
        .await
    }

    /// Gets if both point to the same memory.
    pub fn ptr_eq(&self, other: &Self) -> bool {
        let a = &self[..];
        let b = &other[..];
        (std::ptr::eq(a, b) && a.len() == b.len()) || (a.is_empty() && b.is_empty())
    }

    #[cfg(ipc)]
    const INLINE_MAX: usize = 64 * 1024; // 64KB
    #[cfg(ipc)]
    const UNNAMED_MAX: usize = 128 * 1024 * 1024; // 128MB
}

/// Blocking constructors.
impl IpcBytes {
    /// Start a memory efficient blocking writer for creating a `IpcBytes` with unknown length.
    pub fn new_writer_blocking() -> IpcBytesWriterBlocking {
        IpcBytesWriterBlocking {
            #[cfg(ipc)]
            heap_buf: vec![],
            #[cfg(ipc)]
            memmap: None,

            #[cfg(not(ipc))]
            heap_buf: std::io::Cursor::new(vec![]),
        }
    }

    /// Allocate zeroed mutable memory that can be written to and then converted to `IpcBytes` fast.
    pub fn new_mut_blocking(len: usize) -> io::Result<IpcBytesMut> {
        #[cfg(ipc)]
        if len <= Self::INLINE_MAX {
            Ok(IpcBytesMut {
                len,
                inner: IpcBytesMutInner::Heap(vec![0; len]),
            })
        } else if len <= Self::UNNAMED_MAX {
            Ok(IpcBytesMut {
                len,
                inner: IpcBytesMutInner::AnonMemMap(IpcSharedMemory::from_byte(0, len)),
            })
        } else {
            let (name, file) = Self::create_memmap()?;
            file.lock()?;
            #[cfg(unix)]
            {
                let mut permissions = file.metadata()?.permissions();
                use std::os::unix::fs::PermissionsExt;
                permissions.set_mode(0o600);
                file.set_permissions(permissions)?;
            }
            file.set_len(len as u64)?;
            // SAFETY: we hold write lock
            let map = unsafe { memmap2::MmapMut::map_mut(&file) }?;
            Ok(IpcBytesMut {
                len,
                inner: IpcBytesMutInner::MemMap {
                    name,
                    map,
                    write_handle: file,
                },
            })
        }
        #[cfg(not(ipc))]
        {
            Ok(IpcBytesMut {
                len,
                inner: IpcBytesMutInner::Heap(vec![0; len]),
            })
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

    /// Read `file` into shared memory.
    pub fn from_file_blocking(file: &Path) -> io::Result<Self> {
        #[cfg(ipc)]
        {
            let mut file = fs::File::open(file)?;
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
            let mut file = fs::File::open(file)?;
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
    pub fn new_memmap_blocking(write: impl FnOnce(&mut fs::File) -> io::Result<()>) -> io::Result<Self> {
        let (name, mut file) = Self::create_memmap()?;
        write(&mut file)?;
        let mut permissions = file.metadata()?.permissions();
        permissions.set_readonly(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            permissions.set_mode(0o400);
        }
        file.set_permissions(permissions)?;

        drop(file);
        let map = IpcMemMap::read(name, None)?;
        Ok(Self(Arc::new(IpcBytesData::MemMap(map))))
    }
    #[cfg(ipc)]
    fn create_memmap() -> io::Result<(PathBuf, fs::File)> {
        static MEMMAP_DIR: Mutex<usize> = Mutex::new(0);
        let mut count = MEMMAP_DIR.lock();

        if *count == 0 {
            zng_env::on_process_exit(|_| {
                IpcBytes::cleanup_memmap_storage();
            });
        }

        let dir = zng_env::cache("zng-task-ipc-mem").join(std::process::id().to_string());
        fs::create_dir_all(&dir)?;
        let mut name = dir.join(count.to_string());
        if *count < usize::MAX {
            *count += 1;
        } else {
            // very cold path, in practice the running process will die long before this
            for i in 0..usize::MAX {
                name = dir.join(i.to_string());
                if !name.exists() {
                    break;
                }
            }
            if name.exists() {
                return Err(io::Error::new(io::ErrorKind::StorageFull, ""));
            }
        };

        // read because some callers create a MmapMut
        let file = fs::OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(true)
            .open(&name)?;
        Ok((name, file))
    }
    #[cfg(ipc)]
    fn cleanup_memmap_storage() {
        if let Ok(dir) = fs::read_dir(zng_env::cache("zng-task-ipc-mem")) {
            let entries: Vec<_> = dir.flatten().map(|e| e.path()).collect();
            for entry in entries {
                if entry.is_dir() {
                    fs::remove_dir_all(entry).ok();
                }
            }
        }
    }

    /// Memory map an existing file.
    ///
    /// The `range` defines the slice of the `file` that will be mapped. Returns [`io::ErrorKind::UnexpectedEof`] if the file does not have enough bytes.
    ///
    /// # Safety
    ///
    /// Caller must ensure the `file` is not modified while all clones of the `IpcBytes` exists in the current process and others.
    ///
    /// Note that the safe [`new_memmap`] function assures safety by retaining a read lock (Windows) and restricting access rights (Unix)
    /// so that the file data is as read-only as the static data in the current executable file.
    ///
    /// [`new_memmap`]: Self::new_memmap
    #[cfg(ipc)]
    pub unsafe fn open_memmap_blocking(file: PathBuf, range: Option<ops::Range<usize>>) -> io::Result<Self> {
        let read_handle = fs::File::open(&file)?;
        read_handle.lock_shared()?;
        let len = read_handle.metadata()?.len();
        if let Some(range) = &range
            && len < range.end as u64
        {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "file length < range.end"));
        }
        // SAFETY: up to the caller.
        let map = unsafe { memmap2::Mmap::map(&read_handle) }?;

        let range = range.unwrap_or_else(|| 0..map.len());

        Ok(Self(Arc::new(IpcBytesData::MemMap(IpcMemMap {
            name: file,
            range,
            read_handle: Some(read_handle),
            is_custom: true,
            map: Some(map),
        }))))
    }
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
#[cfg(ipc)]
impl IpcMemMap {
    fn read(name: PathBuf, range: Option<ops::Range<usize>>) -> io::Result<Self> {
        let read_handle = fs::File::open(&name)?;
        read_handle.lock_shared()?;
        // SAFETY: File is marked read-only and a read lock is held for it.
        let map = unsafe { memmap2::Mmap::map(&read_handle) }?;

        let range = range.unwrap_or_else(|| 0..map.len());

        Ok(IpcMemMap {
            name,
            range,
            is_custom: false,
            read_handle: Some(read_handle),
            map: Some(map),
        })
    }
}
#[cfg(ipc)]
impl Serialize for IpcMemMap {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        (&self.name, self.range.clone()).serialize(serializer)
    }
}
#[cfg(ipc)]
impl<'de> Deserialize<'de> for IpcMemMap {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let (name, range) = <(PathBuf, ops::Range<usize>)>::deserialize(deserializer)?;
        IpcMemMap::read(name, Some(range)).map_err(|e| serde::de::Error::custom(format!("cannot load ipc memory map file, {e}")))
    }
}
#[cfg(ipc)]
impl Drop for IpcMemMap {
    fn drop(&mut self) {
        self.map.take();
        self.read_handle.take();
        if !self.is_custom {
            std::fs::remove_file(&self.name).ok();
        }
    }
}

impl Serialize for IpcBytes {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        #[cfg(ipc)]
        {
            if is_ipc_serialization() {
                match &*self.0 {
                    IpcBytesData::Heap(b) => serializer.serialize_newtype_variant("IpcBytes", 0, "Heap", serde_bytes::Bytes::new(&b[..])),
                    IpcBytesData::AnonMemMap(b) => serializer.serialize_newtype_variant("IpcBytes", 1, "AnonMemMap", b),
                    IpcBytesData::MemMap(b) => {
                        // need to keep alive until other process is also holding it, so we send
                        // a sender for the other process to signal received.
                        let (sender, mut recv) = crate::channel::ipc_unbounded::<()>()
                            .map_err(|e| serde::ser::Error::custom(format!("cannot serialize memmap bytes for ipc, {e}")))?;

                        let r = serializer.serialize_newtype_variant("IpcBytes", 2, "MemMap", &(b, sender))?;
                        let hold = self.clone();
                        crate::spawn_wait(move || {
                            if let Err(e) = recv.recv_blocking() {
                                tracing::error!("IpcBytes memmap completion signal not received, {e}")
                            }
                            drop(hold);
                        });
                        Ok(r)
                    }
                }
            } else {
                serializer.serialize_newtype_variant("IpcBytes", 0, "Heap", serde_bytes::Bytes::new(&self[..]))
            }
        }
        #[cfg(not(ipc))]
        {
            serializer.serialize_newtype_variant("IpcBytes", 0, "Heap", serde_bytes::Bytes::new(&self[..]))
        }
    }
}
impl<'de> Deserialize<'de> for IpcBytes {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        enum VariantId {
            Heap,
            #[cfg(ipc)]
            AnonMemMap,
            #[cfg(ipc)]
            MemMap,
        }

        struct EnumVisitor;
        impl<'de> serde::de::Visitor<'de> for EnumVisitor {
            type Value = IpcBytes;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "IpcBytes variant")
            }

            fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::EnumAccess<'de>,
            {
                let (variant, access) = data.variant::<VariantId>()?;
                match variant {
                    VariantId::Heap => access.newtype_variant_seed(ByteSliceVisitor),
                    #[cfg(ipc)]
                    VariantId::AnonMemMap => Ok(IpcBytes(Arc::new(IpcBytesData::AnonMemMap(access.newtype_variant()?)))),
                    #[cfg(ipc)]
                    VariantId::MemMap => {
                        let (memmap, mut completion_sender): (IpcMemMap, crate::channel::IpcSender<()>) = access.newtype_variant()?;
                        completion_sender.send_blocking(()).map_err(|e| {
                            serde::de::Error::custom(format!("cannot deserialize memmap bytes, completion signal failed, {e}"))
                        })?;
                        Ok(IpcBytes(Arc::new(IpcBytesData::MemMap(memmap))))
                    }
                }
            }
        }
        struct ByteSliceVisitor;
        impl<'de> serde::de::Visitor<'de> for ByteSliceVisitor {
            type Value = IpcBytes;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "byte buffer")
            }

            fn visit_borrowed_bytes<E>(self, v: &'de [u8]) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                IpcBytes::from_slice_blocking(v).map_err(serde::de::Error::custom)
            }

            fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                IpcBytes::from_slice_blocking(v).map_err(serde::de::Error::custom)
            }

            fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                IpcBytes::from_vec_blocking(v).map_err(serde::de::Error::custom)
            }
        }
        impl<'de> serde::de::DeserializeSeed<'de> for ByteSliceVisitor {
            type Value = IpcBytes;

            fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                deserializer.deserialize_bytes(ByteSliceVisitor)
            }
        }

        #[cfg(ipc)]
        {
            deserializer.deserialize_enum("IpcBytes", &["Heap", "AnonMemMap", "MemMap"], EnumVisitor)
        }
        #[cfg(not(ipc))]
        {
            deserializer.deserialize_enum("IpcBytes", &["Heap"], EnumVisitor)
        }
    }
}

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

/// Represents an async [`IpcBytes`] writer.
///
/// Use [`IpcBytes::new_writer`] to start writing.
pub struct IpcBytesWriter {
    inner: blocking::Unblock<IpcBytesWriterBlocking>,
}
impl IpcBytesWriter {
    /// Finish writing and move data to a shareable [`IpcBytes`].
    pub async fn finish(self) -> std::io::Result<IpcBytes> {
        let inner = self.inner.into_inner().await;
        blocking::unblock(move || inner.finish()).await
    }

    /// Mode data to an exclusive mutable [`IpcBytes`] that can be further modified, but not resized.
    pub async fn finish_mut(self) -> std::io::Result<IpcBytesMut> {
        let inner = self.inner.into_inner().await;
        blocking::unblock(move || inner.finish_mut()).await
    }
}
impl crate::io::AsyncWrite for IpcBytesWriter {
    fn poll_write(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>, buf: &[u8]) -> std::task::Poll<io::Result<usize>> {
        crate::io::AsyncWrite::poll_write(Pin::new(&mut Pin::get_mut(self).inner), cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<io::Result<()>> {
        crate::io::AsyncWrite::poll_flush(Pin::new(&mut Pin::get_mut(self).inner), cx)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<io::Result<()>> {
        crate::io::AsyncWrite::poll_close(Pin::new(&mut Pin::get_mut(self).inner), cx)
    }
}
impl crate::io::AsyncSeek for IpcBytesWriter {
    fn poll_seek(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>, pos: io::SeekFrom) -> std::task::Poll<io::Result<u64>> {
        crate::io::AsyncSeek::poll_seek(Pin::new(&mut Pin::get_mut(self).inner), cx, pos)
    }
}

/// Represents a blocking [`IpcBytes`] writer.
///
/// Use [`IpcBytes::new_writer_blocking`] to start writing.
pub struct IpcBytesWriterBlocking {
    #[cfg(ipc)]
    heap_buf: Vec<u8>,
    #[cfg(ipc)]
    memmap: Option<(PathBuf, std::fs::File)>,

    #[cfg(not(ipc))]
    heap_buf: std::io::Cursor<Vec<u8>>,
}
impl IpcBytesWriterBlocking {
    /// Finish writing and move data to a shareable [`IpcBytes`].
    pub fn finish(self) -> std::io::Result<IpcBytes> {
        let m = self.finish_mut()?;
        m.finish_blocking()
    }

    /// Mode data to an exclusive mutable [`IpcBytes`] that can be further modified, but not resized.
    pub fn finish_mut(mut self) -> std::io::Result<IpcBytesMut> {
        self.flush()?;
        #[cfg(ipc)]
        {
            let (len, inner) = match self.memmap {
                Some((name, write_handle)) => {
                    // SAFETY: we hold write lock
                    let map = unsafe { memmap2::MmapMut::map_mut(&write_handle) }?;
                    let len = map.len();
                    (len, IpcBytesMutInner::MemMap { name, map, write_handle })
                }
                None => {
                    let len = self.heap_buf.len();
                    let i = if self.heap_buf.len() > IpcBytes::INLINE_MAX {
                        IpcBytesMutInner::AnonMemMap(IpcSharedMemory::from_bytes(&self.heap_buf))
                    } else {
                        IpcBytesMutInner::Heap(self.heap_buf)
                    };
                    (len, i)
                }
            };
            Ok(IpcBytesMut { len, inner })
        }
        #[cfg(not(ipc))]
        {
            let heap_buf = self.heap_buf.into_inner();
            let len = heap_buf.len();
            let inner = IpcBytesMutInner::Heap(heap_buf);
            Ok(IpcBytesMut { len, inner })
        }
    }

    #[cfg(ipc)]
    fn alloc_memmap_file(&mut self) -> io::Result<()> {
        if self.memmap.is_none() {
            let (name, file) = IpcBytes::create_memmap()?;
            file.lock()?;
            #[cfg(unix)]
            {
                let mut permissions = file.metadata()?.permissions();
                use std::os::unix::fs::PermissionsExt;
                permissions.set_mode(0o600);
                file.set_permissions(permissions)?;
            }
            self.memmap = Some((name, file));
        }
        let file = &mut self.memmap.as_mut().unwrap().1;

        file.write_all(&self.heap_buf)?;
        // already allocated UNNAMED_MAX, continue using it as a large buffer
        self.heap_buf.clear();
        Ok(())
    }
}
impl std::io::Write for IpcBytesWriterBlocking {
    fn write(&mut self, write_buf: &[u8]) -> io::Result<usize> {
        #[cfg(ipc)]
        {
            if self.heap_buf.len() + write_buf.len() > IpcBytes::UNNAMED_MAX {
                // write exceed heap buffer, convert to memmap or flush to existing memmap
                self.alloc_memmap_file()?;

                if write_buf.len() > IpcBytes::UNNAMED_MAX {
                    // writing massive payload, skip buffer
                    self.memmap.as_mut().unwrap().1.write_all(write_buf)?;
                } else {
                    self.heap_buf.extend_from_slice(write_buf);
                }
            } else {
                if self.memmap.is_none() {
                    // heap buffer not fully allocated yet, ensure we only allocate up to UNNAMED_MAX
                    self.heap_buf
                        .reserve_exact((self.heap_buf.capacity().max(1024) * 2).min(IpcBytes::UNNAMED_MAX));
                }
                self.heap_buf.extend_from_slice(write_buf);
            }

            Ok(write_buf.len())
        }

        #[cfg(not(ipc))]
        {
            std::io::Write::write(&mut self.heap_buf, write_buf)
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        #[cfg(ipc)]
        if let Some((_, file)) = &mut self.memmap {
            if !self.heap_buf.is_empty() {
                file.write_all(&self.heap_buf)?;
                self.heap_buf.clear();
            }
            file.flush()?;
        }
        Ok(())
    }
}
impl std::io::Seek for IpcBytesWriterBlocking {
    fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
        #[cfg(ipc)]
        {
            self.alloc_memmap_file()?;
            let (_, file) = self.memmap.as_mut().unwrap();
            if !self.heap_buf.is_empty() {
                file.write_all(&self.heap_buf)?;
                self.heap_buf.clear();
            }
            file.seek(pos)
        }
        #[cfg(not(ipc))]
        {
            std::io::Seek::seek(&mut self.heap_buf, pos)
        }
    }
}

enum IpcBytesMutInner {
    Heap(Vec<u8>),
    #[cfg(ipc)]
    AnonMemMap(IpcSharedMemory),
    #[cfg(ipc)]
    MemMap {
        name: PathBuf,
        map: memmap2::MmapMut,
        write_handle: std::fs::File,
    },
}

/// Represents preallocated exclusive mutable memory for a new [`IpcBytes`].
///
/// Use [`IpcBytes::new_mut`] or [`IpcBytes::new_mut_blocking`] to allocate.
pub struct IpcBytesMut {
    inner: IpcBytesMutInner,
    len: usize,
}
impl ops::Deref for IpcBytesMut {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        let len = self.len;
        match &self.inner {
            IpcBytesMutInner::Heap(v) => &v[..len],
            #[cfg(ipc)]
            IpcBytesMutInner::AnonMemMap(m) => &m[..len],
            #[cfg(ipc)]
            IpcBytesMutInner::MemMap { map, .. } => &map[..len],
        }
    }
}
impl ops::DerefMut for IpcBytesMut {
    fn deref_mut(&mut self) -> &mut Self::Target {
        let len = self.len;
        match &mut self.inner {
            IpcBytesMutInner::Heap(v) => &mut v[..len],
            #[cfg(ipc)]
            IpcBytesMutInner::AnonMemMap(m) => {
                // SAFETY: we are the only reference to the map
                unsafe { m.deref_mut() }
            }
            #[cfg(ipc)]
            IpcBytesMutInner::MemMap { map, .. } => &mut map[..len],
        }
    }
}
impl fmt::Debug for IpcBytesMut {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "IpcBytesMut(<{} bytes>)", self.len())
    }
}
impl IpcBytesMut {
    /// Uses `buf` or copies it to exclusive mutable memory.
    pub async fn from_vec(buf: Vec<u8>) -> io::Result<Self> {
        #[cfg(ipc)]
        if buf.len() <= IpcBytes::INLINE_MAX {
            Ok(Self {
                len: buf.len(),
                inner: IpcBytesMutInner::Heap(buf),
            })
        } else {
            blocking::unblock(move || {
                let mut b = IpcBytes::new_mut_blocking(buf.len())?;
                b[..].copy_from_slice(&buf);
                Ok(b)
            })
            .await
        }
        #[cfg(not(ipc))]
        {
            Ok(Self {
                len: buf.len(),
                inner: IpcBytesMutInner::Heap(buf),
            })
        }
    }

    /// Convert to immutable shareable [`IpcBytes`].
    pub async fn finish(mut self) -> io::Result<IpcBytes> {
        let len = self.len;
        let data = match std::mem::replace(&mut self.inner, IpcBytesMutInner::Heap(vec![])) {
            IpcBytesMutInner::Heap(mut v) => {
                v.truncate(len);
                v.shrink_to_fit();
                IpcBytesData::Heap(v)
            }
            #[cfg(ipc)]
            IpcBytesMutInner::AnonMemMap(m) => {
                if len < IpcBytes::INLINE_MAX {
                    IpcBytesData::Heap(m[..len].to_vec())
                } else if len < m.len() {
                    IpcBytesData::AnonMemMap(IpcSharedMemory::from_bytes(&m[..len]))
                } else {
                    IpcBytesData::AnonMemMap(m)
                }
            }
            #[cfg(ipc)]
            IpcBytesMutInner::MemMap { name, map, write_handle } => {
                let len = self.len;
                blocking::unblock(move || Self::finish_memmap(name, map, write_handle, len)).await?
            }
        };
        Ok(IpcBytes(Arc::new(data)))
    }

    #[cfg(ipc)]
    fn finish_memmap(name: PathBuf, map: memmap2::MmapMut, write_handle: fs::File, len: usize) -> Result<IpcBytesData, io::Error> {
        let alloc_len = map.len();
        if alloc_len != len {
            write_handle.set_len(len as u64)?;
        }
        write_handle.unlock()?;
        let map = if alloc_len != len {
            drop(map);
            // SAFETY: we have write access to the file still
            unsafe { memmap2::Mmap::map(&write_handle) }?
        } else {
            map.make_read_only()?
        };
        let mut permissions = write_handle.metadata()?.permissions();
        permissions.set_readonly(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            permissions.set_mode(0o400);
        }
        write_handle.set_permissions(permissions)?;
        drop(write_handle);
        let read_handle = std::fs::File::open(&name)?;
        read_handle.lock_shared()?;
        Ok(IpcBytesData::MemMap(IpcMemMap {
            name,
            range: 0..len,
            is_custom: false,
            map: Some(map),
            read_handle: Some(read_handle),
        }))
    }
}
impl IpcBytesMut {
    /// Uses `buf` or copies it to exclusive mutable memory.
    pub fn from_vec_blocking(buf: Vec<u8>) -> io::Result<Self> {
        #[cfg(ipc)]
        if buf.len() <= IpcBytes::INLINE_MAX {
            Ok(Self {
                len: buf.len(),
                inner: IpcBytesMutInner::Heap(buf),
            })
        } else {
            let mut b = IpcBytes::new_mut_blocking(buf.len())?;
            b[..].copy_from_slice(&buf);
            Ok(b)
        }
        #[cfg(not(ipc))]
        {
            Ok(Self {
                len: buf.len(),
                inner: IpcBytesMutInner::Heap(buf),
            })
        }
    }

    /// Convert to immutable shareable [`IpcBytes`].
    pub fn finish_blocking(mut self) -> io::Result<IpcBytes> {
        let len = self.len;
        let data = match std::mem::replace(&mut self.inner, IpcBytesMutInner::Heap(vec![])) {
            IpcBytesMutInner::Heap(mut v) => {
                v.truncate(len);
                IpcBytesData::Heap(v)
            }
            #[cfg(ipc)]
            IpcBytesMutInner::AnonMemMap(m) => {
                if len < IpcBytes::INLINE_MAX {
                    IpcBytesData::Heap(m[..len].to_vec())
                } else if len < m.len() {
                    IpcBytesData::AnonMemMap(IpcSharedMemory::from_bytes(&m[..len]))
                } else {
                    IpcBytesData::AnonMemMap(m)
                }
            }
            #[cfg(ipc)]
            IpcBytesMutInner::MemMap { name, map, write_handle } => Self::finish_memmap(name, map, write_handle, len)?,
        };
        Ok(IpcBytes(Arc::new(data)))
    }
}
#[cfg(ipc)]
impl Drop for IpcBytesMut {
    fn drop(&mut self) {
        if let IpcBytesMutInner::MemMap { name, map, write_handle } = std::mem::replace(&mut self.inner, IpcBytesMutInner::Heap(vec![])) {
            drop(map);
            drop(write_handle);
            std::fs::remove_file(name).ok();
        }
    }
}

/// Safe bytemuck casting wrapper for [`IpcBytesMut`].
///
/// Use [`IpcBytesMut::cast`] to cast.
pub struct IpcBytesMutCast<T: bytemuck::AnyBitPattern> {
    bytes: IpcBytesMut,
    _t: PhantomData<T>,
}
impl<T: bytemuck::AnyBitPattern> ops::Deref for IpcBytesMutCast<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        bytemuck::cast_slice::<u8, T>(&self.bytes)
    }
}
impl<T: bytemuck::AnyBitPattern + bytemuck::NoUninit> ops::DerefMut for IpcBytesMutCast<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        bytemuck::cast_slice_mut::<u8, T>(&mut self.bytes)
    }
}
impl<T: bytemuck::AnyBitPattern> IpcBytesMutCast<T> {
    /// Convert back to [`IpcBytesMut`].
    pub fn into_inner(self) -> IpcBytesMut {
        self.bytes
    }
}
impl<T: bytemuck::AnyBitPattern> From<IpcBytesMutCast<T>> for IpcBytesMut {
    fn from(value: IpcBytesMutCast<T>) -> Self {
        value.bytes
    }
}
impl IpcBytesMut {
    /// Safe bytemuck casting wrapper.
    ///
    /// The wrapper will deref to `[T]` and can be converted back to `IpcBytesMust`.
    ///
    /// # Panics
    ///
    /// Panics if cannot cast, se [bytemuck docs] for details.
    ///
    /// [bytemuck docs]: https://docs.rs/bytemuck/1.24.0/bytemuck/fn.try_cast_slice.html
    pub fn cast<T: bytemuck::AnyBitPattern>(self) -> IpcBytesMutCast<T> {
        let r = IpcBytesMutCast {
            bytes: self,
            _t: PhantomData,
        };
        let _assert = &r[..];
        r
    }

    /// Safe bytemuck cast to slice.
    ///
    /// # Panics
    ///
    /// Panics if cannot cast, se [bytemuck docs] for details.
    ///
    /// [bytemuck docs]: https://docs.rs/bytemuck/1.24.0/bytemuck/fn.try_cast_slice.html
    pub fn cast_deref<T: bytemuck::AnyBitPattern>(&self) -> &[T] {
        bytemuck::cast_slice(self)
    }

    /// Safe bytemuck cast to mutable slice.
    ///
    /// # Panics
    ///
    /// Panics if cannot cast, se [bytemuck docs] for details.
    ///
    /// [bytemuck docs]: https://docs.rs/bytemuck/1.24.0/bytemuck/fn.try_cast_slice.html
    pub fn cast_deref_mut<T: bytemuck::AnyBitPattern + bytemuck::NoUninit>(&mut self) -> &mut [T] {
        bytemuck::cast_slice_mut(self)
    }
}

/// Safe bytemuck casting wrapper for [`IpcBytes`].
///
/// Use [`IpcBytes::cast`] to cast.
pub struct IpcBytesCast<T: bytemuck::AnyBitPattern> {
    bytes: IpcBytes,
    _t: PhantomData<T>,
}
impl<T: bytemuck::AnyBitPattern> ops::Deref for IpcBytesCast<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        bytemuck::cast_slice::<u8, T>(&self.bytes)
    }
}
impl<T: bytemuck::AnyBitPattern> IpcBytesCast<T> {
    /// Convert back to [`IpcBytes`].
    pub fn into_inner(self) -> IpcBytes {
        self.bytes
    }
}
impl<T: bytemuck::AnyBitPattern> From<IpcBytesCast<T>> for IpcBytes {
    fn from(value: IpcBytesCast<T>) -> Self {
        value.bytes
    }
}
impl IpcBytes {
    /// Safe bytemuck casting wrapper.
    ///
    /// The wrapper will deref to `[T]` and can be converted back to `IpcBytes`.
    ///
    /// # Panics
    ///
    /// Panics if cannot cast, se [bytemuck docs] for details.
    ///
    /// [bytemuck docs]: https://docs.rs/bytemuck/1.24.0/bytemuck/fn.try_cast_slice.html
    pub fn cast<T: bytemuck::AnyBitPattern>(self) -> IpcBytesCast<T> {
        let r = IpcBytesCast {
            bytes: self,
            _t: PhantomData,
        };
        let _assert = &r[..];
        r
    }

    /// Safe bytemuck cast to slice.
    ///
    /// # Panics
    ///
    /// Panics if cannot cast, se [bytemuck docs] for details.
    ///
    /// [bytemuck docs]: https://docs.rs/bytemuck/1.24.0/bytemuck/fn.try_cast_slice.html
    pub fn cast_deref<T: bytemuck::AnyBitPattern>(&self) -> &[T] {
        bytemuck::cast_slice(self)
    }
}

impl IpcBytesMut {
    /// Shorten the bytes length.
    ///
    /// If `new_len` is greater or equal to current length does nothing.
    ///
    /// Note that this does not affect memory allocation, the extra bytes are only dropped on finish.
    pub fn truncate(&mut self, new_len: usize) {
        self.len = self.len.min(new_len);
    }

    /// Convert chunks of length `L0` to chunks of length `L1` where `L1 <= L0`.
    ///
    /// Reuses the same allocation for the new data, replacing in place. Truncates the final buffer to the new length.
    ///
    /// Note that no memory is released by this call as truncated is lazy and applied on finish.
    ///
    /// # Panics
    ///
    /// Panics if `L1 > L0` or if bytes length is not multiple of `L0`.
    pub fn reduce_in_place<const L0: usize, const L1: usize>(&mut self, mut reduce: impl FnMut([u8; L0]) -> [u8; L1]) {
        assert!(L1 <= L0);

        let self_ = &mut self[..];

        let len = self_.len();
        if len == 0 {
            return;
        }
        assert!(len.is_multiple_of(L0), "length must be multiple of L0");

        let ptr = self_.as_mut_ptr();
        let mut write = 0usize;
        let mut read = 0usize;

        // SAFETY: pointers stay inside slice, in_chunk and out_chunk copy never overlaps with slice.
        unsafe {
            while read < len {
                let mut in_chunk = MaybeUninit::<[u8; L0]>::uninit();
                std::ptr::copy_nonoverlapping(ptr.add(read), (*in_chunk.as_mut_ptr()).as_mut_ptr(), L0);
                read += L0;

                let out_chunk = reduce(in_chunk.assume_init());

                std::ptr::copy_nonoverlapping(out_chunk.as_ptr(), ptr.add(write), L1);
                write += L1;
            }
        }

        self.truncate(write);
    }

    /// Convert chunks of `in_chunk_len` to chunks of `out_chunk_buf.len()` where `out_chunk_buf.len() <= in_chunk_len`.
    ///
    /// Reuses the same allocation for the new data, replacing in place. Truncates the final buffer to the new length.
    ///
    /// Note that no memory is released by this call as truncated is lazy and applied on finish.
    ///
    /// # Panics
    ///
    /// Panics if `out_chunk_buf.len() > in_chunk_len` or if bytes length is not multiple of `in_chunk_len`.
    pub fn reduce_in_place_dyn(&mut self, in_chunk_len: usize, out_chunk_buf: &mut [u8], mut reduce: impl FnMut(&[u8], &mut [u8])) {
        assert!(out_chunk_buf.len() < in_chunk_len);

        let self_ = &mut self[..];

        let len = self_.len();
        if len == 0 {
            return;
        }
        assert!(len.is_multiple_of(in_chunk_len), "length must be multiple of in_chunk_len");

        let ptr = self_.as_mut_ptr();
        let mut write = 0usize;
        let mut read = 0usize;

        // SAFETY: pointers stay inside slice, in_chunk and out_chunk copy never overlaps with slice.
        unsafe {
            while read < len {
                reduce(std::slice::from_raw_parts(ptr.add(read), in_chunk_len), &mut *out_chunk_buf);
                read += in_chunk_len;

                std::ptr::copy_nonoverlapping(out_chunk_buf.as_ptr(), ptr.add(write), out_chunk_buf.len());
                write += out_chunk_buf.len();
            }
        }

        self.truncate(write);
    }

    /// Convert chunks of length `L0` to chunks of length `L1` where `size_of::<T1>() * L1 <= size_of::<T0>() * L0`.
    ///
    /// Reuses the same allocation for the new data, replacing in place. Truncates the final buffer to the new length.
    ///
    /// Note that no memory is released by this call as truncated is lazy and applied on finish.
    ///
    /// # Panics
    ///
    /// Panics if `size_of::<T1>() * L1 > size_of::<T0>() * L0` or if bytes length is not multiple of `size_of::<T0>() * L0`.
    pub fn cast_reduce_in_place<T0, const L0: usize, T1, const L1: usize>(&mut self, mut reduce: impl FnMut([T0; L0]) -> [T1; L1])
    where
        T0: bytemuck::AnyBitPattern,
    {
        let l0 = std::mem::size_of::<T0>() * L0;
        let l1 = std::mem::size_of::<T1>() * L1;
        assert!(l1 <= l0);

        let self_ = &mut self[..];

        let len = self_.len();
        if len == 0 {
            return;
        }
        assert!(len.is_multiple_of(l0), "length must be multiple of size_of::<T0>() * L0");

        let ptr = self_.as_mut_ptr();
        let mut write = 0usize;
        let mut read = 0usize;

        // SAFETY:
        // Pointers stay inside slice, in_chunk and out_chunk copy never overlaps with slice.
        // Reading [T0; L0] from raw bytes is safe because T0: AnyBitPattern
        unsafe {
            while read < len {
                let mut in_chunk = MaybeUninit::<[T0; L0]>::uninit();
                std::ptr::copy_nonoverlapping(ptr.add(read), (*in_chunk.as_mut_ptr()).as_mut_ptr() as _, l0);
                read += l0;

                let out_chunk = reduce(in_chunk.assume_init());

                std::ptr::copy_nonoverlapping(out_chunk.as_ptr() as _, ptr.add(write), l1);
                write += l1;
            }
        }

        self.truncate(write);
    }

    /// Convert chunks of `size_of::<T0>() * in_chunk_len` to chunks of `size_of::<T1>() * out_chunk_buf.len()`
    /// where `size_of::<T1>() * out_chunk_buf.len() <= size_of::<T0>() * in_chunk_len`.
    ///
    /// Reuses the same allocation for the new data, replacing in place. Truncates the final buffer to the new length.
    ///
    /// Note that no memory is released by this call as truncated is lazy and applied on finish.
    ///
    /// # Panics
    ///
    /// Panics if `size_of::<T1>() * out_chunk_buf.len() > size_of::<T0>() * in_chunk_len` or if bytes
    /// length is not multiple of `size_of::<T0>() * in_chunk_len`.
    pub fn cast_reduce_in_place_dyn<T0, T1>(
        &mut self,
        in_chunk_len: usize,
        out_chunk_buf: &mut [T1],
        mut reduce: impl FnMut(&[T0], &mut [T1]),
    ) where
        T0: bytemuck::AnyBitPattern,
    {
        let l0 = std::mem::size_of::<T0>() * in_chunk_len;
        let l1 = std::mem::size_of_val(out_chunk_buf);

        assert!(l1 <= l0);

        let self_ = &mut self[..];

        let len = self_.len();
        if len == 0 {
            return;
        }
        assert!(len.is_multiple_of(l0), "length must be multiple of size_of::<T0>() * in_chunk_len");

        let ptr = self_.as_mut_ptr();
        let mut write = 0usize;
        let mut read = 0usize;

        // SAFETY: pointers stay inside slice, in_chunk and out_chunk copy never overlaps with slice.
        unsafe {
            while read < len {
                reduce(
                    bytemuck::cast_slice(std::slice::from_raw_parts(ptr.add(read), l0)),
                    &mut *out_chunk_buf,
                );
                read += l0;

                std::ptr::copy_nonoverlapping(out_chunk_buf.as_ptr() as _, ptr.add(write), l1);
                write += l1;
            }
        }

        self.truncate(write);
    }

    /// Reverses the order of chunks in the slice, in place.
    ///
    /// Chunk length is const L.
    ///
    /// # Panics
    ///
    /// Panics if length is not multiple of `L`.
    pub fn reverse_chunks<const L: usize>(&mut self) {
        let self_ = &mut self[..];

        let len = self_.len();

        if len == 0 || L == 0 {
            return;
        }

        if L == 1 {
            return self_.reverse();
        }

        assert!(len.is_multiple_of(L), "length must be multiple of L");

        // SAFETY: already verified is multiple and already handled L=0
        unsafe { self_.as_chunks_unchecked_mut::<L>() }.reverse();
    }

    /// Reverses the order of chunks in the slice, in place.
    ///
    /// # Panics
    ///
    /// Panics if length is not multiple of `chunk_len`.
    pub fn reverse_chunks_dyn(&mut self, chunk_len: usize) {
        let self_ = &mut self[..];

        let len = self_.len();

        if len == 0 || chunk_len == 0 {
            return;
        }

        if chunk_len == 1 {
            return self_.reverse();
        }

        assert!(len.is_multiple_of(chunk_len), "length must be multiple of chunk_len");

        let mut a = 0;
        let mut b = len - chunk_len;

        let ptr = self_.as_mut_ptr();

        // SAFETY: chunks are not overlapping and loop stops before at mid, chunk_len > 1
        unsafe {
            while a < b {
                std::ptr::swap_nonoverlapping(ptr.add(a), ptr.add(b), chunk_len);
                a += chunk_len;
                b -= chunk_len;
            }
        }
    }
}
