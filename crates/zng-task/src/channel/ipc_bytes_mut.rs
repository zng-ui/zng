#![cfg_attr(not(ipc), allow(unused))]

use std::{
    fmt,
    io::{self, Write as _},
    mem::MaybeUninit,
    ops,
    path::PathBuf,
    pin::Pin,
    sync::Arc,
};

#[cfg(ipc)]
use ipc_channel::ipc::IpcSharedMemory;

use crate::channel::IpcBytes;
use crate::channel::ipc_bytes::IpcBytesData;
#[cfg(ipc)]
use crate::channel::ipc_bytes_memmap::MemmapMut;

enum IpcBytesMutInner {
    Heap(Vec<u8>),
    #[cfg(ipc)]
    AnonMemMap(IpcSharedMemory),
    #[cfg(ipc)]
    MemMap(MemmapMut),
}

/// Represents preallocated exclusive mutable memory that can be converted to [`IpcBytes`].
///
/// Like [`IpcBytes`] three storage modes are supported, heap, shared memory and file backed memory map. Most
/// efficient mode is selected automatically for the given length, unless the constructor function explicitly states otherwise.
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
            IpcBytesMutInner::MemMap(m) => &m[..len],
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
            IpcBytesMutInner::MemMap(m) => &mut m[..len],
        }
    }
}
impl fmt::Debug for IpcBytesMut {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "IpcBytesMut(<{} bytes>)", self.len())
    }
}
impl IpcBytesMut {
    /// Allocate zeroed mutable memory.
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

    /// Allocate zeroed mutable memory.
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
            Ok(IpcBytesMut {
                len,
                inner: IpcBytesMutInner::MemMap(MemmapMut::new(len)?),
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

    /// Allocate zeroed mutable memory in a memory map.
    ///
    /// Note that [`new`] automatically selects the best memory storage for the given `len`, this
    /// function enforces the usage of a memory map, the slowest of the options.
    ///
    /// [`new`]: Self::new
    #[cfg(ipc)]
    pub async fn new_memmap(len: usize) -> io::Result<Self> {
        blocking::unblock(move || Self::new_memmap_blocking(len)).await
    }

    /// Allocate zeroed mutable memory in a memory map.
    ///
    /// Note that [`new_blocking`] automatically selects the best memory storage for the given `len`, this
    /// function enforces the usage of a memory map, the slowest of the options.
    ///
    /// [`new_blocking`]: Self::new_blocking
    #[cfg(ipc)]
    pub fn new_memmap_blocking(len: usize) -> io::Result<Self> {
        Ok(Self {
            len,
            inner: IpcBytesMutInner::MemMap(MemmapMut::new(len)?),
        })
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
                let mut b = Self::new_blocking(buf.len())?;
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

    /// Uses `buf` or copies it to exclusive mutable memory.
    pub fn from_vec_blocking(buf: Vec<u8>) -> io::Result<Self> {
        #[cfg(ipc)]
        if buf.len() <= IpcBytes::INLINE_MAX {
            Ok(Self {
                len: buf.len(),
                inner: IpcBytesMutInner::Heap(buf),
            })
        } else {
            let mut b = Self::new_blocking(buf.len())?;
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
            let mut b = Self::new_blocking(buf.len())?;
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

    /// Use or copy bytes to exclusive mutable memory.
    pub async fn from_bytes(bytes: IpcBytes) -> io::Result<Self> {
        blocking::unblock(move || Self::from_bytes_blocking(bytes)).await
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

    /// Memory map an existing file.
    ///
    /// The `range` defines the slice of the `file` that will be mapped. Returns [`io::ErrorKind::UnexpectedEof`]
    // if the file does not have enough bytes. Returns [`io::ErrorKind::FileTooLarge`] if the range length or file length is
    // greater than `usize::MAX`.
    ///
    /// # Safety
    ///
    /// Caller must ensure the `file` is not modified or removed while the `IpcBytesMut` instance lives, or any instance of
    /// [`IpcBytes`] later created from this.
    #[cfg(ipc)]
    pub async unsafe fn open_memmap(file: PathBuf, range: Option<ops::Range<u64>>) -> io::Result<Self> {
        blocking::unblock(move || {
            // SAFETY: up to the caller
            unsafe { Self::open_memmap_blocking(file, range) }
        })
        .await
    }

    /// Memory map an existing file.
    ///
    /// The `range` defines the slice of the `file` that will be mapped. Returns [`io::ErrorKind::UnexpectedEof`]
    // if the file does not have enough bytes. Returns [`io::ErrorKind::FileTooLarge`] if the range length or file length is
    // greater than `usize::MAX`.
    ///
    /// # Safety
    ///
    /// Caller must ensure the `file` is not modified or removed while the `IpcBytesMut` instance lives, or any instance of
    /// [`IpcBytes`] later created from this.
    #[cfg(ipc)]
    pub unsafe fn open_memmap_blocking(file: PathBuf, range: Option<ops::Range<u64>>) -> io::Result<Self> {
        // SAFETY: up to the caller
        let map = unsafe { MemmapMut::write_user_file(file, range) }?;

        Ok(Self {
            len: map.len(),
            inner: IpcBytesMutInner::MemMap(map),
        })
    }

    /// Create a new zeroed file and memory map it.
    ///
    /// # Safety
    ///
    /// Caller must ensure the `file` is not modified or removed while the `IpcBytesMut` instance lives, or any instance of
    /// [`IpcBytes`] later created from this.
    #[cfg(ipc)]
    pub async unsafe fn create(file: PathBuf, len: usize) -> io::Result<Self> {
        blocking::unblock(move || {
            // SAFETY: up to the caller
            unsafe { Self::create_blocking(file, len) }
        })
        .await
    }

    /// Create a new zeroed file and memory map it.
    ///
    /// # Safety
    ///
    /// Caller must ensure the `file` is not modified or removed while the `IpcBytesMut` instance lives, or any instance of
    /// [`IpcBytes`] later created from this.
    #[cfg(ipc)]
    pub unsafe fn create_blocking(file: PathBuf, len: usize) -> io::Result<Self> {
        // SAFETY: up to the caller
        let map = unsafe { MemmapMut::create_user_file(file, len) }?;

        Ok(Self {
            len,
            inner: IpcBytesMutInner::MemMap(map),
        })
    }
}
impl IpcBytesMut {
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
            IpcBytesMutInner::MemMap(m) => {
                let m = m.into_read_only()?;
                IpcBytesData::MemMap(m)
            }
        };
        Ok(IpcBytes(Arc::new(data)))
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
            IpcBytesMutInner::MemMap(m) => {
                let m = m.into_read_only()?;
                IpcBytesData::MemMap(m)
            }
        };
        Ok(IpcBytes(Arc::new(data)))
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
    memmap: Option<std::fs::File>,

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
                Some(file) => {
                    let map = MemmapMut::end_write(file)?;
                    let len = map.len();
                    (len, IpcBytesMutInner::MemMap(map))
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
            self.memmap = Some(MemmapMut::begin_write()?);
        }
        let file = &mut self.memmap.as_mut().unwrap();

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
                    self.memmap.as_mut().unwrap().write_all(write_buf)?;
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
        if let Some(file) = &mut self.memmap {
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
            let file = self.memmap.as_mut().unwrap();
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
}
