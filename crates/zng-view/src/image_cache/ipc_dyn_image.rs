// like image::DynamicImage, but with IpcBytesMut storage

use std::io::{BufRead, Seek};

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
    pub fn decode<R: BufRead + Seek>(reader: ImageReader<R>, entry: usize) -> image::ImageResult<Self> {
        let decoder = reader.into_decoder()?;
        let (w, h) = decoder.dimensions();
        let color_type = decoder.color_type();

        // copied from image-0.25.9\src\images\dynimage.rs
        match color_type {
            ColorType::Rgb8 => {
                let buf = decoder_to_ipc(decoder)?;
                ImageBuffer::from_raw(w, h, buf).map(IpcDynamicImage::ImageRgb8)
            }

            ColorType::Rgba8 => {
                let buf = decoder_to_ipc(decoder)?;
                ImageBuffer::from_raw(w, h, buf).map(IpcDynamicImage::ImageRgba8)
            }

            ColorType::L8 => {
                let buf = decoder_to_ipc(decoder)?;
                ImageBuffer::from_raw(w, h, buf).map(IpcDynamicImage::ImageLuma8)
            }

            ColorType::La8 => {
                let buf = decoder_to_ipc(decoder)?;
                ImageBuffer::from_raw(w, h, buf).map(IpcDynamicImage::ImageLumaA8)
            }

            ColorType::Rgb16 => {
                let buf = decoder_to_ipc(decoder)?.cast();
                ImageBuffer::from_raw(w, h, buf).map(IpcDynamicImage::ImageRgb16)
            }

            ColorType::Rgba16 => {
                let buf = decoder_to_ipc(decoder)?.cast();
                ImageBuffer::from_raw(w, h, buf).map(IpcDynamicImage::ImageRgba16)
            }

            ColorType::Rgb32F => {
                let buf = decoder_to_ipc(decoder)?.cast();
                ImageBuffer::from_raw(w, h, buf).map(IpcDynamicImage::ImageRgb32F)
            }

            ColorType::Rgba32F => {
                let buf = decoder_to_ipc(decoder)?.cast();
                ImageBuffer::from_raw(w, h, buf).map(IpcDynamicImage::ImageRgba32F)
            }

            ColorType::L16 => {
                let buf = decoder_to_ipc(decoder)?.cast();
                ImageBuffer::from_raw(w, h, buf).map(IpcDynamicImage::ImageLuma16)
            }

            ColorType::La16 => {
                let buf = decoder_to_ipc(decoder)?.cast();
                ImageBuffer::from_raw(w, h, buf).map(IpcDynamicImage::ImageLumaA16)
            }
            _ => unreachable!(),
        }
        .ok_or_else(|| ImageError::Parameter(ParameterError::from_kind(ParameterErrorKind::DimensionMismatch)))
    }

    pub fn dimensions(&self) -> (u32, u32) {
        dynamic_map!(*self, ref p, p.dimensions())
    }
}

fn decoder_to_ipc(decoder: impl image::ImageDecoder) -> ImageResult<IpcBytesMut> {
    let total_bytes = decoder.total_bytes();
    if total_bytes > usize::MAX as u64 {
        return Err(ImageError::Limits(LimitError::from_kind(LimitErrorKind::InsufficientMemory)));
    }

    let mut buf = IpcBytes::new_mut_blocking(total_bytes as usize)?;
    decoder.read_image(&mut buf[..])?;
    Ok(buf)
}
