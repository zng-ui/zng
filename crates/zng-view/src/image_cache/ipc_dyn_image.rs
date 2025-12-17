// like image::DynamicImage, but with IpcBytesMut storage

use image::{error::*, *};
use zng_task::channel::{IpcBytes, IpcBytesMut, IpcBytesMutCast};

pub(crate) enum IpcDynamicImage {
    /// Each pixel in this image is 8-bit Luma
    ImageLuma8(GrayImage),

    /// Each pixel in this image is 8-bit Luma with alpha
    ImageLumaA8(GrayAlphaImage),

    /// Each pixel in this image is 8-bit Rgb
    ImageRgb8(RgbImage),

    /// Each pixel in this image is 8-bit Rgb with alpha
    ImageRgba8(RgbaImage),

    /// Each pixel in this image is 16-bit Luma
    ImageLuma16(Gray16Image),

    /// Each pixel in this image is 16-bit Luma with alpha
    ImageLumaA16(GrayAlpha16Image),

    /// Each pixel in this image is 16-bit Rgb
    ImageRgb16(Rgb16Image),

    /// Each pixel in this image is 16-bit Rgb with alpha
    ImageRgba16(Rgba16Image),

    /// Each pixel in this image is 32-bit float Rgb
    ImageRgb32F(Rgb32FImage),

    /// Each pixel in this image is 32-bit float Rgb with alpha
    ImageRgba32F(Rgba32FImage),
}

/// Sendable Rgb image buffer
pub(crate) type RgbImage = ImageBuffer<Rgb<u8>, IpcBytesMut>;
/// Sendable Rgb + alpha channel image buffer
pub(crate) type RgbaImage = ImageBuffer<Rgba<u8>, IpcBytesMut>;
/// Sendable grayscale image buffer
pub(crate) type GrayImage = ImageBuffer<Luma<u8>, IpcBytesMut>;
/// Sendable grayscale + alpha channel image buffer
pub(crate) type GrayAlphaImage = ImageBuffer<LumaA<u8>, IpcBytesMut>;
/// Sendable 16-bit Rgb image buffer
pub(crate) type Rgb16Image = ImageBuffer<Rgb<u16>, IpcBytesMutCast<u16>>;
/// Sendable 16-bit Rgb + alpha channel image buffer
pub(crate) type Rgba16Image = ImageBuffer<Rgba<u16>, IpcBytesMutCast<u16>>;
/// Sendable 16-bit grayscale image buffer
pub(crate) type Gray16Image = ImageBuffer<Luma<u16>, IpcBytesMutCast<u16>>;
/// Sendable 16-bit grayscale + alpha channel image buffer
pub(crate) type GrayAlpha16Image = ImageBuffer<LumaA<u16>, IpcBytesMutCast<u16>>;

/// An image buffer for 32-bit float RGB pixels,
/// where the backing container is a flattened vector of floats.
pub type Rgb32FImage = ImageBuffer<Rgb<f32>, IpcBytesMutCast<f32>>;

/// An image buffer for 32-bit float RGBA pixels,
/// where the backing container is a flattened vector of floats.
pub type Rgba32FImage = ImageBuffer<Rgba<f32>, IpcBytesMutCast<f32>>;

macro_rules! dynamic_map(
        ($dynimage: expr, $image: pat => $action: expr) => ({
            use IpcDynamicImage::*;
            match $dynimage {
                ImageLuma8($image) => ImageLuma8($action),
                ImageLumaA8($image) => ImageLumaA8($action),
                ImageRgb8($image) => ImageRgb8($action),
                ImageRgba8($image) => ImageRgba8($action),
                ImageLuma16($image) => ImageLuma16($action),
                ImageLumaA16($image) => ImageLumaA16($action),
                ImageRgb16($image) => ImageRgb16($action),
                ImageRgba16($image) => ImageRgba16($action),
                ImageRgb32F($image) => ImageRgb32F($action),
                ImageRgba32F($image) => ImageRgba32F($action),
            }
        });

        ($dynimage: expr, $image:pat_param, $action: expr) => ({
            use IpcDynamicImage::*;
            match $dynimage {
                ImageLuma8($image) => $action,
                ImageLumaA8($image) => $action,
                ImageRgb8($image) => $action,
                ImageRgba8($image) => $action,
                ImageLuma16($image) => $action,
                ImageLumaA16($image) => $action,
                ImageRgb16($image) => $action,
                ImageRgba16($image) => $action,
                ImageRgb32F($image) => $action,
                ImageRgba32F($image) => $action,
            }
        });
);

impl IpcDynamicImage {
    pub fn decode(buf: &[u8], format: ImageFormat, entry: usize) -> image::ImageResult<Self> {
        match format {
            #[cfg(feature = "image_ico")]
            ImageFormat::Ico => return Self::decode_ico(buf, entry),
            #[cfg(feature = "image_tiff")]
            ImageFormat::Tiff => return Self::decode_tiff(buf, entry),
            _ => {}
        }

        let buf = std::io::Cursor::new(buf);

        let mut reader = image::ImageReader::new(buf);
        reader.set_format(format);
        reader.no_limits();

        let decoder = reader.into_decoder()?;

        let (w, h) = decoder.dimensions();
        let color_type = decoder.color_type();

        let mut buf = Self::alloc_buf(decoder.total_bytes())?;
        decoder.read_image(&mut buf[..])?;

        Self::from_decoded(buf, color_type, w, h)
    }

    #[cfg(feature = "image_ico")]
    fn decode_ico(buf: &[u8], entry: usize) -> image::ImageResult<Self> {
        let buf = std::io::Cursor::new(buf);

        let icon = ico::IconDir::read(buf)?;

        let entry = icon.entries()[entry].decode()?;
        let (w, h) = (entry.width(), entry.height());

        let buf = IpcBytesMut::from_vec_blocking(entry.into_rgba_data())?;

        Self::from_decoded(buf, ColorType::Rgba8, w, h)
    }

    #[cfg(feature = "image_tiff")]
    fn decode_tiff(buf: &[u8], entry: usize) -> image::ImageResult<Self> {
        fn e(e: tiff::TiffError) -> image::ImageError {
            // https://docs.rs/image/0.25.9/src/image/codecs/tiff.rs.html#190
            match e {
                tiff::TiffError::IoError(err) => image::ImageError::IoError(err),
                err @ (tiff::TiffError::FormatError(_) | tiff::TiffError::IntSizeError | tiff::TiffError::UsageError(_)) => {
                    image::ImageError::Decoding(DecodingError::new(ImageFormat::Tiff.into(), err))
                }
                tiff::TiffError::UnsupportedError(desc) => image::ImageError::Unsupported(UnsupportedError::from_format_and_kind(
                    ImageFormat::Tiff.into(),
                    UnsupportedErrorKind::GenericFeature(desc.to_string()),
                )),
                tiff::TiffError::LimitsExceeded => image::ImageError::Limits(LimitError::from_kind(LimitErrorKind::InsufficientMemory)),
            }
        }

        let buf = std::io::Cursor::new(buf);

        let mut tiff = tiff::decoder::Decoder::new(buf).map_err(e)?;
        tiff.seek_to_image(entry).map_err(e)?;

        // https://docs.rs/image/0.25.9/src/image/codecs/tiff.rs.html#182
        fn err_unknown_color_type(value: u8) -> ImageError {
            ImageError::Unsupported(UnsupportedError::from_format_and_kind(
                ImageFormat::Tiff.into(),
                UnsupportedErrorKind::Color(ExtendedColorType::Unknown(value)),
            ))
        }
        // https://docs.rs/image/0.25.9/src/image/codecs/tiff.rs.html#74
        let color_type = match tiff.colortype().map_err(e)? {
            tiff::ColorType::Gray(1) => ColorType::L8,
            tiff::ColorType::Gray(8) => ColorType::L8,
            tiff::ColorType::Gray(16) => ColorType::L16,
            tiff::ColorType::GrayA(8) => ColorType::La8,
            tiff::ColorType::GrayA(16) => ColorType::La16,
            tiff::ColorType::RGB(8) => ColorType::Rgb8,
            tiff::ColorType::RGB(16) => ColorType::Rgb16,
            tiff::ColorType::RGBA(8) => ColorType::Rgba8,
            tiff::ColorType::RGBA(16) => ColorType::Rgba16,
            tiff::ColorType::CMYK(8) => ColorType::Rgb8,
            tiff::ColorType::CMYK(16) => ColorType::Rgb16,
            tiff::ColorType::RGB(32) => ColorType::Rgb32F,
            tiff::ColorType::RGBA(32) => ColorType::Rgba32F,

            tiff::ColorType::Palette(n) | tiff::ColorType::Gray(n) => {
                return Err(err_unknown_color_type(n))
            }
            tiff::ColorType::GrayA(n) => return Err(err_unknown_color_type(n.saturating_mul(2))),
            tiff::ColorType::RGB(n) => return Err(err_unknown_color_type(n.saturating_mul(3))),
            tiff::ColorType::YCbCr(n) => return Err(err_unknown_color_type(n.saturating_mul(3))),
            tiff::ColorType::RGBA(n) | tiff::ColorType::CMYK(n) => {
                return Err(err_unknown_color_type(n.saturating_mul(4)))
            }
            tiff::ColorType::Multiband {
                bit_depth,
                num_samples,
            } => {
                return Err(err_unknown_color_type(
                    bit_depth.saturating_mul(num_samples.min(255) as u8),
                ))
            }
            _ => return Err(err_unknown_color_type(0)),
        };

        let (w, h) = tiff.dimensions().map_err(e)?;

        let total_bytes = tiff.image_buffer_layout().map_err(e)?.len;
        let mut buf = Self::alloc_buf(total_bytes as u64)?;

        tiff.read_image_bytes(&mut buf[..]).map_err(e)?;
        
        Self::from_decoded(buf, color_type, w, h)
    }

    fn from_decoded(buf: IpcBytesMut, color_type: image::ColorType, w: u32, h: u32) -> image::ImageResult<Self> {
        // copied from image-0.25.9\src\images\dynimage.rs
        match color_type {
            ColorType::Rgb8 => ImageBuffer::from_raw(w, h, buf).map(IpcDynamicImage::ImageRgb8),
            ColorType::Rgba8 => ImageBuffer::from_raw(w, h, buf).map(IpcDynamicImage::ImageRgba8),
            ColorType::L8 => ImageBuffer::from_raw(w, h, buf).map(IpcDynamicImage::ImageLuma8),
            ColorType::La8 => ImageBuffer::from_raw(w, h, buf).map(IpcDynamicImage::ImageLumaA8),
            ColorType::Rgb16 => ImageBuffer::from_raw(w, h, buf.cast()).map(IpcDynamicImage::ImageRgb16),
            ColorType::Rgba16 => ImageBuffer::from_raw(w, h, buf.cast()).map(IpcDynamicImage::ImageRgba16),
            ColorType::Rgb32F => ImageBuffer::from_raw(w, h, buf.cast()).map(IpcDynamicImage::ImageRgb32F),
            ColorType::Rgba32F => ImageBuffer::from_raw(w, h, buf.cast()).map(IpcDynamicImage::ImageRgba32F),
            ColorType::L16 => ImageBuffer::from_raw(w, h, buf.cast()).map(IpcDynamicImage::ImageLuma16),
            ColorType::La16 => ImageBuffer::from_raw(w, h, buf.cast()).map(IpcDynamicImage::ImageLumaA16),
            _ => unreachable!(),
        }
        .ok_or_else(|| ImageError::Parameter(ParameterError::from_kind(ParameterErrorKind::DimensionMismatch)))
    }

    fn alloc_buf(len: u64) -> image::ImageResult<IpcBytesMut> {
        if len > usize::MAX as u64 {
            return Err(ImageError::Limits(LimitError::from_kind(LimitErrorKind::InsufficientMemory)));
        }

        let buf = IpcBytes::new_mut_blocking(len as usize)?;
        Ok(buf)
    }

    pub fn dimensions(&self) -> (u32, u32) {
        dynamic_map!(*self, ref p, p.dimensions())
    }
}
