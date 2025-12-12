//! Image types.

use std::fmt;

use serde::{Deserialize, Serialize};
use zng_task::channel::IpcBytes;
use zng_txt::Txt;

use zng_unit::{Px, PxDensity2d, PxSize};

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
    /// Bytes layout depends on the `format`, data structure is [`IpcBytes`] or [`IpcReceiver<IpcBytes>`] in the view API.
    ///
    /// [`IpcReceiver<IpcBytes>`]: crate::IpcReceiver
    pub data: D,
    /// Maximum allowed decoded size.
    ///
    /// View-process will avoid decoding and return an error if the image decoded to BGRA (4 bytes) exceeds this size.
    /// This limit applies to the image before the `downscale`.
    pub max_decoded_len: u64,
    /// A size constraints to apply after the image is decoded. The image is resized to fit or fill the given size.
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
    /// Image is downscaled so that at least one dimension fits inside the size. The image is not clipped.
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
        let (new_size, fill) = match self {
            ImageDownscale::Fill(s) => (s, true),
            ImageDownscale::Fit(s) => (s, false),
        };
        let source_size = source_size.cast::<f64>();
        let new_size = new_size.cast::<f64>();

        let w_ratio = new_size.width / source_size.width;
        let h_ratio = new_size.height / source_size.height;

        let ratio = if fill {
            f64::max(w_ratio, h_ratio)
        } else {
            f64::min(w_ratio, h_ratio)
        };

        let nw = u64::max((source_size.width * ratio).round() as _, 1);
        let nh = u64::max((source_size.height * ratio).round() as _, 1);

        const MAX: u64 = Px::MAX.0 as _;

        if nw > MAX {
            let ratio = MAX as f64 / source_size.width;
            (Px::MAX, Px(i32::max((source_size.height * ratio).round() as _, 1))).into()
        } else if nh > MAX {
            let ratio = MAX as f64 / source_size.height;
            (Px(i32::max((source_size.width * ratio).round() as _, 1)), Px::MAX).into()
        } else {
            (Px(nw as _), Px(nh as _)).into()
        }
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
        /// Pixel density of the image.
        density: Option<PxDensity2d>,
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
            density: None,
        }
    }
}
impl PartialEq for ImageDataFormat {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::FileExtension(l0), Self::FileExtension(r0)) => l0 == r0,
            (Self::MimeType(l0), Self::MimeType(r0)) => l0 == r0,
            (Self::Bgra8 { size: s0, density: p0 }, Self::Bgra8 { size: s1, density: p1 }) => {
                s0 == s1 && density_key(*p0) == density_key(*p1)
            }
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
            ImageDataFormat::Bgra8 { size, density } => {
                size.hash(state);
                density_key(*density).hash(state);
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

fn density_key(density: Option<PxDensity2d>) -> Option<(u16, u16)> {
    density.map(|s| ((s.width.ppcm() * 3.0) as u16, (s.height.ppcm() * 3.0) as u16))
}

/// Represents decoded header metadata about an image.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ImageMetadata {
    /// Image ID.
    pub id: ImageId,
    /// Pixel size.
    pub size: PxSize,
    /// Pixel density metadata.
    pub density: Option<PxDensity2d>,
    /// If the `pixels` are in a single channel (A8).
    pub is_mask: bool,
}
impl ImageMetadata {
    /// New.
    pub fn new(id: ImageId, size: PxSize, is_mask: bool) -> Self {
        Self {
            id,
            size,
            density: None,
            is_mask,
        }
    }
}

/// Represents a partial or fully decoded image.
///
/// See [`Event::ImageDecoded`] and [`ImagePartiallyDecoded`] for more details.
///
/// [`Event::ImageDecoded`]: crate::Event::ImageDecoded
/// [`ImagePartiallyDecoded`]: crate::Event::ImagePartiallyDecoded
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ImageDecoded {
    /// Image metadata.
    pub meta: ImageMetadata,
    /// Decoded pixels.
    ///
    /// Is BGRA8 pre-multiplied if `!is_mask` or is `A8` if `is_mask`.
    pub pixels: IpcBytes,
    /// If all pixels have an alpha value of 255.
    pub is_opaque: bool,
}
impl ImageDecoded {
    /// New.
    pub fn new(meta: ImageMetadata, pixels: IpcBytes, is_opaque: bool) -> Self {
        Self { meta, pixels, is_opaque }
    }
}

/// Represents a successfully decoded image.
///
/// See [`Event::ImageLoaded`].
///
/// [`Event::ImageLoaded`]: crate::Event::ImageLoaded
#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ImageLoadedData {
    // TODO(breaking) remove, replace with ImageData
    /// Image ID.
    pub id: ImageId,
    /// Pixel size.
    pub size: PxSize,
    /// Pixel density metadata.
    pub density: Option<PxDensity2d>,
    /// If all pixels have an alpha value of 255.
    pub is_opaque: bool,
    /// If the `pixels` are in a single channel (A8).
    pub is_mask: bool,
    /// Reference to the BGRA8 pre-multiplied image pixels or the A8 pixels if `is_mask`.
    pub pixels: IpcBytes,
}
impl ImageLoadedData {
    /// New response.
    pub fn new(id: ImageId, size: PxSize, density: Option<PxDensity2d>, is_opaque: bool, is_mask: bool, pixels: IpcBytes) -> Self {
        Self {
            id,
            size,
            density,
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
            .field("density", &self.density)
            .field("is_opaque", &self.is_opaque)
            .field("is_mask", &self.is_mask)
            .field("pixels", &format_args!("<{} bytes shared memory>", self.pixels.len()))
            .finish()
    }
}
impl From<ImageDecoded> for ImageLoadedData {
    fn from(value: ImageDecoded) -> Self {
        ImageLoadedData {
            id: value.meta.id,
            size: value.meta.size,
            density: value.meta.density,
            is_opaque: value.is_opaque,
            is_mask: value.meta.is_mask,
            pixels: value.pixels,
        }
    }
}

/// Represents an image codec capability.
///
/// This type will be used in the next breaking release of the view API.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ImageFormat {
    /// Display name of the format.
    pub display_name: Txt,

    /// Media types (MIME) associated with the format.
    ///
    /// Lowercase, without `"image/"` prefix, comma separated if there is more than one.
    pub media_type_suffixes: Txt,

    /// Common file extensions associated with the format.
    ///
    /// Lowercase, without dot, comma separated if there is more than one.
    pub file_extensions: Txt,

    /// If the view-process implementation can encode images in this format.
    ///
    /// Note that the view-process can always decode formats.
    pub can_encode: bool,
}
impl ImageFormat {
    /// From static str.
    ///
    /// # Panics
    ///
    /// Panics if `media_type_suffixes` not ASCII.
    pub const fn from_static(
        display_name: &'static str,
        media_type_suffixes: &'static str,
        file_extensions: &'static str,
        can_encode: bool,
    ) -> Self {
        assert!(media_type_suffixes.is_ascii());
        Self {
            display_name: Txt::from_static(display_name),
            media_type_suffixes: Txt::from_static(media_type_suffixes),
            file_extensions: Txt::from_static(file_extensions),
            can_encode,
        }
    }

    /// Iterate over media type suffixes.
    pub fn media_type_suffixes_iter(&self) -> impl Iterator<Item = &str> {
        self.media_type_suffixes.split(',').map(|e| e.trim())
    }

    /// Iterate over full media types, with `"image/"` prefix.
    pub fn media_types(&self) -> impl Iterator<Item = Txt> {
        self.media_type_suffixes_iter().map(Txt::from_str)
    }

    /// Iterate over extensions.
    pub fn file_extensions_iter(&self) -> impl Iterator<Item = &str> {
        self.file_extensions.split(',').map(|e| e.trim())
    }

    /// Checks if `f` matches any of the mime types or any of the file extensions.
    ///
    /// File extensions comparison ignores dot and ASCII case.
    pub fn matches(&self, f: &str) -> bool {
        let f = f.strip_prefix('.').unwrap_or(f);
        let f = f.strip_prefix("image/").unwrap_or(f);
        self.media_type_suffixes_iter().any(|e| e.eq_ignore_ascii_case(f)) || self.file_extensions_iter().any(|e| e.eq_ignore_ascii_case(f))
    }
}
