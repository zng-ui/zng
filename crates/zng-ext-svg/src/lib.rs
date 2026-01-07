#![doc(html_favicon_url = "https://zng-ui.github.io/res/zng-logo-icon.png")]
#![doc(html_logo_url = "https://zng-ui.github.io/res/zng-logo.png")]
//!
//! SVG image support.
//!
//! # Crate
//!
#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]
#![warn(unused_extern_crates)]
#![warn(missing_docs)]

use zng_app::AppExtension;
use zng_ext_image::*;
use zng_task::channel::IpcBytes;
use zng_txt::{Txt, formatx};
use zng_unit::{ByteLength, Px, PxDensity2d, PxDensityUnits as _, PxSize};

/// Application extension that installs SVG handling.
///
/// This extension installs a [`IMAGES`] extension on init that handles SVG rendering.
#[derive(Default)]
#[non_exhaustive]
pub struct SvgManager {}

impl AppExtension for SvgManager {
    fn init(&mut self) {
        IMAGES.extend(Box::new(SvgRenderExtension::default()));
    }
}

/// Image service extension that handlers SVG requests.
#[derive(Default)]
#[non_exhaustive]
pub struct SvgRenderExtension {}
impl ImagesExtension for SvgRenderExtension {
    fn image_data(
        &mut self,
        max_decoded_len: zng_unit::ByteLength,
        _key: &ImageHash,
        data: &IpcBytes,
        format: &ImageDataFormat,
        options: &ImageOptions,
    ) -> Option<ImageVar> {
        let data = match format {
            ImageDataFormat::FileExtension(txt) if txt == "svg" || txt == "svgz" => SvgData::Raw(data.to_vec()),
            ImageDataFormat::MimeType(txt) if txt == "image/svg+xml" => SvgData::Raw(data.to_vec()),
            ImageDataFormat::Unknown => SvgData::Str(svg_data_from_unknown(data)?),
            _ => return None,
        };
        let mut options = options.clone();
        let downscale = options.downscale.take();
        options.cache_mode = ImageCacheMode::Ignore;
        let limits = ImageLimits::none().with_max_decoded_len(max_decoded_len);
        Some(IMAGES.image_task(async move { load(max_decoded_len, data, downscale) }, options, Some(limits)))
    }

    fn available_formats(&self, formats: &mut Vec<ImageFormat>) {
        let svg = ImageFormat::from_static("SVG", "svg+xml", "svg", ImageFormatCapability::empty());
        formats.push(svg);
    }
}

enum SvgData {
    Raw(Vec<u8>),
    Str(String),
}
fn load(max_decoded_len: ByteLength, data: SvgData, downscale: Option<ImageDownscaleMode>) -> ImageSource {
    let options = resvg::usvg::Options::default();

    let tree = match data {
        SvgData::Raw(data) => resvg::usvg::Tree::from_data(&data, &options),
        SvgData::Str(data) => resvg::usvg::Tree::from_str(&data, &options),
    };
    match tree {
        Ok(tree) => {
            let mut size = tree.size().to_int_size();

            if let Some(d) = downscale {
                let size_px = PxSize::new(Px(size.width() as _), Px(size.height() as _));

                let (full_size, _) = d.sizes(size_px, &[]);
                let full_size = full_size.unwrap_or(size_px);

                match resvg::tiny_skia::IntSize::from_wh(full_size.width.0 as _, full_size.height.0 as _) {
                    Some(s) => size = s,
                    None => tracing::error!("cannot resize svg to zero size"),
                }
            }

            if size.width() as usize * size.height() as usize * 4 > max_decoded_len.bytes() {
                let img = ImageEntry::new_empty(formatx!("cannot render svg, would exceed max {max_decoded_len} allowed"));
                return ImageSource::Image(zng_var::const_var(img));
            }

            let mut data = match IpcBytes::new_mut_blocking(size.width() as usize * size.height() as usize * 4) {
                Ok(b) => b,
                Err(e) => return error(formatx!("can't allocate bytes for {size:?} svg, {e}")),
            };
            let mut pixmap = match resvg::tiny_skia::PixmapMut::from_bytes(&mut data, size.width(), size.height()) {
                Some(p) => p,
                None => return error(formatx!("can't allocate pixmap for {:?} svg", size)),
            };
            resvg::render(&tree, resvg::tiny_skia::Transform::identity(), &mut pixmap);
            let size = PxSize::new(Px(pixmap.width() as _), Px(pixmap.height() as _));

            for rgba in data.chunks_exact_mut(4) {
                // rgba to bgra
                rgba.swap(0, 2);
            }

            ImageSource::Data(
                ImageHash::compute(&data),
                match data.finish_blocking() {
                    Ok(b) => b,
                    Err(e) => return error(formatx!("cannot finish ipc bytes allocation, {e}")),
                },
                ImageDataFormat::Bgra8 {
                    size,
                    density: Some(PxDensity2d::splat(options.dpi.ppi())),
                    original_color_type: ColorType::RGBA8,
                },
            )
        }
        Err(e) => error(formatx!("{e}")),
    }
}

fn error(error: Txt) -> ImageSource {
    ImageSource::Image(IMAGES.dummy(Some(error)))
}

fn svg_data_from_unknown(data: &[u8]) -> Option<String> {
    if data.starts_with(&[0x1f, 0x8b]) {
        // gzip magic number
        let data = resvg::usvg::decompress_svgz(data).ok()?;
        uncompressed_data_is_svg(&data)
    } else {
        uncompressed_data_is_svg(data)
    }
}
fn uncompressed_data_is_svg(data: &[u8]) -> Option<String> {
    let s = std::str::from_utf8(data).ok()?;
    if s.contains("http://www.w3.org/2000/svg") {
        Some(s.to_owned())
    } else {
        None
    }
}
