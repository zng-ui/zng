use std::{
    cell::Cell,
    fmt, fs,
    io::{self, Read, Write},
    ops,
    path::{Path, PathBuf},
    sync::Arc,
};

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
/// # Serialization
///
/// When serialized inside [`ipc_serialization_context`] the memory map bytes are not copied, only the system handle and metadata is serialized.
/// When serialized in other contexts all bytes are copied.
///
/// When deserializing memory map handles are reconnected and if deserializing bytes selects the best storage based on data length.
///
/// [`IpcSender`]: super::IpcSender
#[derive(Clone)]
pub struct IpcBytes(Arc<IpcBytesData>);
enum IpcBytesData {
    Heap(Vec<u8>),
    AnonMemMap(IpcSharedMemory),
    MemMap(IpcMemMap),
}
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
            IpcBytesData::AnonMemMap(m) => m,
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
        let data = if data.len() <= Self::INLINE_MAX {
            IpcBytesData::Heap(data.to_vec())
        } else if data.len() <= Self::UNNAMED_MAX {
            IpcBytesData::AnonMemMap(IpcSharedMemory::from_bytes(data))
        } else {
            todo!()
        };
        Ok(Self(Arc::new(data)))
    }

    /// Copy or move data from vector.
    pub fn from_vec(data: Vec<u8>) -> io::Result<Self> {
        if data.len() <= Self::INLINE_MAX {
            Ok(Self(Arc::new(IpcBytesData::Heap(data))))
        } else {
            Self::from_slice(&data)
        }
    }

    /// Read `data` into shared memory.
    pub fn from_read(data: &mut dyn io::Read) -> io::Result<Self> {
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

    /// Create a memory mapped file.
    ///
    /// Note that the `from_` functions select optimized backing storage depending on data length, this function
    /// always selects the slowest options, a file backed memory map.
    pub fn new_memmap(write: impl FnOnce(&mut fs::File) -> io::Result<()>) -> io::Result<Self> {
        let (name, mut file) = Self::create_memmap()?;
        write(&mut file)?;
        drop(file);
        let map = IpcMemMap::new(name, None)?;
        Ok(Self(Arc::new(IpcBytesData::MemMap(map))))
    }
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

    const INLINE_MAX: usize = 64 * 1024; // 64KB
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
impl IpcMemMap {
    fn new(name: PathBuf, range: Option<ops::Range<usize>>) -> io::Result<Self> {
        let read_handle = fs::File::open(&name)?;
        let mut permissions = read_handle.metadata()?.permissions();
        permissions.set_readonly(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            permissions.set_mode(400);
        }
        read_handle.set_permissions(permissions)?;
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
impl Serialize for IpcMemMap {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        (&self.name, self.range.clone()).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for IpcMemMap {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let (name, range) = <(PathBuf, ops::Range<usize>)>::deserialize(deserializer)?;
        IpcMemMap::new(name, Some(range)).map_err(|e| serde::de::Error::custom(format!("cannot load ipc memory map file, {e}")))
    }
}
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
        if is_ipc_serialization_context() {
            match &*self.0 {
                IpcBytesData::Heap(b) => serializer.serialize_newtype_variant("IpcBytes", 0, "Heap", serde_bytes::Bytes::new(&b[..])),
                IpcBytesData::AnonMemMap(b) => serializer.serialize_newtype_variant("IpcBytes", 1, "AnonMemMap", b),
                IpcBytesData::MemMap(b) => serializer.serialize_newtype_variant("IpcBytes", 2, "MemMap", b),
            }
        } else {
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
            AnonMemMap,
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
                    VariantId::AnonMemMap => Ok(IpcBytes(Arc::new(IpcBytesData::AnonMemMap(access.newtype_variant()?)))),
                    VariantId::MemMap => Ok(IpcBytes(Arc::new(IpcBytesData::MemMap(access.newtype_variant()?)))),
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

        deserializer.deserialize_enum("IpcBytes", &["Heap", "AnonMemMap", "MemMap"], EnumVisitor)
    }
}

/// Enables special serialization of memory mapped files for the `serialize` call.
///
/// IPC channels like [`IpcSender`] serialize messages inside this context to support [`IpcBytes`] fast memory map sharing across processes.
///
/// You can use the [`is_ipc_serialization_context`] to check if inside context.
///
/// [`IpcSender`]: super::IpcSender
pub fn ipc_serialization_context<R>(serialize: impl FnOnce() -> R) -> R {
    let parent = IPC_SERIALIZATION_CONTEXT.replace(true);
    RunOnDrop::new(|| IPC_SERIALIZATION_CONTEXT.set(parent));
    serialize()
}

/// Checks if is inside [`ipc_serialization_context`].
pub fn is_ipc_serialization_context() -> bool {
    IPC_SERIALIZATION_CONTEXT.get()
}

thread_local! {
    static IPC_SERIALIZATION_CONTEXT: Cell<bool> = const { Cell::new(false) };
}
