#![cfg_attr(not(ipc), allow(unused))]

use std::{
    fmt,
    io::{self, Write as _},
    mem::MaybeUninit,
    ops,
    pin::Pin,
    sync::Arc,
};
#[cfg(ipc)]
use std::{fs, path::PathBuf};

#[cfg(ipc)]
use ipc_channel::ipc::IpcSharedMemory;

use crate::channel::IpcBytes;
use crate::channel::ipc_bytes::IpcBytesData;

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
    /// Allocate zeroed mutable memory that can be written to and then converted to `IpcBytes` fast.
    pub async fn new(len: usize) -> io::Result<IpcBytesMut> {
        #[cfg(ipc)]
        if len <= IpcBytes::INLINE_MAX {
            Ok(IpcBytesMut {
                len,
                inner: IpcBytesMutInner::Heap(vec![0; len]),
            })
        } else if len <= IpcBytes::UNNAMED_MAX {
            Ok(IpcBytesMut {
                len,
                inner: IpcBytesMutInner::AnonMemMap(IpcSharedMemory::from_byte(0, len)),
            })
        } else {
            blocking::unblock(move || Self::new_blocking(len)).await
        }

        #[cfg(not(ipc))]
        {
            Ok(IpcBytesMut {
                len,
                inner: IpcBytesMutInner::Heap(vec![0; len]),
            })
        }
    }

    /// Allocate zeroed mutable memory that can be written to and then converted to `IpcBytes` fast.
    pub fn new_blocking(len: usize) -> io::Result<IpcBytesMut> {
        #[cfg(ipc)]
        if len <= IpcBytes::INLINE_MAX {
            Ok(IpcBytesMut {
                len,
                inner: IpcBytesMutInner::Heap(vec![0; len]),
            })
        } else if len <= IpcBytes::UNNAMED_MAX {
            Ok(IpcBytesMut {
                len,
                inner: IpcBytesMutInner::AnonMemMap(IpcSharedMemory::from_byte(0, len)),
            })
        } else {
            let (name, file) = IpcBytes::create_memmap()?;
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

    /// Use or copy bytes to exclusive mutable memory.
    pub async fn from_bytes(bytes: IpcBytes) -> io::Result<Self> {
        blocking::unblock(move || Self::from_bytes_blocking(bytes)).await
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
        Ok(IpcBytesData::MemMap(super::ipc_bytes::IpcMemMap {
            name,
            range: 0..len,
            is_custom: false,
            map: super::ipc_bytes::IpcMemMapData::Connected(map, read_handle),
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

    /// Copy `buf` to exclusive mutable memory.
    pub fn from_slice_blocking(buf: &[u8]) -> io::Result<Self> {
        #[cfg(ipc)]
        if buf.len() <= IpcBytes::INLINE_MAX {
            Ok(Self {
                len: buf.len(),
                inner: IpcBytesMutInner::Heap(buf.to_vec()),
            })
        } else {
            let mut b = IpcBytes::new_mut_blocking(buf.len())?;
            b[..].copy_from_slice(buf);
            Ok(b)
        }
        #[cfg(not(ipc))]
        {
            Ok(Self {
                len: buf.len(),
                inner: IpcBytesMutInner::Heap(buf.to_vec()),
            })
        }
    }

    /// Use or copy `bytes` to exclusive mutable memory.
    pub fn from_bytes_blocking(bytes: IpcBytes) -> io::Result<Self> {
        #[cfg_attr(not(ipc), allow(irrefutable_let_patterns))]
        if let IpcBytesData::Heap(_) = &*bytes.0 {
            match Arc::try_unwrap(bytes.0) {
                Ok(r) => match r {
                    IpcBytesData::Heap(r) => Ok(Self {
                        len: r.len(),
                        inner: IpcBytesMutInner::Heap(r),
                    }),
                    _ => unreachable!(),
                },
                Err(a) => Self::from_slice_blocking(&IpcBytes(a)[..]),
            }
        } else {
            Self::from_slice_blocking(&bytes[..])
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
    pub async fn finish_mut(self) -> std::io::Result<super::IpcBytesMut> {
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
    pub fn finish_mut(mut self) -> std::io::Result<super::IpcBytesMut> {
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

impl IpcBytes {
    /// Start a memory efficient async writer for creating a `IpcBytes` with unknown length.
    pub async fn new_writer() -> IpcBytesWriter {
        IpcBytesWriter {
            inner: blocking::Unblock::new(Self::new_writer_blocking()),
        }
    }

    /// Allocate zeroed mutable memory that can be written to and then converted to `IpcBytes` fast.
    pub async fn new_mut(len: usize) -> io::Result<super::IpcBytesMut> {
        super::IpcBytesMut::new(len).await
    }

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
    pub fn new_mut_blocking(len: usize) -> io::Result<super::IpcBytesMut> {
        super::IpcBytesMut::new_blocking(len)
    }
}
