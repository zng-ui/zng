//! BMP async streaming decoder and encoder.
//!
//! See [https://en.wikipedia.org/wiki/BMP_file_format] for details about the format.

use std::io;

use super::*;
use crate::task::{
    self,
    io::{ReadTask, ReadTaskError},
};

/// BMP header info.
#[derive(Debug, Clone)]
pub struct BmpHeader {
    /// Pixel width.
    pub width: u32,
    /// Pixel height.
    pub height: u32,
    /// Pixel-per-meter horizontal resolution.
    pub ppm_x: u32,
    ///  Pixel-per-meter vertical resolution.
    pub ppm_y: u32,

    /// Direction of the pixel rows.
    ///
    /// Progressive loading depends on what bytes are at the
    /// begining of the file, for BMP files this can be either the top pixel row
    /// or the bottom depending on this value. Most BMP files are bottom-to-top.
    pub row_direction: RowDirection,
}
impl From<BmpHeaderFull> for BmpHeader {
    fn from(bmp: BmpHeaderFull) -> Self {
        BmpHeader {
            width: bmp.width,
            height: bmp.height,
            ppm_x: bmp.ppm_x,
            ppm_y: bmp.ppm_y,
            row_direction: bmp.row_direction,
        }
    }
}

const DEFAULT_PPM: u32 = 3780; // 96dpi

enum Halftoning {
    None,
    ErrorDiffusion(u8),
    Panda(u32, u32), // Processing Algorithm for Noncoded Document Acquisition
    SuperCircle(u32, u32),
}

#[derive(Clone, Copy)]
#[repr(u8)]
enum Bpp {
    // Palette
    V1 = 1,
    V4 = 4,
    V8 = 8,

    // RGB
    V16 = 16,
    V24 = 24,

    // RGB(A)
    V32 = 32,
}
impl Bpp {
    fn new(bpp: u16) -> io::Result<Bpp> {
        match bpp {
            1 => Ok(Bpp::V1),
            4 => Ok(Bpp::V4),
            8 => Ok(Bpp::V8),
            16 => Ok(Bpp::V16),
            24 => Ok(Bpp::V24),
            32 => Ok(Bpp::V32),
            _ => Err(invalid_data("unknown bpp")),
        }
    }
}

enum CompressionMtd {
    Rgb,
    Rle8,
    Rle4,
    Bitfields,
    Jpeg,
    Png,
    AlphaBitfields,
    // only .wmf
    // Cmyk,
    // CmykRle8,
    // CmykRle4,
}
impl CompressionMtd {
    fn new(compression: u32) -> io::Result<Self> {
        match compression {
            0 => Ok(CompressionMtd::Rgb),
            1 => Ok(CompressionMtd::Rle8),
            2 => Ok(CompressionMtd::Rle4),
            3 => Ok(CompressionMtd::Bitfields),
            4 => Ok(CompressionMtd::Jpeg),
            5 => Ok(CompressionMtd::Png),
            6 => Ok(CompressionMtd::AlphaBitfields),
            11 | 12 | 13 => Err(invalid_data("WMF CMYK compression not supported in BMP")),
            // 11 => Ok(CompressionMtd::Cmyk),
            // 12 => Ok(CompressionMtd::CmykRle8),
            // 13 => Ok(CompressionMtd::CmykRle4),
            _ => Err(invalid_data("unknown compression method")),
        }
    }
}

/// Direction the pixel rows are read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowDirection {
    /// The most common, image loads from the bottom.
    BottomUp,
    /// Image loads from the top.
    TopDown,
}

enum ColorSpaceType {
    CalibratedRgb,
    Srgb,
    Windows,
    Linked,
    Embedded,
}
impl ColorSpaceType {
    fn new(cs_type: u32) -> io::Result<ColorSpaceType> {
        // https://docs.microsoft.com/en-us/openspecs/windows_protocols/ms-wmf/eb4bbd50-b3ce-4917-895c-be31f214797f
        match cs_type {
            0 => Ok(ColorSpaceType::CalibratedRgb),
            0x73524742 => Ok(ColorSpaceType::Srgb),
            0x57696E20 => Ok(ColorSpaceType::Windows),
            0x4C494E4B => Ok(ColorSpaceType::Linked),
            0x4D424544 => Ok(ColorSpaceType::Embedded),
            _ => Err(invalid_data("unknown color-space type")),
        }
    }
}

enum CmsIntent {
    Business = 1,
    Graphics = 2,
    Images = 4,
    Colorimetric = 8,
}
impl CmsIntent {
    fn new(intent: u32) -> io::Result<CmsIntent> {
        match intent {
            1 => Ok(CmsIntent::Business),
            2 => Ok(CmsIntent::Graphics),
            4 => Ok(CmsIntent::Images),
            8 => Ok(CmsIntent::Colorimetric),
            _ => Err(invalid_data("unknown color management intent")),
        }
    }
}

struct BmpHeaderFull {
    header_size: u32,

    width: u32,
    height: u32,
    bpp: Bpp,
    ppm_x: u32,
    ppm_y: u32,
    row_direction: RowDirection,
    halftoning: Halftoning,
    compression: CompressionMtd,
    palette_count: i32,
    palette: Vec<[u8; 4]>,

    red_bitmask: u32,
    green_bitmask: u32,
    blue_bitmask: u32,
    alpha_bitmask: u32,

    color_space_type: ColorSpaceType,
    color_space_endpoints: [u8; 36],
    red_gamma: u32,
    green_gamma: u32,
    blue_gamma: u32,

    cms_intent: CmsIntent,
}
impl Default for BmpHeaderFull {
    fn default() -> Self {
        BmpHeaderFull {
            header_size: 0,
            width: 0,
            height: 0,
            bpp: Bpp::V24,
            ppm_x: DEFAULT_PPM,
            ppm_y: DEFAULT_PPM,
            row_direction: RowDirection::BottomUp,
            halftoning: Halftoning::None,
            compression: CompressionMtd::Rgb,
            palette_count: 0,
            palette: vec![],

            red_bitmask: 0,
            green_bitmask: 0,
            blue_bitmask: 0,
            alpha_bitmask: 0,

            color_space_type: ColorSpaceType::CalibratedRgb,
            color_space_endpoints: [0; 36],
            red_gamma: 0,
            green_gamma: 0,
            blue_gamma: 0,

            cms_intent: CmsIntent::Images,
        }
    }
}
impl BmpHeaderFull {
    // reference:
    // https://searchfox.org/mozilla-central/source/image/decoders/nsBMPDecoder.cpp#197
    // https://en.wikipedia.org/wiki/BMP_file_format

    pub fn read(&mut self, read: &mut impl io::Read) -> io::Result<()> {
        let mut head = ArrayRead::<14>::load(read)?;

        head.match_ascii(b"BM")?;

        self.header_size = head.read_u32_le();

        match self.header_size {
            12 => self.read_core_header(read)?,
            64 => {
                self.read_core_header(read)?;
                self.read_core_header2(read)?;
            }
            16 => {
                self.read_core_header(read)?;
            }
            40 => {
                self.read_info_header(read)?;
                if !matches!(self.compression, CompressionMtd::Rgb | CompressionMtd::Rle4 | CompressionMtd::Rle8) {
                    return Err(invalid_data("compression not supported in BMP-v1"));
                }
            }
            52 => {
                self.read_info_header(read)?;
                if !matches!(
                    self.compression,
                    CompressionMtd::Rgb | CompressionMtd::Rle4 | CompressionMtd::Rle8 | CompressionMtd::Bitfields
                ) {
                    return Err(invalid_data("compression not supported in BMP-v2"));
                }

                if matches!(self.compression, CompressionMtd::Bitfields) {
                    self.read_rgb_bitmasks(read)?;
                }
            }
            56 => {
                self.read_info_header(read)?;
                if matches!(self.compression, CompressionMtd::Jpeg | CompressionMtd::Png) {
                    return Err(invalid_data("compression not supported in BMP-v3"));
                }

                self.maybe_read_bitmasks(read)?;
            }
            108 => {
                self.read_info_header(read)?;
                self.maybe_read_bitmasks(read)?;

                self.read_info_header4(read)?;

                todo!()
            }
            124 => {
                self.read_info_header(read)?;
                self.maybe_read_bitmasks(read)?;

                self.read_info_header4(read)?;
                self.read_info_header5(read)?;
            }

            unknown => return Err(invalid_data(format!("unknown header size `{}`", unknown))),
        }

        self.maybe_read_color_table(read)?;

        match self.compression {
            CompressionMtd::Rle8 => {
                if !matches!(self.bpp, Bpp::V8) {
                    return Err(invalid_data("compression RLE8 but BPP not 8"));
                }
            }
            CompressionMtd::Rle4 => {
                if !matches!(self.bpp, Bpp::V4) {
                    return Err(invalid_data("compression RLE4 but BPP not 4"));
                }
            }
            CompressionMtd::Bitfields => {
                if !matches!(self.bpp, Bpp::V16 | Bpp::V32) {
                    return Err(invalid_data("compression BITFIELDS but BPP not 16 nor 32"));
                }
            }

            CompressionMtd::AlphaBitfields => {
                if !matches!(self.bpp, Bpp::V32) {
                    return Err(invalid_data("compression ALPHABITFIELDS but BPP not 32"));
                }
            }
            _ => {}
        }

        Ok(())
    }

    fn read_core_header(&mut self, read: &mut impl io::Read) -> io::Result<()> {
        let mut h = ArrayRead::<{ 12 - 4 }>::load(read)?;

        self.width = h.read_u16_le() as u32;
        self.height = h.read_u16_le() as u32;

        h.skip(2); // ignore color plane count, always 1

        self.bpp = Bpp::new(h.read_u16_le())?;

        if matches!(self.bpp, Bpp::V16 | Bpp::V32) {
            return Err(invalid_data("invalid bpp for bmp-core"));
        }

        Ok(())
    }

    fn read_core_header2(&mut self, read: &mut impl io::Read) -> io::Result<()> {
        let mut h = ArrayRead::<{ 64 - 12 - 4 }>::load(read)?;

        h.skip(2); // ignore resolution unit, always pixels-per-metre
        h.skip(2); // padding
        h.skip(2); // ignore direction, always bottom-to-top
        let halftoning_u16 = h.read_u16_le();

        self.halftoning = match halftoning_u16 {
            0 => Halftoning::None,
            1 => {
                let pct = h.read_u32_le();
                if pct > 100 {
                    return Err(invalid_data("expected value in the `0..=100` range"));
                }
                Halftoning::ErrorDiffusion(pct as u8)
            }
            2 => Halftoning::Panda(h.read_u32_le(), h.read_u32_le()),
            3 => Halftoning::SuperCircle(h.read_u32_le(), h.read_u32_le()),
            _ => return Err(invalid_data("unknown halftoning algorithm")),
        };

        #[cfg(debug_assertions)]
        {
            h.skip(4); // ignore color encoding, only RGB
            h.skip(4); // ignore tag
        }

        Ok(())
    }

    fn read_info_header(&mut self, read: &mut impl io::Read) -> io::Result<()> {
        let mut header = ArrayRead::<{ 40 - 4 }>::load(read)?;

        let w = header.read_i32_le();
        if w < 0 {
            return Err(invalid_data("width less then zero"));
        }
        self.width = w as u32;

        let mut height = header.read_i32_le();
        if height < 0 {
            height *= -1;
            self.row_direction = RowDirection::TopDown;
        }
        self.height = height as u32;

        header.skip(2); // ignore number of color planes

        self.bpp = Bpp::new(header.read_u16_le())?;

        self.compression = CompressionMtd::new(header.read_u32_le())?;

        if matches!(self.row_direction, RowDirection::TopDown) && !matches!(self.compression, CompressionMtd::Rgb) {
            return Err(invalid_data("top-down images cannot be compressed"));
        }

        header.skip(4); // ignore image size in bytes

        self.ppm_x = header.read_i32_le().max(0) as u32;
        self.ppm_y = header.read_i32_le().max(0) as u32;

        self.palette_count = header.read_i32_le();

        #[cfg(debug_assertions)]
        {
            header.skip(4); // ignore "important colors" field.
        }

        Ok(())
    }

    fn maybe_read_bitmasks(&mut self, read: &mut impl io::Read) -> io::Result<()> {
        match self.compression {
            CompressionMtd::Bitfields => self.read_rgb_bitmasks(read)?,
            CompressionMtd::AlphaBitfields => {
                self.read_rgb_bitmasks(read)?;
                self.read_alpha_bitmask(read)?;
            }
            _ => {}
        }
        Ok(())
    }
    fn read_rgb_bitmasks(&mut self, read: &mut impl io::Read) -> io::Result<()> {
        let mut masks = ArrayRead::<{ 4 * 3 }>::load(read)?;

        self.red_bitmask = masks.read_u32_le();
        self.green_bitmask = masks.read_u32_le();
        self.blue_bitmask = masks.read_u32_le();

        Ok(())
    }
    fn read_alpha_bitmask(&mut self, read: &mut impl io::Read) -> io::Result<()> {
        let mut mask = ArrayRead::<4>::load(read)?;
        self.alpha_bitmask = mask.read_u32_le();
        Ok(())
    }

    fn read_info_header4(&mut self, read: &mut impl io::Read) -> io::Result<()> {
        let mut header = ArrayRead::<{ 108 - 56 - 4 }>::load(read)?;

        self.color_space_type = ColorSpaceType::new(header.read_u32_le())?;

        let endpoints = header.read::<36>();

        match self.color_space_type {
            ColorSpaceType::CalibratedRgb => {}
            ColorSpaceType::Embedded => {}
            _ => { }
        }

        self.red_gamma = header.read_u32_le();
        self.green_gamma = header.read_u32_le();
        self.blue_gamma = header.read_u32_le();

        Ok(())
    }

    fn read_info_header5(&mut self, read: &mut impl io::Read) -> io::Result<()> {
        let mut icc_info = ArrayRead::<{ 124 - 108 - 4 }>::load(read)?;

        self.cms_intent = CmsIntent::new(icc_info.read_u32_le())?;

        Ok(())
    }

    fn maybe_read_color_table(&mut self, read: &mut impl io::Read) -> io::Result<()> {
        let bpp = self.bpp as u8;
        if bpp <= 8 {
            let required_count = 1 << bpp;
            if 0 < self.palette_count && self.palette_count < required_count {
                self.palette_count = required_count as i32;
            }
        }

        if self.palette_count > 0 {
            // 256 because some files try to index out of bounds.
            self.palette = vec![[0u8; 4]; 256];
            for c in &mut self.palette[0..self.palette_count as usize] {
                read.read_exact(c)?;
            }
        }

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
    pub async fn read_lines(&mut self, line_count: usize) -> Result<Bgra8Buf, ReadTaskError> {
        todo!()
    }

    /// Read all lines to the end and the trailing ICC profile if any where defined.
    pub async fn read_end(&mut self) -> Result<(Bgra8Buf, Option<lcms2::Profile>), ReadTaskError> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, future::Future, path::PathBuf, time::Duration};

    use super::*;
    use crate::task;

    #[track_caller]
    fn async_test<F>(test: F) -> F::Output
    where
        F: Future,
    {
        task::block_on(task::with_timeout(test, Duration::from_secs(1))).unwrap()
    }

    #[test]
    pub fn bad() {
        let files = fs::read_dir(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../tests/bmp-suite/bad")).unwrap();
        for file in files {
            let file = file.unwrap().path();
            let file_name = file.file_name().unwrap();
            let file = fs::File::open(&file).unwrap();

            async_test(async move {
                if let Ok((_, h)) = Decoder::read_header(file).await {
                    panic!("`{}` did not cause an error on start, result: {:?}", file_name.to_string_lossy(), h)
                }
            });
        }
    }

    #[test]
    pub fn good() {
        let files = fs::read_dir(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../tests/bmp-suite/good")).unwrap();
        for file in files {
            let file = file.unwrap().path();
            let file_name = file.file_name().unwrap();
            let file = fs::File::open(&file).unwrap();

            async_test(async move {
                let mut d = Decoder::start(file).await.unwrap();
                d.read_end().await.unwrap();
            });
        }
    }
}
