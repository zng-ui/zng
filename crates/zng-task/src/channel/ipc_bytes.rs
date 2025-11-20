#![cfg_attr(not(ipc), allow(unused))]

use std::{
    cell::Cell,
    fmt, fs,
    io::{self, Read, Write},
    ops,
    path::{Path, PathBuf},
    sync::{Arc, Weak},
};

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

    /// Copy data from slice.
    pub fn from_slice(data: &[u8]) -> io::Result<Self> {
        #[cfg(ipc)]
        {
            if data.len() <= Self::INLINE_MAX {
                Ok(Self(Arc::new(IpcBytesData::Heap(data.to_vec()))))
            } else if data.len() <= Self::UNNAMED_MAX {
                Ok(Self(Arc::new(IpcBytesData::AnonMemMap(IpcSharedMemory::from_bytes(data)))))
            } else {
                Self::new_memmap(|m| m.write_all(data))
            }
        }
        #[cfg(not(ipc))]
        {
            Ok(Self(Arc::new(IpcBytesData::Heap(data.to_vec()))))
        }
    }

    /// Copy or move data from vector.
    pub fn from_vec(data: Vec<u8>) -> io::Result<Self> {
        #[cfg(ipc)]
        {
            if data.len() <= Self::INLINE_MAX {
                Ok(Self(Arc::new(IpcBytesData::Heap(data))))
            } else {
                Self::from_slice(&data)
            }
        }
        #[cfg(not(ipc))]
        {
            Ok(Self(Arc::new(IpcBytesData::Heap(data))))
        }
    }

    /// Read `data` into shared memory.
    pub fn from_read(data: &mut dyn io::Read) -> io::Result<Self> {
        #[cfg(ipc)]
        {
            Self::from_read_ipc(data)
        }
        #[cfg(not(ipc))]
        {
            let mut buf = vec![];
            data.read_to_end(&mut buf)?;
            Self::from_vec(buf)
        }
    }
    #[cfg(ipc)]
    fn from_read_ipc(data: &mut dyn io::Read) -> io::Result<Self> {
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
        Self::new_memmap(|m| {
            m.write_all(&buf)?;
            io::copy(data, m)?;
            Ok(())
        })
    }

    /// Read `file` into shared memory.
    pub fn from_file(file: &Path) -> io::Result<Self> {
        #[cfg(ipc)]
        {
            let mut file = fs::File::open(file)?;
            let len = file.metadata()?.len();
            if len <= Self::UNNAMED_MAX as u64 {
                let mut buf = vec![0u8; len as usize];
                file.read_exact(&mut buf)?;
                Self::from_vec(buf)
            } else {
                Self::new_memmap(|m| {
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
            Self::from_vec(buf)
        }
    }

    /// Create a memory mapped file.
    ///
    /// Note that the `from_` functions select optimized backing storage depending on data length, this function
    /// always selects the slowest options, a file backed memory map.
    #[cfg(ipc)]
    pub fn new_memmap(write: impl FnOnce(&mut fs::File) -> io::Result<()>) -> io::Result<Self> {
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

        let file = fs::File::create(&name)?;
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
    pub unsafe fn open_memmap(file: PathBuf, range: Option<ops::Range<usize>>) -> io::Result<Self> {
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
                IpcBytes::from_slice(v).map_err(serde::de::Error::custom)
            }

            fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                IpcBytes::from_slice(v).map_err(serde::de::Error::custom)
            }

            fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                IpcBytes::from_vec(v).map_err(serde::de::Error::custom)
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
