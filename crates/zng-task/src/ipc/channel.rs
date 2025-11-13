use std::{
    fmt, fs,
    io::{self, Read, Write},
    ops,
    path::{Path, PathBuf},
    sync::Arc,
};

use ipc_channel::ipc::IpcSharedMemory;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use tracing::field::Field;

/// Immutable shared memory reference that can be send fast over IPC.
#[derive(Clone)]
pub struct IpcBytes(Arc<IpcBytesData>);
enum IpcBytesData {
    Inline(Vec<u8>),
    UnnamedSharedMemory(IpcSharedMemory),
    NamedSharedMemory(IpcNamedSharedMemory),
}
struct IpcNamedSharedMemory {
    name: PathBuf,
    read_handle: fs::File,
    map: memmap2::Mmap,
    range: ops::Range<usize>,
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
            IpcBytesData::Inline(i) => i,
            IpcBytesData::UnnamedSharedMemory(m) => m,
            IpcBytesData::NamedSharedMemory(f) => &f.map,
        }
    }
}
impl IpcBytes {
    /// New empty.
    pub fn empty() -> Self {
        IpcBytes(Arc::new(IpcBytesData::Inline(vec![])))
    }

    /// Copy data from slice.
    pub fn from_slice(data: &[u8]) -> io::Result<Self> {
        let data = if data.len() <= Self::INLINE_MAX {
            IpcBytesData::Inline(data.to_vec())
        } else if data.len() <= Self::UNNAMED_MAX {
            IpcBytesData::UnnamedSharedMemory(IpcSharedMemory::from_bytes(data))
        } else {
            todo!()
        };
        Ok(Self(Arc::new(data)))
    }

    /// Copy or move data from vector.
    pub fn from_vec(data: Vec<u8>) -> io::Result<Self> {
        if data.len() <= Self::INLINE_MAX {
            Ok(Self(Arc::new(IpcBytesData::Inline(data))))
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
                        return Ok(Self(Arc::new(IpcBytesData::Inline(buf))));
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
                        return Ok(Self(Arc::new(IpcBytesData::UnnamedSharedMemory(IpcSharedMemory::from_bytes(
                            &buf[..len],
                        )))));
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
    /// Note that this enforces the use of a file, the slowest option and only used huge data payloads by the other constructor functions.
    pub fn new_memmap(write: impl FnOnce(&mut fs::File) -> io::Result<()>) -> io::Result<Self> {
        let (name, mut file) = Self::create_memmap()?;
        write(&mut file)?;
        drop(file);
        let read_handle = fs::File::open(&name)?;
        read_handle.lock_shared()?;
        // SAFETY: !!: TODO, more access restrictions, security
        let map = unsafe { memmap2::Mmap::map(&read_handle) }?;
        let range = 0..map.len();
        Ok(Self(Arc::new(IpcBytesData::NamedSharedMemory(IpcNamedSharedMemory {
            name,
            read_handle,
            map,
            range,
        }))))
    }
    fn create_memmap() -> io::Result<(PathBuf, fs::File)> {
        static MEMMAP_DIR: Mutex<usize> = Mutex::new(0);
        let mut count = MEMMAP_DIR.lock();

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
    pub unsafe fn open_memmap(file: PathBuf, range: ops::Range<usize>) -> io::Result<Self> {
        let read_handle = fs::File::open(&file)?;
        read_handle.lock_shared()?;
        let len = read_handle.metadata()?.len();
        if len < range.end as u64 {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "file length < range.end"));
        }
        // SAFETY: up to the caller.
        let map = unsafe { memmap2::Mmap::map(&read_handle) }?;

        Ok(Self(Arc::new(IpcBytesData::NamedSharedMemory(IpcNamedSharedMemory {
            name: file,
            read_handle,
            map,
            range,
        }))))
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
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for IpcBytes {}

/// The transmitting end of an IPC channel.
pub struct IpcSender<T: Serialize> {
    sender: ipc_channel::ipc::IpcSender<T>,
}

/// The receiving end of an IPC channel.
pub struct IpcReceiver<T: DeserializeOwned> {
    recv: ipc_channel::ipc::IpcReceiver<T>,
}
