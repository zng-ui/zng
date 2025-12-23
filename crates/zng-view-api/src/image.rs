//! Image types.

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
    #[derive(Copy, Debug, PartialEq, Eq, Clone, Hash, Serialize, Deserialize)]
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
            ImageEntryKind::Reduced { .. } => ImageEntriesMode::REDUCED,
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

    /// A size constraints to apply after the image is decoded. The image is resized to fit or fill the given size.
    /// Optionally generate multiple reduced entries.
    ///
    /// If the image contains multiple images selects the nearest *reduced alternate* that can be downscaled.
    ///
    /// If `entries` requests `REDUCED` only the alternates smaller than the requested downscale are included.
    pub downscale: Option<ImageDownscaleMode>,

    /// Convert or decode the image into a single channel mask (R8).
    pub mask: Option<ImageMaskMode>,

    /// Defines what images are decoded from multi image containers.
    pub entries: ImageEntriesMode,

    /// Image is an entry (or subtree) of this other image.
    pub parent: Option<ImageEntryMetadata>,
}
impl<D> ImageRequest<D> {
    /// New request.
    pub fn new(
        format: ImageDataFormat,
        data: D,
        max_decoded_len: u64,
        downscale: Option<ImageDownscaleMode>,
        mask: Option<ImageMaskMode>,
    ) -> Self {
        Self {
            format,
            data,
            max_decoded_len,
            downscale,
            mask,
            entries: ImageEntriesMode::PRIMARY,
            parent: None,
        }
    }
}

/// Defines how an image is downscaled after decoding.
///
/// The image aspect ratio is preserved in all modes. The image is never upscaled. If the image container
/// contains reduced alternative the nearest to the requested size is used as source.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ImageDownscaleMode {
    /// Image is downscaled so that both dimensions fit inside the size.
    Fit(PxSize),
    /// Image is downscaled so that at least one dimension fits inside the size. The image is not clipped.
    Fill(PxSize),
    /// Generate synthetic [`ImageEntryKind::Reduced`] entries each half the size of the image until the sample that is
    /// nearest `min_size` and greater or equal to it. If the image container already has alternates that are equal to
    /// or *near* a mip size that size is used instead.
    MipMap {
        /// Minimum sample size.
        min_size: PxSize,
        /// Maximum `Fill` size.
        max_size: PxSize,
    },
    /// Generate multiple synthetic [`ImageEntryKind::Reduced`] entries. The entry sizes are sorted by largest first,
    /// if the image full size already fits in the largest downscale requested the full image is returned and any
    /// downscale actually smaller becomes a synthetic entry. If the image is larger than all requested sizes it is downscaled as well.
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
    /// Default mipmap min/max when the objective of the mipmap is optimizing dynamically resizing massive images.
    pub fn mip_map() -> Self {
        Self::MipMap {
            min_size: PxSize::splat(Px(512)),
            max_size: PxSize::splat(Px::MAX),
        }
    }

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

    /// Get downscale sizes that need to be generated.
    ///
    /// The `page_size` is the image full size, the `reduced_sizes` are
    /// sizes of reduced alternates that are already provided by the image  container.
    ///
    /// Returns the downscale for the image full size, if needed and a list of reduced entries that must be synthesized,
    /// sorted largest to smallest.
    pub fn sizes(&self, page_size: PxSize, reduced_sizes: &[PxSize]) -> (Option<PxSize>, Vec<PxSize>) {
        match self {
            ImageDownscaleMode::Fit(s) => (fit_fill(page_size, *s, false), vec![]),
            ImageDownscaleMode::Fill(s) => (fit_fill(page_size, *s, true), vec![]),
            ImageDownscaleMode::MipMap { min_size, max_size } => Self::collect_mip_map(page_size, reduced_sizes, &[], *min_size, *max_size),
            ImageDownscaleMode::Entries(modes) => {
                let mut include_full_size = false;
                let mut sizes = vec![];
                let mut mip_map = None;
                for m in modes {
                    m.collect_entries(page_size, &mut sizes, &mut mip_map, &mut include_full_size);
                }
                if let Some([min_size, max_size]) = mip_map {
                    let (first, mips) = Self::collect_mip_map(page_size, reduced_sizes, &sizes, min_size, max_size);
                    include_full_size |= first.is_some();
                    sizes.extend(first);
                    sizes.extend(mips);
                }

                sizes.sort_by_key(|s| s.width.0 * s.height.0);
                sizes.dedup();

                let full_downscale = if include_full_size { None } else { sizes.pop() };
                sizes.reverse();

                (full_downscale, sizes)
            }
        }
    }

    fn collect_mip_map(
        page_size: PxSize,
        reduced_sizes: &[PxSize],
        entry_sizes: &[PxSize],
        min_size: PxSize,
        max_size: PxSize,
    ) -> (Option<PxSize>, Vec<PxSize>) {
        let page_downscale = fit_fill(page_size, max_size, true);
        let mut size = page_downscale.unwrap_or(page_size) / Px(2);
        let mut entries = vec![];
        while min_size.width >= size.width && min_size.height >= size.height {
            if let Some(entry) = fit_fill(page_size, size, true)
                && !reduced_sizes.iter().any(|s| Self::near(entry, *s))
                && !entry_sizes.iter().any(|s| Self::near(entry, *s))
            {
                entries.push(entry);
            }
            size /= Px(2);
        }
        (page_downscale, entries)
    }
    fn near(candidate: PxSize, existing: PxSize) -> bool {
        let dist = (candidate - existing).abs();
        dist.width < Px(10) && dist.height <= Px(10)
    }

    fn collect_entries(&self, page_size: PxSize, sizes: &mut Vec<PxSize>, mip_map: &mut Option<[PxSize; 2]>, include_full_size: &mut bool) {
        match self {
            ImageDownscaleMode::Fit(s) => match fit_fill(page_size, *s, false) {
                Some(s) => sizes.push(s),
                None => *include_full_size = true,
            },
            ImageDownscaleMode::Fill(s) => match fit_fill(page_size, *s, true) {
                Some(s) => sizes.push(s),
                None => *include_full_size = true,
            },
            ImageDownscaleMode::MipMap { min_size, max_size } => {
                *include_full_size = true;
                if let Some([min, max]) = mip_map {
                    *min = min.min(*min_size);
                    *max = max.min(*min_size);
                } else {
                    *mip_map = Some([*min_size, *max_size]);
                }
            }
            ImageDownscaleMode::Entries(modes) => {
                for m in modes {
                    m.collect_entries(page_size, sizes, mip_map, include_full_size);
                }
            }
        }
    }
}

fn fit_fill(source_size: PxSize, new_size: PxSize, fill: bool) -> Option<PxSize> {
    let source_size = source_size.cast::<f64>();
    let new_size = new_size.cast::<f64>();

    let w_ratio = new_size.width / source_size.width;
    let h_ratio = new_size.height / source_size.height;

    let ratio = if fill {
        f64::max(w_ratio, h_ratio)
    } else {
        f64::min(w_ratio, h_ratio)
    };

    if ratio >= 1.0 {
        return None;
    }

    let nw = u64::max((source_size.width * ratio).round() as _, 1);
    let nh = u64::max((source_size.height * ratio).round() as _, 1);

    const MAX: u64 = Px::MAX.0 as _;

    let r = if nw > MAX {
        let ratio = MAX as f64 / source_size.width;
        (Px::MAX, Px(i32::max((source_size.height * ratio).round() as _, 1)))
    } else if nh > MAX {
        let ratio = MAX as f64 / source_size.height;
        (Px(i32::max((source_size.width * ratio).round() as _, 1)), Px::MAX)
    } else {
        (Px(nw as _), Px(nh as _))
    }
    .into();

    Some(r)
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
        /// Original color type of the image.
        original_color_type: ColorType,
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
            original_color_type: ColorType::BGRA8,
        }
    }
}
impl PartialEq for ImageDataFormat {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::FileExtension(l0), Self::FileExtension(r0)) => l0 == r0,
            (Self::MimeType(l0), Self::MimeType(r0)) => l0 == r0,
            (
                Self::Bgra8 {
                    size: s0,
                    density: p0,
                    original_color_type: oc0,
                },
                Self::Bgra8 {
                    size: s1,
                    density: p1,
                    original_color_type: oc1,
                },
            ) => s0 == s1 && density_key(*p0) == density_key(*p1) && oc0 == oc1,
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
            ImageDataFormat::Bgra8 {
                size,
                density,
                original_color_type,
            } => {
                size.hash(state);
                density_key(*density).hash(state);
                original_color_type.hash(state)
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
pub struct ImageEntryMetadata {
    /// Image this one belongs too.
    ///
    /// The view-process always sends the parent image metadata first, so this id should be known by the app-process.
    pub parent: ImageId,
    /// Sort index of the image in the list of entries.
    pub index: usize,
    /// Kind of entry.
    pub kind: ImageEntryKind,
}
impl ImageEntryMetadata {
    /// New.
    pub fn new(parent: ImageId, index: usize, kind: ImageEntryKind) -> Self {
        Self { parent, index, kind }
    }
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
    /// Extra metadata if this image is an entry in another image.
    ///
    /// When this is `None` the is the first [`ImageEntryKind::Page`] in the container, usually the only page.
    pub parent: Option<ImageEntryMetadata>,
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
            parent: None,
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
impl ImageEntryKind {
    fn discriminant(&self) -> u8 {
        match self {
            ImageEntryKind::Page => 0,
            ImageEntryKind::Reduced { .. } => 1,
            ImageEntryKind::Other { .. } => 2,
        }
    }
}
impl std::cmp::Ord for ImageEntryKind {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.discriminant().cmp(&other.discriminant())
    }
}
impl std::cmp::PartialOrd for ImageEntryKind {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
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

bitflags! {
    /// Capabilities of an [`ImageFormat`] implementation.
    ///
    /// Note that `DECODE` capability is omitted because the view-process can always decode formats.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub struct ImageFormatCapability: u8 {
        /// View-process can encode images in this format.
        const ENCODE = 0b_0000_0001;
        /// View-process can decode multiple containers of the format with multiple image entries.
        const DECODE_ENTRIES = 0b_0000_0010;
        /// View-process can encode multiple images into a single container of the format.
        const ENCODE_ENTRIES = 0b_0000_0101;
        /// View-process can decode pixels as they are received.
        ///
        /// Note that the view-process can always handle progressive data by accumulating it and then decoding.
        /// The decoder can also decode the metadata before receiving all data, that does not count as progressive decoding either.
        const DECODE_PROGRESSIVE = 0b_0000_1000;
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

    /// Capabilities of this format.
    pub capabilities: ImageFormatCapability,
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
        capabilities: ImageFormatCapability,
    ) -> Self {
        assert!(media_type_suffixes.is_ascii());
        Self {
            display_name: Txt::from_static(display_name),
            media_type_suffixes: Txt::from_static(media_type_suffixes),
            file_extensions: Txt::from_static(file_extensions),
            capabilities,
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
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
