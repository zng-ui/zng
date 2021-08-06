//! BMP async streaming decoder and encoder.
//!
//! See [https://en.wikipedia.org/wiki/BMP_file_format] for details about the format.

use std::io;

use super::formats::{Bgra8Buf, u16_le, u32_le};
use crate::task::{
    self,
    io::{ReadTask, ReadTaskError},
};

/// BMP header info.
pub struct BmpHeader {
    /// Pixel width.
    pub width: u32,
    /// Pixel height.
    pub height: u32,
    /// Pixel-per-meter horizontal resolution.
    pub ppm_x: u32,
    ///  Pixel-per-meter vertical resolution.
    pub ppm_y: u32,
}
impl From<BmpHeaderFull> for BmpHeader {
    fn from(bmp: BmpHeaderFull) -> Self {
        BmpHeader {
            width: bmp.width,
            height: bmp.height,
            ppm_x: bmp.ppm_x,
            ppm_y: bmp.ppm_y,
        }
    }
}

const DEFAULT_PPM: u32 = 3780; // 96dpi

enum Halftoning {
    None,
    ErrorDiffusion(u8),
    Panda(u32, u32), // Processing Algorithm for Noncoded Document Acquisition
    SuperCircle(u32, u32)
}

struct BmpHeaderFull {
    header_size: u32,

    width: u32,
    height: u32,
    bpp: u8,
    ppm_x: u32,
    ppm_y: u32,
    top_down: bool,
    halftoning: Halftoning,
    
}
impl Default for BmpHeaderFull {
    fn default() -> Self {
        BmpHeaderFull {
            header_size: 0,
            width: 0,
            height: 0,
            bpp: 0,
            ppm_x: DEFAULT_PPM,
            ppm_y: DEFAULT_PPM,
            top_down: false,
            halftoning: Halftoning::None,
        }
    }
}
impl BmpHeaderFull {
    // reference: https://searchfox.org/mozilla-central/source/image/decoders/nsBMPDecoder.cpp#197
    //            https://en.wikipedia.org/wiki/BMP_file_format

    pub fn read(&mut self, read: &mut impl io::Read) -> io::Result<()> {
        let mut head = [0u8; 14];
        read.read_exact(&mut head)?;

        if &head[..2] != b"BM" {
            return Err(io::Error::new(io::ErrorKind::InvalidData, r#"expected `b"BM"` magic"#));
        }

        let mut size = [0u8; 4];
        read.read_exact(&mut size)?;
        self.header_size = u32::from_le_bytes(size);

        match self.header_size {
            12 => self.read_core_header(read),
            64 => {
                self.read_core_header(read);
                self.read_core_header2(read)
            },
            40 => self.read_info_header(read),

            unknown => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("unknown header size `{}`", unknown),
            )),
        }
    }

    fn read_core_header(&mut self, read: &mut impl io::Read) -> io::Result<()> {
        let mut h = [0u8; 12 - 4];
        read.read_exact(&mut h)?;

        let mut cur = 0;
        self.width = u16::from_le_bytes([h[cur], h[cur + 1]]) as u32;
        cur += 2;
        self.height = u16::from_le_bytes([h[cur], h[cur + 1]]) as u32;
        cur += 2;

        cur += 2; // ignore color plane count

        self.bpp = u16::from_le_bytes([h[cur], h[cur + 1]]) as u8;

        Ok(())
    }

    fn read_core_header2(&mut self, read: &mut impl io::Read) -> io::Result<()> {
        let mut h = [0u8; 64 - 12 - 4];
        read.read_exact(&mut h)?;

        let mut cur = 0;
        cur += 2; // ignore always pixels per metre        
        cur += 2; // ignore padding
        cur += 2; // ignore always bottom-to-top  
        let halftoning_u16 = u16_le(&h, cur);
        cur += 2;

        self.halftoning = match halftoning_u16 {
            0 => Halftoning::None,
            // 1 => Halftoning::ErrorDiffusion(u32_le(&h, cur)),
            // 2 => Halftoning::Panda(),
            // 3 => Halftoning::SuperCircle(),
            _ => return Err(io::Error::new(io::ErrorKind::InvalidData, "unknown halftoning algorithm"))
        };
        
        //

        Ok(())
    }

    fn read_info_header(&mut self, read: &mut impl io::Read) -> io::Result<()> {
        Ok(())
    }
}

/// BMP async streaming reader.
pub struct Decoder<R> {
    header: BmpHeader,

    task: ReadTask<R>,
}
impl<R> Decoder<R> {
    /// Header info.
    pub fn header(&self) -> &BmpHeader {
        &self.header
    }
}
impl<R: io::Read + Send + 'static> Decoder<R> {
    /// Reads the header only.
    pub async fn read_header(read: R) -> io::Result<(R, BmpHeader)> {
        let (r, header_full) = Self::read_header_full(read).await?;
        Ok((r, header_full.into()))
    }

    async fn read_header_full(mut read: R) -> io::Result<(R, BmpHeaderFull)> {
        task::wait(move || {
            let mut h = BmpHeaderFull::default();
            h.read(&mut read)?;
            Ok((read, h))
        })
        .await
    }

    /// Reads the header and starts the decoder task.
    ///
    /// Note that the ICC profile is not in the header but trailing after the pixels so
    /// progressive rendering may show incorrect colors.
    pub async fn start(read: R) -> io::Result<Decoder<R>> {
        let (read, header) = Self::read_header_full(read).await?;

        todo!()
    }

    /// Read and decode a pixel line
    pub async fn read_line(&mut self) -> Result<Bgra8Buf, ReadTaskError> {
        todo!()
    }

    /// Read all lines to the end and the trailing ICC profile if any where defined.
    pub fn read_end(&mut self) -> Result<(Bgra8Buf, Option<lcms2::Profile>), ReadTaskError> {
        todo!()
    }
}
