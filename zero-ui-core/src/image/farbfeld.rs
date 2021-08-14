//! Farbfeld async streaming decoder/encoder.
//!
//! See <https://tools.suckless.org/farbfeld/> for details about format.

// This format is the most simple and so is also the test-bed of encoder/decoder API designs.

use super::*;
use crate::task;
use crate::units::*;

/// Farbfeld async streaming reader.
pub struct Decoder<R> {
    width: u32,
    height: u32,
    task: R,
    pending_lines: u32,
    line_len: usize,
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
    pub fn dpi(&self) -> Ppi {
        Ppi::default()
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
impl Decoder<()> {
    /// Reads the header only, returns the (reader, width, height).
    pub async fn read_header<R>(read: &mut R) -> Result<(u32, u32), R::Error>
    where
        R: task::ReadThenReceive,
    {
        let mut header = ArrayRead::<{ 8 + 4 + 4 }>::load(read).await?;

        header.match_ascii(b"farbfeld")?;

        let width = header.read_u32_be();
        let height = header.read_u32_be();

        Ok((width, height))
    }
}
impl<R> Decoder<R>
where
    R: task::ReceiverTask,
{
    /// Reads the header and starts the decoder task.
    pub async fn start<RR>(mut read: RR, max_bytes: ByteLength) -> Result<Decoder<R>, RR::Error>
    where
        RR: task::ReadThenReceive<Spawned = R>,
    {
        let (width, height) = Decoder::read_header(&mut read).await?;

        check_limit(width, height, (4 * 2).bytes(), max_bytes)?;

        let line_len = width as usize * 4 * 2;
        let task = read.spawn(line_len.bytes(), height as usize);
        Ok(Decoder {
            width,
            height,
            task,
            line_len,
            pending_lines: height,
        })
    }

    /// Read and decode a count of pixel lines.
    pub async fn read_lines(&mut self, line_count: u32) -> Result<Bgra8Buf, R::Error> {
        let count = line_count.max(self.pending_lines) as usize;
        if count == 0 {
            return Ok(Bgra8Buf::empty());
        }

        let mut r = Bgra8Buf::with_capacity(count * self.width as usize * 4);

        for _ in 0..count {
            let line = Bgra8Buf::from_rgba16_be8(self.task.recv().await?);
            if line.len() < self.line_len {
                return Err(unexpected_eof("did not read a full line").into());
            }
            r.extend(line);
        }

        self.pending_lines -= count as u32;

        Ok(r)
    }

    /// Read all pending lines.
    pub async fn read_all(&mut self) -> Result<Bgra8Buf, R::Error> {
        self.read_lines(self.pending_lines).await
    }
}

/// An async streaming farbfeld encoder.
pub struct Encoder<S> {
    task: S,
    pending_bytes: usize,
}
impl<S> Encoder<S>
where
    S: task::SenderTask,
{
    /// Write the farbfeld header and starts the writer task.
    pub async fn start(width: u32, height: u32, task: S) -> Result<Self, S::Error> {
        let mut header = vec![0; 8 + 4 + 4];
        header.extend(b"farbfeld");
        header.extend(&width.to_be_bytes());
        header.extend(&height.to_be_bytes());

        match task.send(header).await {
            Ok(_) => Ok(Encoder {
                task,
                pending_bytes: width as usize * height as usize * 4 * 2,
            }),
            Err(_) => match task.finish().await {
                Err(e) => Err(e),
                Ok(_) => unreachable!(),
            },
        }
    }

    /// Encode and write more image pixels.
    pub async fn write(&mut self, partial_bgra8_payload: Bgra8Buf) -> Result<(), task::SenderTaskClosed> {
        let mut bytes: Vec<u8> = partial_bgra8_payload.into_rgba16_be8();
        if self.pending_bytes < bytes.len() {
            let rest = bytes.drain(self.pending_bytes..).collect();
            if !bytes.is_empty() {
                self.task.send(bytes).await?;
            }
            Err(task::SenderTaskClosed { payload: rest })
        } else {
            self.task.send(bytes).await
        }
    }

    /// Flushes and closes the writer task, returns the write back and any error that happened during write.
    pub async fn finish(self) -> Result<S::Writer, S::Error> {
        self.task.finish().await
    }
}
