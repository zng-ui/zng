use std::{fs, io, pin::Pin};

use crate::channel::{IpcBytes, IpcFileHandle};

/// File handle or allocated bytes that can be read after sending to another process.
#[derive(Debug)]
#[non_exhaustive]
pub enum IpcReadHandle {
    /// Read directly from disk.
    File(IpcFileHandle),
    /// Read from allocated bytes.
    Bytes(IpcBytes),
}
impl From<IpcFileHandle> for IpcReadHandle {
    fn from(f: IpcFileHandle) -> Self {
        IpcReadHandle::File(f)
    }
}
impl From<IpcBytes> for IpcReadHandle {
    fn from(b: IpcBytes) -> Self {
        IpcReadHandle::Bytes(b)
    }
}
impl From<fs::File> for IpcReadHandle {
    fn from(f: fs::File) -> Self {
        IpcReadHandle::File(f.into())
    }
}
impl IpcReadHandle {
    /// Duplicate file handle or clone reference to bytes.
    pub fn duplicate(&self) -> io::Result<Self> {
        match self {
            IpcReadHandle::File(h) => h.duplicate().map(Self::File),
            IpcReadHandle::Bytes(b) => Ok(IpcReadHandle::Bytes(b.clone())),
        }
    }

    /// Begin reading using the std blocking API.
    pub fn read_blocking(self) -> IpcReadBlocking {
        match self {
            IpcReadHandle::File(h) => IpcReadBlocking::File(io::BufReader::new(h.into())),
            IpcReadHandle::Bytes(b) => IpcReadBlocking::Bytes(io::Cursor::new(b)),
        }
    }

    /// Begin reading using the async API.
    pub fn read(self) -> IpcRead {
        match self {
            IpcReadHandle::File(h) => IpcRead::File(crate::io::BufReader::new(h.into())),
            IpcReadHandle::Bytes(b) => IpcRead::Bytes(crate::io::Cursor::new(b)),
        }
    }

    /// Read file to new [`IpcBytes`] or unwrap bytes.
    pub fn read_to_bytes_blocking(self) -> io::Result<IpcBytes> {
        match self {
            IpcReadHandle::File(h) => IpcBytes::from_file_blocking(h.into()),
            IpcReadHandle::Bytes(b) => Ok(b),
        }
    }

    /// Read file to new [`IpcBytes`] or unwrap bytes.
    pub async fn read_to_bytes(self) -> io::Result<IpcBytes> {
        match self {
            IpcReadHandle::File(h) => IpcBytes::from_file(h.into()).await,
            IpcReadHandle::Bytes(b) => Ok(b),
        }
    }
}

/// Blocking read implementer for [`IpcReadHandle::read_blocking`].
#[derive(Debug)]
#[non_exhaustive]
pub enum IpcReadBlocking {
    /// Buffered reader from file.
    File(io::BufReader<fs::File>),
    /// Bytes.
    Bytes(io::Cursor<IpcBytes>),
}
impl io::Read for IpcReadBlocking {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            IpcReadBlocking::File(f) => f.read(buf),
            IpcReadBlocking::Bytes(b) => b.read(buf),
        }
    }

    fn read_vectored(&mut self, bufs: &mut [io::IoSliceMut<'_>]) -> io::Result<usize> {
        match self {
            IpcReadBlocking::File(f) => f.read_vectored(bufs),
            IpcReadBlocking::Bytes(b) => b.read_vectored(bufs),
        }
    }

    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> io::Result<usize> {
        match self {
            IpcReadBlocking::File(f) => f.read_to_end(buf),
            IpcReadBlocking::Bytes(b) => b.read_to_end(buf),
        }
    }

    fn read_to_string(&mut self, buf: &mut String) -> io::Result<usize> {
        match self {
            IpcReadBlocking::File(f) => f.read_to_string(buf),
            IpcReadBlocking::Bytes(b) => b.read_to_string(buf),
        }
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> io::Result<()> {
        match self {
            IpcReadBlocking::File(f) => f.read_exact(buf),
            IpcReadBlocking::Bytes(b) => b.read_exact(buf),
        }
    }
}
impl io::Seek for IpcReadBlocking {
    fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
        match self {
            IpcReadBlocking::File(f) => f.seek(pos),
            IpcReadBlocking::Bytes(b) => b.seek(pos),
        }
    }

    fn stream_position(&mut self) -> io::Result<u64> {
        match self {
            IpcReadBlocking::File(f) => f.stream_position(),
            IpcReadBlocking::Bytes(b) => b.stream_position(),
        }
    }
}
impl io::BufRead for IpcReadBlocking {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        match self {
            IpcReadBlocking::File(f) => f.fill_buf(),
            IpcReadBlocking::Bytes(b) => b.fill_buf(),
        }
    }

    fn consume(&mut self, amount: usize) {
        match self {
            IpcReadBlocking::File(f) => f.consume(amount),
            IpcReadBlocking::Bytes(b) => b.consume(amount),
        }
    }
}

/// Async read implementer for [`IpcReadHandle::read`]
#[derive(Debug)]
#[non_exhaustive]
pub enum IpcRead {
    /// Buffered reader from file.
    File(crate::io::BufReader<crate::fs::File>),
    /// Bytes.
    Bytes(crate::io::Cursor<IpcBytes>),
}
impl crate::io::AsyncRead for IpcRead {
    fn poll_read(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>, buf: &mut [u8]) -> std::task::Poll<io::Result<usize>> {
        match self.get_mut() {
            IpcRead::File(f) => Pin::new(f).poll_read(cx, buf),
            IpcRead::Bytes(b) => Pin::new(b).poll_read(cx, buf),
        }
    }

    fn poll_read_vectored(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        bufs: &mut [io::IoSliceMut<'_>],
    ) -> std::task::Poll<io::Result<usize>> {
        match self.get_mut() {
            IpcRead::File(f) => Pin::new(f).poll_read_vectored(cx, bufs),
            IpcRead::Bytes(b) => Pin::new(b).poll_read_vectored(cx, bufs),
        }
    }
}
impl crate::io::AsyncBufRead for IpcRead {
    fn poll_fill_buf(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<io::Result<&[u8]>> {
        match self.get_mut() {
            IpcRead::File(f) => Pin::new(f).poll_fill_buf(cx),
            IpcRead::Bytes(b) => Pin::new(b).poll_fill_buf(cx),
        }
    }

    fn consume(self: Pin<&mut Self>, amt: usize) {
        match self.get_mut() {
            IpcRead::File(f) => Pin::new(f).consume(amt),
            IpcRead::Bytes(b) => Pin::new(b).consume(amt),
        }
    }
}
impl crate::io::AsyncSeek for IpcRead {
    fn poll_seek(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>, pos: io::SeekFrom) -> std::task::Poll<io::Result<u64>> {
        match self.get_mut() {
            IpcRead::File(f) => Pin::new(f).poll_seek(cx, pos),
            IpcRead::Bytes(b) => Pin::new(b).poll_seek(cx, pos),
        }
    }
}
