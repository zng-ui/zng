//! Farbfeld async streaming decoder/encoder.
//!
//! See [https://tools.suckless.org/farbfeld/] for details about format.

// thid format is the most simple and so is also the test-bed of encoder/decoder API designs.

use std::io;

use super::*;
use crate::task::{
    self,
    io::{ReadTask, ReadTaskError, WriteTask, WriteTaskClosed, WriteTaskError},
};

/// Farbfeld async streaming reader.
pub struct Decoder<R> {
    width: u32,
    height: u32,
    task: ReadTask<R>,
    pending_lines: u32,
}
impl<R> Decoder<R> {
    /// Pixel width.
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Pixel height.
    pub fn height(&self) -> u32 {
        self.height
    }

    /// `96.0`
    pub fn dpi(&self) -> f32 {
        96.0
    }

    /// `sRGB`
    pub fn icc(&self) -> lcms2::Profile {
        lcms2::Profile::new_srgb()
    }

    /// Number of pixel lines not yet read.
    pub fn pending_lines(&self) -> u32 {
        self.pending_lines
    }
}
impl<R> Decoder<R>
where
    R: io::Read + Send + 'static,
{
    /// Reads the header only, returns the (reader, width, height).
    pub async fn read_header(mut read: R) -> io::Result<(R, u32, u32)> {
        let (read, mut header) = task::wait(move || {
            let r = ArrayRead::<{ 8 + 4 + 4 }>::load(&mut read)?;
            Ok::<_, io::Error>((read, r))
        })
        .await?;

        header.match_ascii(b"farbfeld")?;

        let width = header.read_u32_be();
        let height = header.read_u32_be();

        Ok((read, width, height))
    }

    /// Reads the header and starts the decoder task.
    pub async fn start(read: R, max_bytes: usize) -> io::Result<Decoder<R>> {
        let (read, width, height) = Self::read_header(read).await?;

        check_limit(width, height, 4 * 2, max_bytes)?;

        let task = ReadTask::default()
            .payload_len(width as usize * 4 * 2)
            .channel_capacity(height as usize)
            .spawn(read);

        Ok(Decoder {
            width,
            height,
            task,
            pending_lines: height,
        })
    }

    /// Read and decode a count of pixel lines.
    pub async fn read_lines(&mut self, line_count: u32) -> Result<Bgra8Buf, ReadTaskError> {
        let count = line_count.max(self.pending_lines) as usize;
        if count == 0 {
            return Ok(Bgra8Buf::empty());
        }

        let mut r = Bgra8Buf::with_capacity(count * self.width as usize * 4);

        for _ in 0..count {
            let line = Bgra8Buf::from_rgba16_be8(self.task.read().await?);
            if line.len() < self.task.payload_len() {
                return Err(unexpected_eof("did not read a full line").into());
            }
            r.extend(line);
        }

        self.pending_lines -= count as u32;

        Ok(r)
    }

    /// Read all pending lines.
    pub async fn read_all(&mut self) -> Result<Bgra8Buf, ReadTaskError> {
        self.read_lines(self.pending_lines).await
    }
}

/// An async streaming farbfeld encoder.
pub struct Encoder<W> {
    task: WriteTask<W>,
    pending_bytes: usize,
}
impl<W> Encoder<W>
where
    W: io::Write + Send + 'static,
{
    /// Write the farbfeld header and starts the writer task.
    pub async fn start(width: u32, height: u32, write_magic: bool, mut write: W) -> io::Result<Self> {
        let write = task::wait(move || {
            if write_magic {
                write.write_all(b"farbfeld")?;
                write.write_all(&width.to_be_bytes())?;
                write.write_all(&height.to_be_bytes())?;
            }
            Ok::<_, io::Error>(write)
        })
        .await?;

        let task = WriteTask::default().channel_capacity(height as usize).spawn(write);

        Ok(Encoder {
            task,
            pending_bytes: width as usize * height as usize * 4 * 2,
        })
    }

    /// Write the image pixels.
    pub async fn write(&mut self, payload: impl Into<Bgra8Buf>) -> Result<(), WriteTaskClosed> {
        let mut bytes: Vec<u8> = payload.into().into_rgba16_be8();
        if self.pending_bytes < bytes.len() {
            let rest = bytes.drain(self.pending_bytes..).collect();
            if !bytes.is_empty() {
                self.task.write(bytes).await?;
            }
            Err(WriteTaskClosed { payload: rest })
        } else {
            self.task.write(bytes).await
        }
    }

    /// Flushes and closes the writer, returns the write back and any error that happened during write.
    pub async fn finish(self) -> Result<W, WriteTaskError<W>> {
        self.task.finish().await
    }
}
