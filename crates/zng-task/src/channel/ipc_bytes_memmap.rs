use std::{
    fs,
    io::{self, Seek},
    ops,
    path::PathBuf,
    sync::{
        OnceLock,
        atomic::{AtomicUsize, Ordering},
    },
};

use serde::{Deserialize, Serialize};

use crate::channel::IpcFileHandle;

pub(super) struct MemmapMut {
    range: ops::Range<u64>,
    map: memmap2::MmapMut,
    handle: fs::File,
}
impl ops::Deref for MemmapMut {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.map
    }
}
impl ops::DerefMut for MemmapMut {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.map
    }
}
impl MemmapMut {
    /// Open an existing file with read/write access, exclusive lock it and map to mutable memory.
    ///
    /// # Safety
    ///
    /// Caller must ensure the file is only accessed by the current process while the memory map is in use.
    pub(super) unsafe fn write_user_file(path: PathBuf, range: Option<ops::Range<u64>>) -> io::Result<Self> {
        let file = fs::OpenOptions::new().read(true).write(true).open(path)?;
        file.lock()?;
        let len = file.metadata()?.len();
        let range = match range {
            Some(r) => {
                if len < r.end - r.start {
                    return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "cannot map range, file too small"));
                } else {
                    r
                }
            }
            None => 0..len,
        };
        let len = match usize::try_from(range.end - range.start) {
            Ok(l) => l,
            Err(_) => return Err(io::Error::new(io::ErrorKind::FileTooLarge, "cannot map more than usize::MAX")),
        };
        let mut opt = memmap2::MmapOptions::new();
        opt.offset(range.start).len(len);
        // SAFETY: up to the caller
        let map = unsafe { opt.map_mut(&file) }?;

        Ok(Self { range, handle: file, map })
    }

    /// Create or truncate a file, resize it to `len`, exclusive lock it and map to mutable memory.
    ///
    /// # Safety
    ///
    /// Caller must ensure the file is only accessed by the current process while the memory map is in use.
    pub(super) unsafe fn create_user_file(path: PathBuf, len: usize) -> io::Result<Self> {
        let file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)?;
        file.lock()?;
        file.set_len(len as u64)?;
        let mut opt = memmap2::MmapOptions::new();
        opt.len(len);
        // SAFETY: up to the caller
        let map = unsafe { opt.map_mut(&file) }?;

        Ok(Self {
            range: 0..len as u64,
            handle: file,
            map,
        })
    }

    pub(super) fn begin_write() -> io::Result<fs::File> {
        static TMP: OnceLock<PathBuf> = OnceLock::new();
        static TMP_ID: AtomicUsize = AtomicUsize::new(0);

        let file_path = TMP
            .get_or_init(|| {
                // * We prefer the `zng::env::cache` over $TMP,  because if is configurable by the app.
                // * We use $TMP if cache is not on the same disk as the executable.
                // * Goal is to avoid a disk that can be unmounted while the app is executing.

                let mut path = if let Ok(exe) = std::env::current_exe()
                    && let Ok(exe) = exe.canonicalize()
                    && let Some(exe) = exe.parent()
                    && let Ok(mut p) = zng_env::cache("").canonicalize()
                {
                    #[cfg(windows)]
                    if p.components().next() != exe.components().next() {
                        p = std::env::temp_dir();
                    }
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::MetadataExt as _;
                        if let Ok(m1) = fs::metadata(&p)
                            && let Ok(m2) = fs::metadata(&exe)
                            && m1.dev() == m2.dev()
                        {
                            // same disk, ok
                        } else {
                            p = std::env::temp_dir();
                        }
                    }
                    p
                } else {
                    std::env::temp_dir()
                };

                path.push(format!("zng-task-channel-{}", std::process::id()));
                path
            })
            .with_added_extension(format!("{}.mmap", TMP_ID.fetch_add(1, Ordering::Relaxed)));

        let mut opt = fs::OpenOptions::new();
        #[cfg(windows)]
        {
            use std::os::windows::prelude::*;
            use windows_sys::Win32::Storage::FileSystem::*;

            opt.attributes(FILE_ATTRIBUTE_TEMPORARY).custom_flags(FILE_FLAG_DELETE_ON_CLOSE);
        }
        let file = opt.read(true).write(true).create_new(true).open(file_path)?;
        file.lock()?;
        #[cfg(unix)]
        {
            let _ = fs::remove_file(&file_path);
        }
        Ok(file)
    }

    pub(super) fn end_write(file: fs::File) -> io::Result<Self> {
        let len = file.metadata()?.len();
        if len > usize::MAX as u64 {
            return Err(io::Error::new(io::ErrorKind::FileTooLarge, "cannot map more than usize::MAX"));
        }
        let mut opt = memmap2::MmapOptions::new();
        opt.len(len as usize);
        // SAFETY:
        //  - No other user process can access the file.
        //  - We selected the disk least likely to be unmounted.
        let map = unsafe { opt.map_mut(&file) }?;

        Ok(Self {
            range: 0..len,
            handle: file,
            map,
        })
    }

    /// Create a new safe memory map.
    pub(super) fn new(len: usize) -> io::Result<Self> {
        let file = Self::begin_write()?;
        file.set_len(len as u64)?;
        Self::end_write(file)
    }

    /// Downgrade lock to shared, convert memory map to immutable.
    pub(super) fn into_read_only(self) -> io::Result<Memmap> {
        self.handle.unlock()?;
        self.handle.lock_shared()?;
        let map = memmap2::MmapMut::make_read_only(self.map)?;
        Ok(Memmap {
            range: self.range,
            map,
            handle: self.handle.into(),
        })
    }
}

pub(super) struct Memmap {
    range: ops::Range<u64>,
    handle: IpcFileHandle,
    map: memmap2::Mmap,
}
impl ops::Deref for Memmap {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.map
    }
}
impl Memmap {
    /// Open an existing file with read access, shared lock it and map to immutable memory.
    ///
    /// # Safety
    ///
    /// Caller must ensure the file is not modified or removed while the map is in use.
    pub(super) unsafe fn read_user_file(path: PathBuf, range: Option<ops::Range<u64>>) -> io::Result<Self> {
        let file = fs::File::open(path)?;
        file.lock_shared()?;
        let len = file.metadata()?.len();
        let range = match range {
            Some(r) => {
                if len < r.end - r.start {
                    return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "cannot map range, file too small"));
                } else {
                    r
                }
            }
            None => 0..len,
        };
        let len = match usize::try_from(range.end - range.start) {
            Ok(l) => l,
            Err(_) => return Err(io::Error::new(io::ErrorKind::FileTooLarge, "cannot map more than usize::MAX")),
        };
        let mut opt = memmap2::MmapOptions::new();
        opt.offset(range.start).len(len);
        // SAFETY: up to the caller
        let map = unsafe { opt.map(&file) }?;

        Ok(Self {
            range,
            handle: IpcFileHandle::from(file),
            map,
        })
    }

    pub(super) fn read_copy(path: PathBuf, range: Option<ops::Range<u64>>) -> io::Result<Self> {
        // Windows lock is enforced so we can skip copy if we can determinate that the `path`
        // is not in a disk that can be removed while the app is running
        #[cfg(windows)]
        if let Ok(exe) = std::env::current_exe()
            && let Ok(path) = path.canonicalize()
            && (exe.components().next() == path.components().next() || std::env::temp_dir().components().next() == exe.components().next())
        {
            // SAFETY: file will be locked and it is in the system disk or on the exe disk
            return unsafe { Self::read_user_file(path, range) };
        }

        let mut read = fs::File::open(path)?;
        Self::read_file(&mut read, range)
    }

    pub(super) fn read_file(file: &mut fs::File, range: Option<ops::Range<u64>>) -> io::Result<Self> {
        let len = file.metadata()?.len();
        let range = match range {
            Some(r) => {
                if len < r.end - r.start {
                    return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "cannot read range, file to small"));
                } else {
                    r
                }
            }
            None => 0..len,
        };
        let len = match usize::try_from(range.end - range.start) {
            Ok(l) => l,
            Err(_) => return Err(io::Error::new(io::ErrorKind::FileTooLarge, "cannot map more than usize::MAX")),
        };
        file.seek(io::SeekFrom::Start(range.start))?;

        Self::copy_stream(len, file)
    }

    pub(super) fn copy_stream(len: usize, stream: &mut impl io::Read) -> io::Result<Self> {
        let mut map = MemmapMut::new(len)?;
        stream.read_exact(&mut map)?;
        map.into_read_only()
    }

    pub(super) fn end_write(file: fs::File) -> io::Result<Self> {
        MemmapMut::end_write(file)?.into_read_only()
    }
}
impl Serialize for Memmap {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        (&self.range, &self.handle).serialize(serializer)
    }
}
impl<'de> Deserialize<'de> for Memmap {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let (range, handle) = <(ops::Range<u64>, IpcFileHandle) as Deserialize>::deserialize(deserializer)?;
        let file = fs::File::from(handle);
        let mut opt = memmap2::MmapOptions::new();
        opt.offset(range.start).len((range.end - range.start) as usize);
        // SAFETY: we trust the data was ok when it serialized
        let map = unsafe { opt.map(&file) }.map_err(serde::de::Error::custom)?;
        Ok(Self {
            range,
            handle: file.into(),
            map,
        })
    }
}
