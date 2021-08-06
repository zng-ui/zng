//! Image format detection and pixel layouts.

use std::{convert::TryInto, fmt, io};

use rayon::prelude::*;

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
    /// use zero_ui_core::image::formats::ImageFormat;
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

pub(crate) fn u16_le(h: &[u8], cur: usize) -> u16 {
    u16::from_le_bytes([h[cur], h[cur + 1]])
}

pub(crate) fn u32_le(h: &[u8], cur: usize) -> u32 {
    u32::from_le_bytes(h[cur..cur + 4].try_into().unwrap())
}