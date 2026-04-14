use futures_lite::AsyncSeekExt as _;
use std::{
    fs,
    io::{self, Seek as _},
    mem,
    pin::{Pin, pin},
};
use zng_unit::ByteUnits as _;

use crate::channel::{IpcBytes, IpcFileHandle};

/// File handle or allocated bytes that can be read after sending to another process.
///
/// # Position
///
/// Read always starts from the beginning, the `read` methods seek the file start before returning. Note
/// that the read position is associated with the system handle, if you create multiple duplicates of the
/// same handle reading in one instance advances the position in all.
#[derive(Debug)]
#[cfg_attr(ipc, derive(serde::Serialize, serde::Deserialize))]
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
    /// Either keep the handle or read to bytes, whichever is likely to be faster for
    /// the common usage pattern of sending to another process and a single full read with some seeking.
    pub fn best_read_blocking(mut file: std::fs::File) -> io::Result<Self> {
        if file.metadata()?.len() > 5.megabytes().bytes() as u64 {
            Ok(file.into())
        } else {
            file.seek(io::SeekFrom::Start(0))?;
            IpcBytes::from_file_blocking(file).map(Into::into)
        }
    }

    /// Either keep the handle or read to bytes, whichever is likely to be faster for
    /// the common usage pattern of sending to another process and a single full read with some seeking.
    pub async fn best_read(mut file: crate::fs::File) -> io::Result<Self> {
        if file.metadata().await?.len() > 5.megabytes().bytes() as u64 {
            let file = file.try_unwrap().await.unwrap();
            Ok(file.into())
        } else {
            file.seek(io::SeekFrom::Start(0)).await?;
            IpcBytes::from_file(file).await.map(Into::into)
        }
    }

    /// Duplicate file handle or clone reference to bytes.
    pub fn duplicate(&self) -> io::Result<Self> {
        match self {
            IpcReadHandle::File(h) => h.duplicate().map(Self::File),
            IpcReadHandle::Bytes(b) => Ok(IpcReadHandle::Bytes(b.clone())),
        }
    }

    /// Begin reading using the std blocking API.
    pub fn read_blocking(self) -> io::Result<IpcReadBlocking> {
        match self {
            IpcReadHandle::File(h) => {
                let mut file = std::fs::File::from(h);
                file.seek(io::SeekFrom::Start(0))?;
                Ok(IpcReadBlocking::File(io::BufReader::new(file)))
            }
            IpcReadHandle::Bytes(b) => Ok(IpcReadBlocking::Bytes(io::Cursor::new(b))),
        }
    }

    /// Begin reading using the async API.
    pub async fn read(self) -> io::Result<IpcRead> {
        match self {
            IpcReadHandle::File(h) => {
                let mut file = crate::fs::File::from(h);
                file.seek(io::SeekFrom::Start(0)).await?;
                Ok(IpcRead::File(crate::io::BufReader::new(file)))
            }
            IpcReadHandle::Bytes(b) => Ok(IpcRead::Bytes(crate::io::Cursor::new(b))),
        }
    }

    /// Read file to new [`IpcBytes`] or unwrap bytes.
    pub fn read_to_bytes_blocking(self) -> io::Result<IpcBytes> {
        match self {
            IpcReadHandle::File(h) => {
                let mut file = std::fs::File::from(h);
                file.seek(io::SeekFrom::Start(0))?;
                IpcBytes::from_file_blocking(file)
            }
            IpcReadHandle::Bytes(b) => Ok(b),
        }
    }

    /// Read file to new [`IpcBytes`] or unwrap bytes.
    pub async fn read_to_bytes(self) -> io::Result<IpcBytes> {
        match self {
            IpcReadHandle::File(h) => {
                let mut file = crate::fs::File::from(h);
                file.seek(io::SeekFrom::Start(0)).await?;
                IpcBytes::from_file(file).await
            }
            IpcReadHandle::Bytes(b) => Ok(b),
        }
    }

    /// Attempts [`duplicate`] with read fallback.
    ///
    /// If duplicate fails attempts [`read_to_bytes`], if it succeeds replaces `self` with read bytes and returns a clone
    /// if it fails replaces `self` with empty and returns the read error.
    ///
    /// [`duplicate`]: Self::duplicate
    /// [`read_to_bytes`]: Self::read_to_bytes
    pub async fn duplicate_or_read(&mut self) -> io::Result<Self> {
        match self.duplicate() {
            Ok(d) => Ok(d),
            Err(e) => {
                tracing::debug!("duplicate_or_read duplicate error, {e}");
                let f = mem::replace(self, IpcReadHandle::Bytes(IpcBytes::empty()));
                let b = f.read_to_bytes().await?;
                *self = IpcReadHandle::Bytes(b);
                self.duplicate()
            }
        }
    }

    /// Attempts [`duplicate`] with read fallback.
    ///
    /// If duplicate fails attempts [`read_to_bytes_blocking`], if it succeeds replaces `self` with read bytes and returns a clone
    /// if it fails replaces `self` with empty and returns the read error.
    ///
    /// [`duplicate`]: Self::duplicate
    /// [`read_to_bytes_blocking`]: Self::read_to_bytes_blocking
    pub fn duplicate_or_read_blocking(&mut self) -> io::Result<Self> {
        match self.duplicate() {
            Ok(d) => Ok(d),
            Err(e) => {
                tracing::debug!("duplicate_or_read_blocking duplicate error, {e}");
                let f = mem::replace(self, IpcReadHandle::Bytes(IpcBytes::empty()));
                let b = f.read_to_bytes_blocking()?;
                *self = IpcReadHandle::Bytes(b);
                self.duplicate()
            }
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
impl IpcReadBlocking {
    /// Read all bytes until EOF to a new [`IpcBytes`].
    ///
    /// If the position is at 0 and is already `Bytes` returns it.
    pub fn read_to_bytes(&mut self) -> io::Result<IpcBytes> {
        match self {
            IpcReadBlocking::File(f) => IpcBytes::from_read_blocking(f),
            IpcReadBlocking::Bytes(c) => {
                let start = c.position();
                let len = c.get_ref().len();
                c.set_position(len as u64);
                if start == 0 {
                    Ok(c.get_ref().clone())
                } else {
                    IpcBytes::from_slice_blocking(&c.get_ref()[start as usize..])
                }
            }
        }
    }

    /// Remaining bytes length.
    pub fn remaining_len(&mut self) -> io::Result<u64> {
        match self {
            IpcReadBlocking::File(b) => {
                let total_len = b.get_ref().metadata()?.len();
                let position = b.stream_position()?;
                Ok(total_len - position.min(total_len))
            }
            IpcReadBlocking::Bytes(b) => {
                let total_len = b.get_ref().len() as u64;
                Ok(total_len - b.position().min(total_len))
            }
        }
    }
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
impl IpcRead {
    /// Read all bytes until EOF to a new [`IpcBytes`].
    ///
    /// If the position is at 0 and is already `Bytes` returns it.
    pub async fn read_to_bytes(&mut self) -> io::Result<IpcBytes> {
        match self {
            IpcRead::File(f) => IpcBytes::from_read(pin!(f)).await,
            IpcRead::Bytes(c) => {
                let start = c.position();
                let len = c.get_ref().len();
                c.set_position(len as u64);
                let b = c.get_ref().clone();
                if start == 0 {
                    Ok(b)
                } else {
                    blocking::unblock(move || IpcBytes::from_slice_blocking(&b[start as usize..])).await
                }
            }
        }
    }

    /// Remaining bytes length.
    pub async fn remaining_len(&mut self) -> io::Result<u64> {
        match self {
            IpcRead::File(b) => {
                let total_len = b.get_ref().metadata().await?.len();
                let pos = b.seek(io::SeekFrom::Current(0)).await?;
                Ok(total_len - pos.min(total_len))
            }
            IpcRead::Bytes(b) => {
                let total_len = b.get_ref().len() as u64;
                Ok(total_len - b.position().min(total_len))
            }
        }
    }
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
