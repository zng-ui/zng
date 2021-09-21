//! Farbfeld async streaming decoder/encoder.
//!
//! See <https://tools.suckless.org/farbfeld/> for details about format.

// This format is the most simple and so is also the test-bed of encoder/decoder API designs.

use super::*;
use crate::task::io::*;
use crate::units::*;

/// Farbfeld async streaming reader.
pub struct Decoder<R> {
    width: u32,
    height: u32,
    read: BufReader<R>,
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
    pub async fn read_header<R>(read: &mut R) -> Result<(u32, u32)>
    where
        R: AsyncRead + Unpin,
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
    R: AsyncRead + Unpin,
{
    /// Reads the header and starts the decoder task.
    pub async fn start(mut read: R, max_bytes: ByteLength) -> Result<Decoder<R>> {
        let (width, height) = Decoder::read_header(&mut read).await?;

        check_limit(width, height, (4 * 2).bytes(), max_bytes)?;

        let line_len = width as usize * 4 * 2;
        let read = BufReader::with_capacity(line_len, read);
        Ok(Decoder {
            width,
            height,
            read,
            line_len,
            pending_lines: height,
        })
    }

    /// Read and decode a count of pixel lines.
    pub async fn read_lines(&mut self, line_count: u32) -> Result<Bgra8Buf> {
        let count = line_count.max(self.pending_lines) as usize;
        if count == 0 {
            return Ok(Bgra8Buf::empty());
        }

        let line = self.width as usize * 4;
        let mut r = Bgra8Buf::with_capacity(count * line);

        for _ in 0..count {
            let mut buf = vec![0; line];
            self.read.read_exact(&mut buf).await?;
            let line = Bgra8Buf::from_rgba16_be8(buf);
            if line.len() < self.line_len {
                return Err(unexpected_eof("did not read a full line"));
            }
            r.extend(line);
        }

        self.pending_lines -= count as u32;

        Ok(r)
    }

    /// Read all pending lines.
    pub async fn read_all(&mut self) -> Result<Bgra8Buf> {
        self.read_lines(self.pending_lines).await
    }
}

/// An async streaming farbfeld encoder.
pub struct Encoder<W> {
    write: W,
    pending_bytes: usize,
}
impl<W> Encoder<W>
where
    W: AsyncWrite + Unpin,
{
    /// Write the farbfeld header and starts the writer task.
    pub async fn start(width: u32, height: u32, mut write: W) -> Result<Self> {
        let mut header = vec![0; 8 + 4 + 4];
        header.extend(b"farbfeld");
        header.extend(&width.to_be_bytes());
        header.extend(&height.to_be_bytes());

        write.write_all(&header).await?;

        Ok(Encoder {
            write,
            pending_bytes: width as usize * height as usize * 4 * 2,
        })
    }

    /// Encode and write more image pixels.
    pub async fn write(&mut self, partial_bgra8_payload: Bgra8Buf) -> Result<()> {
        let bytes = partial_bgra8_payload.into_rgba16_be8();

        if self.pending_bytes < bytes.len() {
            // overflow, write the rest and return an error.
            self.write.write_all(&bytes[..self.pending_bytes]).await?;
            Err(invalid_input("payload encoded to more bytes then expected for image"))
        } else {
            self.write.write_all(&bytes).await
        }
    }

    /// Flushes and returns the write task.
    pub async fn finish(mut self) -> Result<W> {
        self.write.flush().await?;
        Ok(self.write)
    }
}
