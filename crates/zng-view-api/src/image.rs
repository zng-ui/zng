//! Image types.

use std::fmt;

use serde::{Deserialize, Serialize};
use zng_txt::Txt;

use crate::ipc::IpcBytes;
use zng_unit::{Px, PxSize};

crate::declare_id! {
    /// Id of a decoded image in the cache.
    ///
    /// The View Process defines the ID.
    pub struct ImageId(_);

    /// Id of an image loaded in a renderer.
    ///
    /// The View Process defines the ID.
    pub struct ImageTextureId(_);
}

/// Defines how the A8 image mask pixels are to be derived from a source mask image.
#[derive(Debug, Copy, Clone, Serialize, PartialEq, Eq, Hash, Deserialize, Default)]
#[non_exhaustive]
pub enum ImageMaskMode {
    /// Alpha channel.
    ///
    /// If the image has no alpha channel masks by `Luminance`.
    #[default]
    A,
    /// Blue channel.
    ///
    /// If the image has no color channel fallback to monochrome channel, or `A`.
    B,
    /// Green channel.
    ///
    /// If the image has no color channel fallback to monochrome channel, or `A`.
    G,
    /// Red channel.
    ///
    /// If the image has no color channel fallback to monochrome channel, or `A`.
    R,
    /// Relative luminance.
    ///
    /// If the image has no color channel fallback to monochrome channel, or `A`.
    Luminance,
}

/// Represent a image load/decode request.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ImageRequest<D> {
    /// Image data format.
    pub format: ImageDataFormat,
    /// Image data.
    ///
    /// Bytes layout depends on the `format`, data structure is [`IpcBytes`] or [`IpcBytesReceiver`] in the view API.
    ///
    /// [`IpcBytesReceiver`]: crate::IpcBytesReceiver
    pub data: D,
    /// Maximum allowed decoded size.
    ///
    /// View-process will avoid decoding and return an error if the image decoded to BGRA (4 bytes) exceeds this size.
    /// This limit applies to the image before the `resize_to_fit`.
    pub max_decoded_len: u64,
    /// A size constraints to apply after the image is decoded. The image is resized so both dimensions fit inside
    /// the constraints, the image aspect ratio is preserved.
    pub downscale: Option<ImageDownscale>,
    /// Convert or decode the image into a single channel mask (R8).
    pub mask: Option<ImageMaskMode>,
}
impl<D> ImageRequest<D> {
    /// New request.
    pub fn new(
        format: ImageDataFormat,
        data: D,
        max_decoded_len: u64,
        downscale: Option<ImageDownscale>,
        mask: Option<ImageMaskMode>,
    ) -> Self {
        Self {
            format,
            data,
            max_decoded_len,
            downscale,
            mask,
        }
    }
}

/// Defines how an image is downscaled after decoding.
///
/// The image aspect ratio is preserved in both modes, the image is not upscaled, if it already fits the size
/// constraints if will not be resized.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ImageDownscale {
    /// Image is downscaled so that both dimensions fit inside the size.
    Fit(PxSize),
    /// Image is downscaled so that at least one dimension fits inside the size.
    Fill(PxSize),
}
impl From<PxSize> for ImageDownscale {
    /// Fit
    fn from(fit: PxSize) -> Self {
        ImageDownscale::Fit(fit)
    }
}
impl From<Px> for ImageDownscale {
    /// Fit splat
    fn from(fit: Px) -> Self {
        ImageDownscale::Fit(PxSize::splat(fit))
    }
}
#[cfg(feature = "var")]
zng_var::impl_from_and_into_var! {
    fn from(fit: PxSize) -> ImageDownscale;
    fn from(fit: Px) -> ImageDownscale;
    fn from(some: ImageDownscale) -> Option<ImageDownscale>;
}
impl ImageDownscale {
    /// Compute the expected final size if the downscale is applied on an image of `source_size`.
    pub fn resize_dimensions(self, source_size: PxSize) -> PxSize {
        // code from image crate
        fn resize_dimensions(width: u32, height: u32, n_width: u32, n_height: u32, fill: bool) -> (u32, u32) {
            use std::cmp::max;

            let w_ratio = n_width as f64 / width as f64;
            let h_ratio = n_height as f64 / height as f64;

            let ratio = if fill {
                f64::max(w_ratio, h_ratio)
            } else {
                f64::min(w_ratio, h_ratio)
            };

            let nw = max((width as f64 * ratio).round() as u64, 1);
            let nh = max((height as f64 * ratio).round() as u64, 1);

            if nw > u64::from(u32::MAX) {
                let ratio = u32::MAX as f64 / width as f64;
                (u32::MAX, max((height as f64 * ratio).round() as u32, 1))
            } else if nh > u64::from(u32::MAX) {
                let ratio = u32::MAX as f64 / height as f64;
                (max((width as f64 * ratio).round() as u32, 1), u32::MAX)
            } else {
                (nw as u32, nh as u32)
            }
        }

        let (x, y) = match self {
            ImageDownscale::Fit(s) => resize_dimensions(
                source_size.width.0.max(0) as _,
                source_size.height.0.max(0) as _,
                s.width.0.max(0) as _,
                s.height.0.max(0) as _,
                false,
            ),
            ImageDownscale::Fill(s) => resize_dimensions(
                source_size.width.0.max(0) as _,
                source_size.height.0.max(0) as _,
                s.width.0.max(0) as _,
                s.height.0.max(0) as _,
                true,
            ),
        };
        PxSize::new(Px(x as _), Px(y as _))
    }
}

/// Format of the image bytes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ImageDataFormat {
    /// Decoded BGRA8.
    ///
    /// This is the internal image format, it indicates the image data
    /// is already decoded and color managed (to sRGB).
    Bgra8 {
        /// Size in pixels.
        size: PxSize,
        /// Pixels-per-inch of the image.
        ppi: Option<ImagePpi>,
    },

    /// Decoded A8.
    ///
    /// This is the internal mask format it indicates the mask data
    /// is already decoded.
    A8 {
        /// Size in pixels.
        size: PxSize,
    },

    /// The image is encoded.
    ///
    /// This file extension maybe identifies the format. Fallback to `Unknown` handling if the file extension
    /// is unknown or the file header does not match.
    FileExtension(Txt),

    /// The image is encoded.
    ///
    /// This MIME type maybe identifies the format. Fallback to `Unknown` handling if the file extension
    /// is unknown or the file header does not match.
    MimeType(Txt),

    /// The image is encoded.
    ///
    /// A decoder will be selected using the "magic number" at the start of the bytes buffer.
    Unknown,
}
impl From<Txt> for ImageDataFormat {
    fn from(ext_or_mime: Txt) -> Self {
        if ext_or_mime.contains('/') {
            ImageDataFormat::MimeType(ext_or_mime)
        } else {
            ImageDataFormat::FileExtension(ext_or_mime)
        }
    }
}
impl From<&str> for ImageDataFormat {
    fn from(ext_or_mime: &str) -> Self {
        Txt::from_str(ext_or_mime).into()
    }
}
impl From<PxSize> for ImageDataFormat {
    fn from(bgra8_size: PxSize) -> Self {
        ImageDataFormat::Bgra8 {
            size: bgra8_size,
            ppi: None,
        }
    }
}
impl PartialEq for ImageDataFormat {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::FileExtension(l0), Self::FileExtension(r0)) => l0 == r0,
            (Self::MimeType(l0), Self::MimeType(r0)) => l0 == r0,
            (Self::Bgra8 { size: s0, ppi: p0 }, Self::Bgra8 { size: s1, ppi: p1 }) => s0 == s1 && ppi_key(*p0) == ppi_key(*p1),
            (Self::Unknown, Self::Unknown) => true,
            _ => false,
        }
    }
}
impl Eq for ImageDataFormat {}
impl std::hash::Hash for ImageDataFormat {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);
        match self {
            ImageDataFormat::Bgra8 { size, ppi } => {
                size.hash(state);
                ppi_key(*ppi).hash(state);
            }
            ImageDataFormat::A8 { size } => {
                size.hash(state);
            }
            ImageDataFormat::FileExtension(ext) => ext.hash(state),
            ImageDataFormat::MimeType(mt) => mt.hash(state),
            ImageDataFormat::Unknown => {}
        }
    }
}

fn ppi_key(ppi: Option<ImagePpi>) -> Option<(u16, u16)> {
    ppi.map(|s| ((s.x * 3.0) as u16, (s.y * 3.0) as u16))
}

/// Represents a successfully decoded image.
///
/// See [`Event::ImageLoaded`].
///
/// [`Event::ImageLoaded`]: crate::Event::ImageLoaded
#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ImageLoadedData {
    /// Image ID.
    pub id: ImageId,
    /// Pixel size.
    pub size: PxSize,
    /// Pixel-per-inch metadata.
    pub ppi: Option<ImagePpi>,
    /// If all pixels have an alpha value of 255.
    pub is_opaque: bool,
    /// If the `pixels` are in a single channel (A8).
    pub is_mask: bool,
    /// Reference to the BGRA8 pre-multiplied image pixels or the A8 pixels if `is_mask`.
    pub pixels: IpcBytes,
}
impl ImageLoadedData {
    /// New response.
    pub fn new(id: ImageId, size: PxSize, ppi: Option<ImagePpi>, is_opaque: bool, is_mask: bool, pixels: IpcBytes) -> Self {
        Self {
            id,
            size,
            ppi,
            is_opaque,
            is_mask,
            pixels,
        }
    }
}
impl fmt::Debug for ImageLoadedData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ImageLoadedData")
            .field("id", &self.id)
            .field("size", &self.size)
            .field("ppi", &self.ppi)
            .field("is_opaque", &self.is_opaque)
            .field("is_mask", &self.is_mask)
            .field("pixels", &format_args!("<{} bytes shared memory>", self.pixels.len()))
            .finish()
    }
}
/// Pixels-per-inch of each dimension of an image.
#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ImagePpi {
    /// Pixels-per-inch in the X dimension.
    pub x: f32,
    /// Pixels-per-inch in the Y dimension.
    pub y: f32,
}
impl ImagePpi {
    /// New from x, y.
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// New equal in both dimensions.
    pub const fn splat(xy: f32) -> Self {
        Self::new(xy, xy)
    }

    /// New from pixels-per-centimeter
    pub const fn new_cm(x: f32, y: f32) -> Self {
        Self {
            x: x * Self::CM_TO_INCH,
            y: y * Self::CM_TO_INCH,
        }
    }

    /// Pixels-per-centimeter in the X dimension.
    pub const fn x_cm(self) -> f32 {
        self.x / Self::CM_TO_INCH
    }

    /// Pixels-per-centimeter in the Y dimension.
    pub const fn y_cm(self) -> f32 {
        self.y / Self::CM_TO_INCH
    }

    const CM_TO_INCH: f32 = 2.54;
}
impl Default for ImagePpi {
    /// 96.0
    fn default() -> Self {
        Self::splat(96.0)
    }
}
impl fmt::Debug for ImagePpi {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() || self.x != self.y {
            f.debug_struct("ImagePpi").field("x", &self.x).field("y", &self.y).finish()
        } else {
            write!(f, "{}", self.x)
        }
    }
}

impl From<f32> for ImagePpi {
    fn from(xy: f32) -> Self {
        ImagePpi::splat(xy)
    }
}
impl From<(f32, f32)> for ImagePpi {
    fn from((x, y): (f32, f32)) -> Self {
        ImagePpi::new(x, y)
    }
}

#[cfg(feature = "var")]
zng_var::impl_from_and_into_var! {
    fn from(xy: f32) -> ImagePpi;
    fn from(xy: (f32, f32)) -> ImagePpi;
}
