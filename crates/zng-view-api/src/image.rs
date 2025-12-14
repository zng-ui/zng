//! Image types.

use std::fmt;

use bitflags::bitflags;
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

bitflags! {
    /// Defines what images are decoded from multi image containers.
    ///
    /// These flags represent the different [`ImageEntryKind`].
    #[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
    pub struct ImageEntriesMode: u8 {
        /// Decodes all pages.
        const PAGES = 0b0001;
        /// Decodes reduced alternates of the selected pages.
        const REDUCED = 0b0010;
        /// Decodes only the first page, or the page explicitly marked as primary by the container format.
        ///
        /// Note that this is 0, empty.
        const PRIMARY = 0;

        /// Decodes any other images that are not considered pages nor reduced alternates.
        const OTHER = 0b1000;
    }
}
#[cfg(feature = "var")]
zng_var::impl_from_and_into_var! {
    fn from(kind: ImageEntryKind) -> ImageEntriesMode {
        match kind {
            ImageEntryKind::Page => ImageEntriesMode::PAGES,
            ImageEntryKind::Reduced { synthetic } => {
                if synthetic {
                    ImageEntriesMode::PRIMARY | ImageEntriesMode::REDUCED
                } else {
                    ImageEntriesMode::REDUCED
                }
            }
            ImageEntryKind::Other { .. } => ImageEntriesMode::OTHER,
        }
    }
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

    /// Deprecated.
    #[deprecated = "use `downscale2`"]
    #[allow(deprecated)]
    pub downscale: Option<ImageDownscale>,
    /// A size constraints to apply after the image is decoded. The image is resized to fit or fill the given size.
    /// Optionally generate multiple reduced entries.
    ///
    /// If the image contains multiple images selects the nearest *reduced alternate* that can be downscaled.
    ///
    /// If `entries` requests `REDUCED` only the alternates smaller than the requested downscale are included.
    pub downscale2: Option<ImageDownscaleMode>,

    /// Convert or decode the image into a single channel mask (R8).
    pub mask: Option<ImageMaskMode>,

    /// Defines what images are decoded from multi image containers.
    pub entries: ImageEntriesMode,
}
impl<D> ImageRequest<D> {
    /// New request.
    #[allow(deprecated)]
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
            downscale2: downscale.map(|d| match d {
                ImageDownscale::Fit(s) => ImageDownscaleMode::Fit(s),
                ImageDownscale::Fill(s) => ImageDownscaleMode::Fill(s),
            }),
            mask,
            entries: ImageEntriesMode::PRIMARY,
        }
    }
}

/// Defines how an image is downscaled after decoding.
///
/// The image aspect ratio is preserved in all modes. The image is never upscaled.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ImageDownscaleMode {
    /// Image is downscaled so that both dimensions fit inside the size.
    Fit(PxSize),
    /// Image is downscaled so that at least one dimension fits inside the size. The image is not clipped.
    Fill(PxSize),
    /// Image is downscaled to `Fit`, but can be a slightly larger size if downscaling to the larger size is faster.
    LooseFit(PxSize),
    /// Image is downscaled to `Fill`, but can be a slightly larger size if downscaling to the larger size is faster.
    LooseFill(PxSize),
    /// Generate synthetic [`ImageEntryKind::Reduced`] entries each half the size of the image until the sample that is
    /// nearest `min_size` and greater or equal to it.
    MipMap {
        /// Minimum sample size.
        min_size: PxSize,
        /// `LooseFill` maximum size.
        max_size: PxSize,
    },
    /// Applies the first to image and generate synthetic [`ImageEntryKind::Reduced`] for the others.
    Entries(Vec<ImageDownscaleMode>),
}
impl From<PxSize> for ImageDownscaleMode {
    /// Fit
    fn from(fit: PxSize) -> Self {
        ImageDownscaleMode::Fit(fit)
    }
}
impl From<Px> for ImageDownscaleMode {
    /// Fit splat
    fn from(fit: Px) -> Self {
        ImageDownscaleMode::Fit(PxSize::splat(fit))
    }
}
#[cfg(feature = "var")]
zng_var::impl_from_and_into_var! {
    fn from(fit: PxSize) -> ImageDownscaleMode;
    fn from(fit: Px) -> ImageDownscaleMode;
    fn from(some: ImageDownscaleMode) -> Option<ImageDownscaleMode>;
}
impl ImageDownscaleMode {
    /// Append entry downscale request.
    pub fn with_entry(self, other: impl Into<ImageDownscaleMode>) -> Self {
        self.with_impl(other.into())
    }
    fn with_impl(self, other: Self) -> Self {
        let mut v = match self {
            Self::Entries(e) => e,
            s => vec![s],
        };
        match other {
            Self::Entries(o) => v.extend(o),
            o => v.push(o),
        }
        Self::Entries(v)
    }

    /// Compute the expected final sizes if the downscale is applied on an image of `source_size`.
    /// 
    /// The values are `(loose, target_size)` where if loose is `true` any approximation of the size 
    /// that is larger or equal to it is enough.
    pub fn target_sizes(&self, source_size: PxSize, sizes: &mut Vec<(bool, PxSize)>) {
        let (new_size, fill, loose) = match self {
            Self::Fit(s) => (*s, false, false),
            Self::Fill(s) => (*s, true, false),
            Self::LooseFit(s) => (*s, false, true),
            Self::LooseFill(s) => (*s, true, true),
            Self::MipMap { min_size, max_size } => {
                let mut max = fit_fill(source_size, *max_size, true);
                if sizes.is_empty() {
                    sizes.push((true, max));
                    max /= Px(2);
                }
                while max.width >= min_size.width && max.height >= min_size.height {
                    sizes.push((true, max));
                    max /= Px(2);
                }
                return;
            },
            Self::Entries(e) => {
                for e in e {
                    e.target_sizes(source_size, sizes);
                }
                return;
            }
        };

        sizes.push((loose, fit_fill(source_size, new_size, fill)));
    }
}

fn fit_fill(source_size: PxSize, new_size: PxSize, fill: bool) -> PxSize {
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
        (Px::MAX, Px(i32::max((source_size.height * ratio).round() as _, 1)))
    } else if nh > MAX {
        let ratio = MAX as f64 / source_size.height;
        (Px(i32::max((source_size.width * ratio).round() as _, 1)), Px::MAX)
    } else {
        (Px(nw as _), Px(nh as _))
    }.into()
    
}

/// Deprecated
#[deprecated = "use `ImageDownscaleMode`"]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ImageDownscale {
    // TODO(breaking) remove
    /// Image is downscaled so that both dimensions fit inside the size.
    Fit(PxSize),
    /// Image is downscaled so that at least one dimension fits inside the size. The image is not clipped.
    Fill(PxSize),
}
#[allow(deprecated)]
mod _old_impl {
    use super::*;

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
            fit_fill(source_size, new_size, fill)
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
        // TODO(breaking) add original_color_type
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
    /// Image color type before it was converted to BGRA8 or A8.
    pub original_color_type: ColorType,

    /// Kind of image container entry this image was decoded from.
    ///
    /// If the kind is `Page` and the `entry_parent` is `None` the image represents the
    /// first page in the container and will list any other pages in `entries`.
    ///
    /// If the kind is `Page` and the `entry_parent` is set, the parent will list the pages,
    /// this image will only list other entries directly related to it.
    ///
    /// If the kind is `Reduced` the `entry_parent` must be set.
    pub entry_kind: ImageEntryKind,

    /// Image this one belongs too.
    ///
    /// This image will be listed on the parent `entries`.
    pub entry_parent: Option<ImageId>,

    /// Other images from the same container.
    ///
    /// The other images will reference this image back as a parent in `entry_parent`.
    ///
    /// The other images are always referenced first here before the first decoded event targeting each.
    pub entries: Vec<ImageId>,
}
impl ImageMetadata {
    /// New.
    pub fn new(id: ImageId, size: PxSize, is_mask: bool, original_color_type: ColorType) -> Self {
        Self {
            id,
            size,
            density: None,
            is_mask,
            original_color_type,
            entry_kind: ImageEntryKind::Page,
            entry_parent: None,
            entries: vec![],
        }
    }
}

/// Kind of image container entry an image was decoded from.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[non_exhaustive]
pub enum ImageEntryKind {
    /// Full sized image in the container.
    Page,
    /// Reduced resolution alternate of the other image.
    ///
    /// Can be mip levels, a thumbnail or a smaller symbolic alternative designed to be more readable at smaller scale.
    Reduced {
        /// If reduced image was generated, not part of the image container file.
        synthetic: bool,
    },
    /// Custom kind identifier.
    Other {
        /// Custom identifier.
        ///
        /// This is an implementation specific value that can be parsed.
        kind: Txt,
    },
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

/// Basic info about a color model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ColorType {
    /// Color model name.
    pub name: Txt,
    /// Bits per channel.
    pub bits: u8,
    /// Channels per pixel.
    pub channels: u8,
}
impl ColorType {
    /// New.
    pub const fn new(name: Txt, bits: u8, channels: u8) -> Self {
        Self { name, bits, channels }
    }

    /// Bit length of a pixel.
    pub fn bits_per_pixel(&self) -> u16 {
        self.bits as u16 * self.channels as u16
    }

    /// Byte length of a pixel.
    pub fn bytes_per_pixel(&self) -> u16 {
        self.bits_per_pixel() / 8
    }
}
impl ColorType {
    /// BGRA8
    pub const BGRA8: ColorType = ColorType::new(Txt::from_static("BGRA8"), 8, 4);
    /// RGBA8
    pub const RGBA8: ColorType = ColorType::new(Txt::from_static("RGBA8"), 8, 4);

    /// A8
    pub const A8: ColorType = ColorType::new(Txt::from_static("A8"), 8, 4);
}
