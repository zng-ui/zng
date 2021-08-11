//! BMP async streaming decoder and encoder.
//!
//! See [https://en.wikipedia.org/wiki/BMP_file_format] for details about the format.

use std::io;

use rayon::prelude::*;

use super::*;
use crate::task;
use crate::task::SenderTask;
use crate::units::*;

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
impl BmpHeader {
    fn new(bmp: &BmpHeaderFull) -> Self {
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

#[derive(Debug)]
enum Halftoning {
    None,
    ErrorDiffusion(u8),
    Panda(u32, u32), // Processing Algorithm for Noncoded Document Acquisition
    SuperCircle(u32, u32),
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
enum Bpp {
    // Palette
    B1 = 1,
    B4 = 4,
    B8 = 8,

    // RGB
    B16 = 16,
    B24 = 24,

    // RGB(A)
    B32 = 32,
}
impl Bpp {
    fn new(bpp: u16) -> io::Result<Bpp> {
        match bpp {
            1 => Ok(Bpp::B1),
            4 => Ok(Bpp::B4),
            8 => Ok(Bpp::B8),
            16 => Ok(Bpp::B16),
            24 => Ok(Bpp::B24),
            32 => Ok(Bpp::B32),
            _ => Err(invalid_data("unknown bpp")),
        }
    }
}

#[derive(Debug)]
enum CompressionMtd {
    Rgb,
    Rle8,
    Rle4,
    Bitfields,
    // rarely supported, only for printers?
    // Jpeg,
    // Png,
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
            4 => Err(invalid_data("BMP with embedded JPEG is not supported")),
            5 => Err(invalid_data("BMP with embedded PNG is not supported")),
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

#[derive(Debug)]
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

#[derive(Debug)]
enum CmsIntent {
    Business = 1,
    Graphics = 2,
    Images = 4,
    AbsColorimetric = 8,
}
impl CmsIntent {
    fn new(intent: u32) -> io::Result<CmsIntent> {
        match intent {
            1 => Ok(CmsIntent::Business),
            2 => Ok(CmsIntent::Graphics),
            4 | 0 => Ok(CmsIntent::Images),
            8 => Ok(CmsIntent::AbsColorimetric),
            n => Err(invalid_data(format!("unknown color management intent: {}", n))),
        }
    }
}

#[derive(Debug, Default)]
struct RgbEndpoint {
    mx: u32,
    my: u32,
    mz: u32,
    gamma: u32,
}

#[derive(Debug)]
struct BmpHeaderFull {
    header_size: u32,
    gap1: usize,

    width: u32,
    height: u32,
    bpp: Bpp,
    ppm_x: u32,
    ppm_y: u32,
    row_direction: RowDirection,
    halftoning: Halftoning,
    compression: CompressionMtd,
    palette_count: u32,
    palette: Vec<[u8; 4]>,

    red_bitmask: u32,
    green_bitmask: u32,
    blue_bitmask: u32,
    alpha_bitmask: u32,

    color_space_type: ColorSpaceType,
    red_endpoint: RgbEndpoint,
    green_endpoint: RgbEndpoint,
    blue_endpoint: RgbEndpoint,
    red_gamma: u32,
    green_gamma: u32,
    blue_gamma: u32,

    cms_intent: CmsIntent,
    gap2: usize,
    icc_size: u32,
}
impl Default for BmpHeaderFull {
    fn default() -> Self {
        BmpHeaderFull {
            header_size: 0,
            gap1: 0,
            width: 0,
            height: 0,
            bpp: Bpp::B24,
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
            red_endpoint: RgbEndpoint::default(),
            green_endpoint: RgbEndpoint::default(),
            blue_endpoint: RgbEndpoint::default(),
            red_gamma: 0,
            green_gamma: 0,
            blue_gamma: 0,

            cms_intent: CmsIntent::Images,
            gap2: 0,
            icc_size: 0,
        }
    }
}
impl BmpHeaderFull {
    // reference:
    // https://searchfox.org/mozilla-central/source/image/decoders/nsBMPDecoder.cpp#197
    // https://en.wikipedia.org/wiki/BMP_file_format

    pub async fn read<R>(&mut self, read: &mut R) -> Result<(), R::Error>
    where
        R: task::ReadThenReceive,
    {
        let mut head = ArrayRead::<{ 14 + 4 }>::load(read).await?;

        head.match_ascii(b"BM")?;

        head.skip(4); // file size
        head.skip(2); // reserved
        head.skip(2); // reserved

        self.gap1 = head.read_u32_le() as usize;

        self.header_size = head.read_u32_le();
        self.gap1 -= self.header_size as usize;

        match self.header_size {
            12 => self.read_core_header(read).await?,
            64 => {
                self.read_core_header(read).await?;
                self.read_core_header2(read).await?;
            }
            16 => {
                self.read_core_header(read).await?;
                let _ = read.read_exact::<4>().await?;
            }
            40 => {
                self.read_info_header(read).await?;
                // bitfields because case: g/rgb16-565.bmp
                if !matches!(
                    self.compression,
                    CompressionMtd::Rgb | CompressionMtd::Rle4 | CompressionMtd::Rle8 | CompressionMtd::Bitfields
                ) {
                    return Err(invalid_data(format!("compression `{:?}` not supported in BMP-v1", self.compression)).into());
                }
            }
            52 => {
                self.read_info_header(read).await?;
                if !matches!(
                    self.compression,
                    CompressionMtd::Rgb | CompressionMtd::Rle4 | CompressionMtd::Rle8 | CompressionMtd::Bitfields
                ) {
                    return Err(invalid_data("compression not supported in BMP-v2").into());
                }

                if matches!(self.compression, CompressionMtd::Bitfields) {
                    self.read_rgb_bitmasks(read).await?;
                }
            }
            56 => {
                self.read_info_header(read).await?;
                let _ = read.read_exact::<4>().await?;

                // we don't support these compressions
                // if matches!(self.compression, CompressionMtd::Jpeg | CompressionMtd::Png) {
                //     return Err(invalid_data("compression not supported in BMP-v3").into());
                // }

                self.maybe_read_bitmasks(read).await?;
            }
            108 => {
                self.read_info_header(read).await?;
                self.maybe_read_bitmasks(read).await?;

                self.read_info_header4(read).await?;
            }
            124 => {
                self.read_info_header(read).await?;
                self.maybe_read_bitmasks(read).await?;

                self.read_info_header4(read).await?;
                self.read_info_header5(read).await?;
            }

            unknown => return Err(invalid_data(format!("unknown header size `{}`", unknown)).into()),
        }

        match self.compression {
            CompressionMtd::Rle8 => {
                if !matches!(self.bpp, Bpp::B8) {
                    return Err(invalid_data("compression RLE8 but BPP not 8").into());
                }
            }
            CompressionMtd::Rle4 => {
                if !matches!(self.bpp, Bpp::B4) {
                    return Err(invalid_data("compression RLE4 but BPP not 4").into());
                }
            }
            CompressionMtd::Bitfields => {
                if !matches!(self.bpp, Bpp::B16 | Bpp::B32) {
                    return Err(invalid_data("compression BITFIELDS but BPP not 16 nor 32").into());
                }
            }

            CompressionMtd::AlphaBitfields => {
                if !matches!(self.bpp, Bpp::B32) {
                    return Err(invalid_data("compression ALPHABITFIELDS but BPP not 32").into());
                }
            }
            _ => {}
        }

        self.maybe_read_palette(read).await?;

        Ok(())
    }

    async fn read_core_header<R>(&mut self, read: &mut R) -> Result<(), R::Error>
    where
        R: task::ReadThenReceive,
    {
        let mut h = ArrayRead::<{ 12 - 4 }>::load(read).await?;

        self.width = h.read_u16_le() as u32;
        self.height = h.read_u16_le() as u32;

        h.skip(2); // ignore color plane count, always 1

        self.bpp = Bpp::new(h.read_u16_le())?;

        if matches!(self.bpp, Bpp::B16 | Bpp::B32) {
            return Err(invalid_data("invalid bpp for bmp-core").into());
        }

        Ok(())
    }

    async fn read_core_header2<R>(&mut self, read: &mut R) -> Result<(), R::Error>
    where
        R: task::ReadThenReceive,
    {
        let mut h = ArrayRead::<{ 64 - 12 - 4 }>::load(read).await?;

        h.skip(2); // ignore resolution unit, always pixels-per-metre
        h.skip(2); // padding
        h.skip(2); // ignore direction, always bottom-to-top
        let halftoning_u16 = h.read_u16_le();

        self.halftoning = match halftoning_u16 {
            0 => Halftoning::None,
            1 => {
                let pct = h.read_u32_le();
                if pct > 100 {
                    return Err(invalid_data("expected value in the `0..=100` range").into());
                }
                Halftoning::ErrorDiffusion(pct as u8)
            }
            2 => Halftoning::Panda(h.read_u32_le(), h.read_u32_le()),
            3 => Halftoning::SuperCircle(h.read_u32_le(), h.read_u32_le()),
            _ => return Err(invalid_data("unknown halftoning algorithm").into()),
        };

        #[cfg(debug_assertions)]
        {
            h.skip(4); // ignore color encoding, only RGB
            h.skip(4); // ignore tag
        }

        Ok(())
    }

    async fn read_info_header<R>(&mut self, read: &mut R) -> Result<(), R::Error>
    where
        R: task::ReadThenReceive,
    {
        let mut header = ArrayRead::<{ 40 - 4 }>::load(read).await?;

        let w = header.read_i32_le();
        if w < 0 {
            return Err(invalid_data("width less then zero").into());
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
            return Err(invalid_data("top-down images cannot be compressed").into());
        }

        header.skip(4); // ignore image size in bytes

        let ppm_x = header.read_i32_le();
        let ppm_y = header.read_i32_le();

        // 1_181_100 is 29_999dpi, the maximum in Photoshop, still a ridiculous number.
        if ppm_x > 0 && ppm_x <= 1_181_100 && ppm_y > 0 && ppm_y <= 1_181_100 {
            if ppm_x != ppm_y {
                // validate the pixel aspect ratio < 4:1
                let max = self.ppm_x.max(self.ppm_y) as f32;
                let min = self.ppm_x.min(self.ppm_y) as f32;

                let ratio = max / min;
                if ratio < 4.0 {
                    self.ppm_x = ppm_x as u32;
                    self.ppm_y = ppm_y as u32;
                }
            } else {
                self.ppm_x = ppm_x as u32;
                self.ppm_y = ppm_y as u32;
            }
        }

        let palette_count = header.read_i32_le();
        if !(0..=256).contains(&palette_count) {
            return Err(invalid_data("incorrect palette colors count").into());
        }
        self.palette_count = palette_count as u32;

        #[cfg(debug_assertions)]
        {
            header.skip(4); // ignore "important colors" field.
        }

        Ok(())
    }

    async fn maybe_read_bitmasks<R>(&mut self, read: &mut R) -> Result<(), R::Error>
    where
        R: task::ReadThenReceive,
    {
        match self.compression {
            CompressionMtd::Bitfields => self.read_rgb_bitmasks(read).await?,
            CompressionMtd::AlphaBitfields => {
                self.read_rgb_bitmasks(read).await?;
                self.read_alpha_bitmask(read).await?;
            }
            _ => {}
        }
        Ok(())
    }
    async fn read_rgb_bitmasks<R>(&mut self, read: &mut R) -> Result<(), R::Error>
    where
        R: task::ReadThenReceive,
    {
        let mut masks = ArrayRead::<{ 4 * 3 }>::load(read).await?;

        self.red_bitmask = masks.read_u32_le();
        self.green_bitmask = masks.read_u32_le();
        self.blue_bitmask = masks.read_u32_le();

        Ok(())
    }
    async fn read_alpha_bitmask<R>(&mut self, read: &mut R) -> Result<(), R::Error>
    where
        R: task::ReadThenReceive,
    {
        let mut mask = ArrayRead::<4>::load(read).await?;
        self.alpha_bitmask = mask.read_u32_le();
        Ok(())
    }

    async fn read_info_header4<R>(&mut self, read: &mut R) -> Result<(), R::Error>
    where
        R: task::ReadThenReceive,
    {
        let mut header = ArrayRead::<{ 108 - 52 - 4 }>::load(read).await?;

        self.color_space_type = ColorSpaceType::new(header.read_u32_le())?;

        self.red_endpoint.mx = header.read_u32_le();
        self.red_endpoint.my = header.read_u32_le();
        self.red_endpoint.mz = header.read_u32_le();

        self.green_endpoint.mx = header.read_u32_le();
        self.green_endpoint.my = header.read_u32_le();
        self.green_endpoint.mz = header.read_u32_le();

        self.blue_endpoint.mx = header.read_u32_le();
        self.blue_endpoint.my = header.read_u32_le();
        self.blue_endpoint.mz = header.read_u32_le();

        self.red_endpoint.gamma = header.read_u32_le();
        self.green_endpoint.gamma = header.read_u32_le();
        self.blue_endpoint.gamma = header.read_u32_le();

        Ok(())
    }

    async fn read_info_header5<R>(&mut self, read: &mut R) -> Result<(), R::Error>
    where
        R: task::ReadThenReceive,
    {
        let mut icc_info = ArrayRead::<{ 124 - 108 - 4 }>::load(read).await?;

        let icc_offset = icc_info.read_u32_le();
        self.cms_intent = CmsIntent::new(icc_info.read_u32_le())?;
        self.gap2 = icc_offset as usize - self.header_size as usize - self.gap1 - self.palette_count as usize;
        self.icc_size = icc_info.read_u32_le();

        // wiki diagram expects this
        //#[cfg(debug_assertions)]
        //{
        //    icc_info.read_u32_le(); // reserved
        //}

        Ok(())
    }

    async fn maybe_read_palette<R>(&mut self, read: &mut R) -> Result<(), R::Error>
    where
        R: task::ReadThenReceive,
    {
        let bpp = self.bpp as u8;
        if bpp <= 8 {
            let required_count = 1 << bpp;
            if required_count < self.palette_count {
                self.palette_count = required_count;
            }
        }

        if self.palette_count > 0 {
            self.gap1 -= self.palette_count as usize;
            // 256 because some files try to index out of bounds.
            self.palette = vec![[0u8; 4]; 256];
            for c in &mut self.palette[0..self.palette_count as usize] {
                *c = read.read_exact::<4>().await?;
            }
        }

        Ok(())
    }
}

/// BMP async streaming reader.
pub struct Decoder<R> {
    header: BmpHeaderFull,

    task: R,
    pending_lines: u32,
    next_line: Vec<u8>,
}
impl<R> Decoder<R> {
    /// Header info.
    pub fn header(&self) -> BmpHeader {
        BmpHeader::new(&self.header)
    }
}
impl Decoder<()> {
    async fn read_header_full<R>(read: &mut R) -> Result<BmpHeaderFull, R::Error>
    where
        R: task::ReadThenReceive,
    {
        let mut header = BmpHeaderFull::default();
        header.read(read).await?;
        Ok(header)
    }

    /// Read the header only.
    pub async fn read_header<R>(read: &mut R) -> Result<BmpHeader, R::Error>
    where
        R: task::ReadThenReceive,
    {
        let header_full = Decoder::read_header_full(read).await?;
        Ok(BmpHeader::new(&header_full))
    }
}
impl<R: task::ReceiverTask> Decoder<R> {
    /// Reads the header and starts the decoder task.
    ///
    /// Note that the ICC profile is not in the header but trailing after the pixels so
    /// progressive rendering may show incorrect colors.
    pub async fn start<RR>(mut read: RR, max_bytes: ByteLength) -> Result<Self, RR::Error>
    where
        RR: task::ReadThenReceive<Spawned = R>,
    {
        let header = Decoder::read_header_full(&mut read).await?;

        check_limit(header.width, header.height, 4.bytes(), max_bytes)?;

        read.read_exact_heap(header.gap1.bytes()).await?;

        let payload_len = match header.compression {
            CompressionMtd::Rgb => {
                let len = header.width as usize * (header.bpp as usize / 8);
                let padding = len % 4;
                len + padding
            }
            CompressionMtd::Rle8 | CompressionMtd::Rle4 => header.width as usize * 2,
            CompressionMtd::Bitfields | CompressionMtd::AlphaBitfields => header.width as usize,
        };

        let task = read.spawn(payload_len.bytes(), header.height as usize);

        Ok(Decoder {
            pending_lines: header.height,
            next_line: vec![],
            header,
            task,
        })
    }

    /// Read and decode some pixel lines.
    pub async fn read_lines(&mut self, line_count: u32) -> Result<Bgra8Buf, R::Error> {
        let count = line_count.max(self.pending_lines) as usize;
        if count == 0 {
            return Ok(Bgra8Buf::empty());
        }

        match self.header.compression {
            CompressionMtd::Rgb => self.read_rgb_lines(line_count).await,
            CompressionMtd::Rle8 => self.read_rle_lines(line_count, false).await,
            CompressionMtd::Rle4 => self.read_rle_lines(line_count, true).await,
            CompressionMtd::Bitfields => self.read_bitfield_lines(line_count).await,
            CompressionMtd::AlphaBitfields => self.read_bitfield_lines(line_count).await,
        }
    }

    /// Read all lines to the end and the trailing ICC profile if any where defined.
    pub async fn read_end(&mut self) -> Result<(Bgra8Buf, Option<lcms2::Profile>), R::Error> {
        let rest = self.read_lines(self.pending_lines).await?;

        let icc = if self.header.icc_size > 0 { todo!() } else { None };

        Ok((rest, icc))
    }

    async fn read_rgb_lines(&mut self, line_count: u32) -> Result<Bgra8Buf, R::Error> {
        match self.header.bpp {
            Bpp::B1 => todo!(),
            Bpp::B4 => todo!(),
            Bpp::B8 => todo!(),
            Bpp::B16 => todo!(),
            // BGR8
            Bpp::B24 => {
                let line_len = self.header.width as usize * 3;
                let mut r = Bgra8Buf::with_capacity(self.header.width as usize * 4 * line_count as usize);
                for _ in 0..line_count {
                    let mut line = self.task.recv().await?;
                    if line.len() < line_len {
                        return Err(unexpected_eof("did not contain all pixels").into());
                    } else {
                        line.truncate(line_len); // remove padding
                        r.extend(Bgra8Buf::from_bgr8(line));
                    }
                }
                self.pending_lines -= line_count;
                Ok(r)
            }
            // BGRA8 or _BGR8
            Bpp::B32 => {
                let line_len = self.header.width as usize * 4;
                let mut r = Bgra8Buf::with_capacity(line_len * line_count as usize);
                for _ in 0..line_count {
                    let mut line = self.task.recv().await?;
                    if line.len() < line_len {
                        return Err(unexpected_eof("did not contain all pixels").into());
                    } else {
                        line.truncate(line_len); // remove padding
                        r.extend(Bgra8Buf::from_bgra8(line));
                    }
                }
                self.pending_lines -= line_count;
                Ok(r)
            }
        }
    }

    async fn read_rle_lines(&mut self, line_count: u32, rle8: bool) -> Result<Bgra8Buf, R::Error> {
        todo!()
    }

    async fn read_bitfield_lines(&mut self, line_count: u32) -> Result<Bgra8Buf, R::Error> {
        todo!()
    }
}

/// An async streaming BMP encoder.
pub struct Encoder<S> {
    task: S,
    opaque: bool,
    pending_bytes: usize,
}
impl<S: SenderTask> Encoder<S> {
    /// Write the BMP header and starts the writer task.
    pub async fn start(width: u32, height: u32, ppm: Ppm, opaque: bool, task: S) -> Result<Self, S::Error> {
        let mut header = vec![0; 54];
        header.extend(b"BM");

        let pixels_size = width * height * if opaque { 3 } else { 4 };

        header.extend(u32::to_be_bytes(54 + pixels_size)); // file size

        header.extend([0; 4]); // unused

        header.extend(u32::to_be_bytes(54 - header.len() as u32)); // pixels offset.

        header.extend(40u32.to_be_bytes()); // header size
        header.extend(width.to_be_bytes());
        header.extend(height.to_be_bytes());
        header.extend(1u16.to_be_bytes()); // 1 plane
        header.extend(if opaque { 24u16 } else { 32u16 }.to_be_bytes());

        let ppm = (ppm.max(0.dpi()).0 as u32).to_be_bytes();
        header.extend(ppm); // ppm_x
        header.extend(ppm); // ppm_y

        header.extend([0; 8]); // 0u32 colors in pallete and 0u32 important colors.

        match task.send(header).await {
            Ok(_) => Ok(Encoder {
                task,
                opaque: true,
                pending_bytes: pixels_size as usize,
            }),
            Err(_) => match task.finish().await {
                Err(e) => Err(e),
                Ok(_) => unreachable!(),
            },
        }
    }

    /// Encode and write more image pixels.
    pub async fn write(&mut self, partial_bgra8_payload: Bgra8Buf) -> Result<(), task::SenderTaskClosed> {
        let mut bytes: Vec<u8> = if self.opaque {
            partial_bgra8_payload.into_bgr8()
        } else {
            partial_bgra8_payload.into_bgra8()
        };
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
        let files = fs::read_dir(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../tests/bmp-suite/b")).unwrap();
        for file in files {
            let file_path = file.unwrap().path();
            let file_name = file_path.file_name().unwrap().to_string_lossy().to_string();

            println!("{}", file_name);
            //if file_name != "badpalettesize.bmp" {
            //    continue;
            //}

            #[derive(Clone, Copy)]
            enum Do {
                Allow,
                AllowHeader,
                Expect,
            }

            let allow = match file_name.as_str() {
                // ERROR: Header incorrectly indicates that the file is several GB in size.
                //        Header incorrectly indicates that the bitmap is several GB in size.
                //
                // ALLOW:
                // We don't consider the reported size, but try to read the pixels based on the
                // described layout. And if the file is actually huge it will fail because of the
                // limit checking.
                "badfilesize.bmp" | "badbitssize.bmp" => Do::Allow,
                // ERROR: Header indicates an absurdly large number of bits/pixel.
                //
                // ALLOW: We ignore ppm > 1_181_100ppm (29_999dpi) and that imply a pixel aspect ratio > 4
                "baddens1.bmp" | "baddens2.bmp" => Do::Allow,

                // ERROR: The “planes” setting, which is required to be 1, is not 1.
                //
                // ALLOW: We just assume it is 1, Firefox and Chrome do it too :P
                "badplanes.bmp" => Do::Allow,

                // ERROR: Many of the palette indices used in the image are not present in the palette.
                //
                // ALLOW: We always allocate 256 bytes for the pallete so "out-of-bounds" turns into black pixels.
                "pal8badindex.bmp" => Do::Allow,

                // errors in the pixels only:
                "badrle4.bmp" | "badrle4bis.bmp" | "badrle4ter.bmp" | "badrle.bmp" | "badrlebis.bmp" | "badrleter.bmp"
                | "shortfile.bmp" => Do::AllowHeader,

                _ => Do::Expect,
            };

            async_test(async move {
                let file = task::io::ReadThenReceive::open(file_path).await.unwrap();
                let r = Decoder::start(file, 10.kibi_bytes()).await;
                match allow {
                    Do::Allow | Do::AllowHeader => {
                        let mut d = r.unwrap_or_else(|e| panic!("error decoding allowed bad file `{}` header\n{}", file_name, e));
                        match d.read_end().await {
                            Ok((_, _)) => {
                                if let Do::AllowHeader = allow {
                                    panic!(
                                        "bad file `{}` did not cause an error in pixel decoding, header: {:#?}",
                                        file_name, d.header
                                    );
                                }
                            }
                            Err(e) => {
                                if let Do::Allow = allow {
                                    panic!("error decoding allowed bad file `{}` pixels\n{}", file_name, e);
                                }
                            }
                        }
                    }
                    Do::Expect => {
                        if let Ok(r) = r {
                            panic!(
                                "bad file `{}` did not cause an error in header decoding, header: {:#?}",
                                file_name, r.header
                            )
                        }
                    }
                };
            });
        }
    }

    #[test]
    pub fn good() {
        let files = fs::read_dir(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../tests/bmp-suite/g")).unwrap();
        for file in files {
            let file_path = file.unwrap().path();
            let file_name = file_path.file_name().unwrap().to_string_lossy().to_string();

            //println!("{}", file_name);
            //if file_name != "pal8v5.bmp" {
            //    continue;
            //}

            async_test(async move {
                let file = task::io::ReadThenReceive::open(file_path).await.unwrap();
                let mut d = Decoder::start(file, 10.kibi_bytes())
                    .await
                    .unwrap_or_else(|e| panic!("error decoding good file `{}` header\n{}", file_name, e));

                d.read_end()
                    .await
                    .unwrap_or_else(|e| panic!("error decoding good file `{}` pixels\n{}", file_name, e));
            });
        }
    }
}
