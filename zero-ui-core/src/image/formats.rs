//! Image format detection and pixel layouts.

use std::{convert::TryInto, fmt, io, ops::Deref};

use rayon::prelude::*;

use crate::{task, units::*};

/// All supported image formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ImageFormat {
    /// TODO
    Png,
    /// TODO
    Jpeg,
    /// TODO
    Tiff,
    /// TODO
    Ico,
    /// TODO
    Cur,
    /// TODO
    Gif,
    /// See [`bmp`].
    ///
    /// [`bmp`]: crate::image::formats::bmp
    Bmp,
    /// TODO
    Tga,
    /// See [`farbfeld`].
    ///
    /// [`farbfeld`]: crate::image::formats::farbfeld
    Farbfeld,
}
impl ImageFormat {
    /// Gets the format that matches the `extension`.
    ///
    /// # Examples
    ///
    /// ```
    /// use zero_ui_core::image::ImageFormat;
    ///
    /// fn format(path: &std::path::Path) -> Option<ImageFormat> {
    ///     path.extension().and_then(ImageFormat::from_extension)
    /// }
    /// ```
    pub fn from_extension(extension: &std::ffi::OsStr) -> Option<ImageFormat> {
        match extension.to_ascii_lowercase().to_string_lossy().as_bytes() {
            b"png" => Some(ImageFormat::Png),
            b"jpg" | b"jpeg" => Some(ImageFormat::Jpeg),
            b"tiff" => Some(ImageFormat::Tiff),
            b"ico" => Some(ImageFormat::Ico),
            b"cur" => Some(ImageFormat::Cur),
            b"gif" => Some(ImageFormat::Gif),
            b"bmp" => Some(ImageFormat::Bmp),
            b"tga" => Some(ImageFormat::Tga),
            b"ff" | b"farbfeld" => Some(ImageFormat::Farbfeld),
            _ => None,
        }
    }

    /// Gets a file extension for the format (without the dot).
    pub fn to_extension(self) -> &'static str {
        match self {
            ImageFormat::Png => "png",
            ImageFormat::Jpeg => "jpg",
            ImageFormat::Tiff => "tiff",
            ImageFormat::Ico => "ico",
            ImageFormat::Cur => "cur",
            ImageFormat::Gif => "gif",
            ImageFormat::Bmp => "bmp",
            ImageFormat::Tga => "tga",
            ImageFormat::Farbfeld => "ff",
        }
    }
}

/// A BGRA8 pixel buffer.
#[derive(Clone)]
pub struct Bgra8Buf(Vec<u8>, bool);
impl Bgra8Buf {
    /// Empty buffer.
    #[inline]
    pub fn empty() -> Self {
        Self(vec![], true)
    }

    /// Empty buffer with pre-allocated byte capacity.
    #[inline]
    pub fn with_capacity(bytes_capacity: usize) -> Self {
        Self(Vec::with_capacity(bytes_capacity), true)
    }

    /// New buffer.
    ///
    /// The `opaque` flag indicates if all alpha values are 255. If `None` all alphas will be checked.
    ///
    /// # Panics
    ///
    /// If the `bgra8` length is not divisible by 4.
    pub fn new(bgra8: Vec<u8>, opaque: Option<bool>) -> Self {
        assert!(bgra8.len() % 4 == 0);

        let opaque = opaque.unwrap_or_else(|| !bgra8.par_chunks(4).any(|c| c[3] < 255));

        Self(bgra8, opaque)
    }

    /// Extend this buffer with pixels from `other`.
    pub fn extend(&mut self, other: Self) {
        self.0.extend(other.0);
        self.1 &= other.1;
    }

    /// New buffer from BGRA8.
    ///
    /// # Panics
    ///
    /// If the `bgra8` length is not divisible by 4.
    pub fn from_bgra8(bgra8: Vec<u8>) -> Self {
        Self::new(bgra8, None)
    }

    /// New buffer from BGR8.
    ///
    /// # Panics
    ///
    /// If the `bgr8` length is not divisible by 3.
    pub fn from_bgr8(bgr8: Vec<u8>) -> Self {
        assert!(bgr8.len() % 3 == 0);
        Self(bgr8.par_chunks(3).map(|c| [c[0], c[1], c[2], 255]).flatten().collect(), true)
    }

    /// New buffer from RGBA8.
    ///
    /// # Panics
    ///
    /// If the `rgba8` length is not divisible by 4.
    pub fn from_rgba8(mut rgba8: Vec<u8>) -> Self {
        rgba8.par_chunks_mut(4).for_each(|c| c.swap(0, 2));
        let opaque = !rgba8.par_chunks(4).any(|c| c[3] < 255);
        Self(rgba8, opaque)
    }

    /// New buffer from RGB8.
    ///
    /// # Panics
    ///
    /// If the `rgb8` length is not divisible by 3.
    pub fn from_rgb8(rgb8: Vec<u8>) -> Self {
        assert!(rgb8.len() % 3 == 0);
        Self(rgb8.par_chunks(3).map(|c| [c[2], c[1], c[0], 255]).flatten().collect(), true)
    }

    /// New buffer from Luma8 (single byte-per-pixel grayscale).
    pub fn from_luma8(luma8: Vec<u8>) -> Self {
        Self(luma8.into_par_iter().map(|c| [c, c, c, 255]).flatten().collect(), true)
    }

    /// New buffer from LumaA8 (2 bytes-per-pixel, grayscale and alpha)
    pub fn from_la8(la8: Vec<u8>) -> Self {
        let opaque = !la8.par_chunks(2).any(|c| c[3] < 255);
        let r = la8.par_chunks(2).map(|c| [c[0], c[0], c[0], c[1]]).flatten().collect();
        Self(r, opaque)
    }

    /// New buffer from RGBA16 encoded in pairs of bytes big endian.
    ///
    /// # Panics
    ///
    /// If the `rgba16_be8` length is not divisible by 8.
    pub fn from_rgba16_be8(rgba16_be8: Vec<u8>) -> Self {
        assert!(rgba16_be8.len() % 8 == 0);
        let r = rgba16_be8
            .par_chunks(8)
            .map(|c| {
                let max = u16::MAX as f32;
                let r = ((u16::from_be_bytes([c[0], c[1]]) as f32 / max) * 255.0) as u8;
                let g = ((u16::from_be_bytes([c[2], c[3]]) as f32 / max) * 255.0) as u8;
                let b = ((u16::from_be_bytes([c[4], c[5]]) as f32 / max) * 255.0) as u8;
                let a = ((u16::from_be_bytes([c[6], c[7]]) as f32 / max) * 255.0) as u8;
                [r, g, b, a]
            })
            .flatten()
            .collect();

        Self::new(r, None)
    }
}
impl Bgra8Buf {
    /// Returns `true` if all alpha values are 255.
    #[inline]
    pub fn opaque(&self) -> bool {
        self.1
    }

    /// Into BGRA8.
    #[inline]
    pub fn into_bgra8(self) -> Vec<u8> {
        self.0
    }

    /// Into re-multiplied BGRA8.
    pub fn into_bgra8_premultiplied(mut self) -> Vec<u8> {
        if !self.1 {
            self.0.par_chunks_mut(4).for_each(|c| {
                if c[3] < 255 {
                    let a = c[3] as f32 / 255.0;
                    c[0..3].iter_mut().for_each(|c| *c = (*c as f32 * a) as u8);
                }
            });
        }
        self.0
    }

    /// Into BGR8.
    #[inline]
    pub fn into_bgr8(self) -> Vec<u8> {
        self.0.par_chunks(4).map(|c| [c[0], c[1], c[2]]).flatten().collect()
    }

    /// Into RGBA8.
    #[inline]
    pub fn into_rgba8(mut self) -> Vec<u8> {
        self.0.par_chunks_mut(4).for_each(|c| c.swap(0, 2));
        self.0
    }

    /// Into RGB8.
    #[inline]
    pub fn into_rgb8(self) -> Vec<u8> {
        self.0.par_chunks(4).map(|c| [c[2], c[1], c[0]]).flatten().collect()
    }

    /// Into RGBA16 values encoded as big-endian byte pairs.
    pub fn into_rgba16_be8(self) -> Vec<u8> {
        self.0
            .par_chunks(4)
            .map(|c| {
                fn cv(c: u8) -> [u8; 2] {
                    let c = (c as f32 / 255.0) * u16::MAX as f32;
                    (c as u16).to_be_bytes()
                }
                let c = [cv(c[2]), cv(c[1]), cv(c[0]), cv(c[3])];
                [c[0][0], c[0][1], c[1][0], c[1][1], c[2][0], c[2][1], c[3][0], c[3][1]]
            })
            .flatten()
            .collect()
    }
}
impl fmt::Debug for Bgra8Buf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Bgra8Buf(<{} bytes>, opaque: {})", self.0.len(), self.1)
    }
}
impl Deref for Bgra8Buf {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

pub(crate) fn check_limit(width: u32, height: u32, bytes_per_pixel: usize, max_bytes: usize) -> io::Result<()> {
    let width = width as usize;
    let height = height as usize;

    let result = width
        .checked_mul(bytes_per_pixel)
        .and_then(|r| r.checked_mul(height))
        .unwrap_or(usize::MAX);

    if result > max_bytes {
        Err(io::Error::new(io::ErrorKind::OutOfMemory, "image too large"))
    } else {
        Ok(())
    }
}

pub(crate) fn invalid_data(error: impl ToString) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, error.to_string())
}

pub(crate) fn unexpected_eof(error: impl ToString) -> io::Error {
    io::Error::new(io::ErrorKind::UnexpectedEof, error.to_string())
}

pub(crate) struct ArrayRead<const N: usize> {
    buf: [u8; N],
    cur: usize,
}
impl<const N: usize> ArrayRead<N> {
    pub async fn load<R: task::ReadThenReceive>(read: &mut R) -> Result<Self, R::Error> {
        let buf = read.read_exact::<N>().await?;
        Ok(ArrayRead { buf, cur: 0 })
    }

    pub fn match_ascii(&mut self, ascii: &'static [u8]) -> io::Result<()> {
        let matches = ascii == &self.buf[self.cur..self.cur + ascii.len()];
        if matches {
            self.cur += ascii.len();
            debug_assert!(self.cur <= self.buf.len());
            Ok(())
        } else {
            Err(invalid_data(format!("expected `{}`", std::str::from_utf8(ascii).unwrap())))
        }
    }

    pub fn read<const R: usize>(&mut self) -> [u8; R] {
        let r = self.buf[self.cur..self.cur + R].try_into().unwrap();
        self.cur += R;
        r
    }

    pub fn read_u32_be(&mut self) -> u32 {
        u32::from_be_bytes(self.read::<4>())
    }

    pub fn read_u16_le(&mut self) -> u16 {
        u16::from_le_bytes(self.read::<2>())
    }

    pub fn read_u32_le(&mut self) -> u32 {
        u32::from_le_bytes(self.read::<4>())
    }

    pub fn read_i32_le(&mut self) -> i32 {
        i32::from_le_bytes(self.read::<4>())
    }

    pub fn skip(&mut self, byte_count: usize) {
        self.cur += byte_count;
        debug_assert!(self.cur <= self.buf.len());
    }
}

/// Limits of an image decoder.
///
///
#[derive(Clone, Copy, Debug)]
pub struct DecoderLimit {
    /// Maximum pixel width.
    ///
    /// `30_000` by default.
    pub max_width: u32,
    /// Maximum pixel height.
    ///
    /// `30_000` by default.
    pub max_height: u32,

    /// Maximum decoded byte size.
    ///
    /// `2.gibi_bytes()` by default.
    pub max_size: ByteLength,

    /// Maximum byte size during decoding.
    ///
    /// `4.gibi_bytes()` by default.
    pub max_temporary_size: ByteLength,
}
impl Default for DecoderLimit {
    fn default() -> Self {
        DecoderLimit {
            max_width: 30_000,
            max_height: 30_000,
            max_size: 1.gibi_bytes(),
            max_temporary_size: 2.gibi_bytes(),
        }
    }
}
impl DecoderLimit {
    /// Compute a limit the is the maximum of each field between `self` and `other`.
    pub fn max(self, other: DecoderLimit) -> DecoderLimit {
        DecoderLimit {
            max_width: self.max_width.max(other.max_width),
            max_height: self.max_height.max(other.max_height),
            max_size: self.max_size.max(other.max_size),
            max_temporary_size: self.max_temporary_size.max(other.max_temporary_size),
        }
    }

    /// Compute a limit the is the minimum of each field between `self` and `other`.
    pub fn min(self, other: DecoderLimit) -> DecoderLimit {
        DecoderLimit {
            max_width: self.max_width.min(other.max_width),
            max_height: self.max_height.min(other.max_height),
            max_size: self.max_size.min(other.max_size),
            max_temporary_size: self.max_temporary_size.min(other.max_temporary_size),
        }
    }
}
