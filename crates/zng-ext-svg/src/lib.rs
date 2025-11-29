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
use zng_unit::{Px, PxDensity2d, PxDensityUnits as _, PxSize};

/// Application extension that installs SVG handling.
///
/// This extension installs the [`SvgRenderCache`] in [`IMAGES`] on init.
#[derive(Default)]
#[non_exhaustive]
pub struct SvgManager {}

impl AppExtension for SvgManager {
    fn init(&mut self) {
        IMAGES.install_proxy(Box::new(SvgRenderCache::default()));
    }
}

/// Image cache proxy that handlers SVG requests.
#[derive(Default)]
#[non_exhaustive]
pub struct SvgRenderCache {}
impl ImageCacheProxy for SvgRenderCache {
    fn data(
        &mut self,
        key: &ImageHash,
        data: &[u8],
        format: &ImageDataFormat,
        mode: ImageCacheMode,
        downscale: Option<ImageDownscale>,
        mask: Option<ImageMaskMode>,
        is_loaded: bool,
    ) -> Option<ImageVar> {
        let data = match format {
            ImageDataFormat::FileExtension(txt) if txt == "svg" || txt == "svgz" => SvgData::Raw(data.to_vec()),
            ImageDataFormat::MimeType(txt) if txt == "image/svg+xml" => SvgData::Raw(data.to_vec()),
            ImageDataFormat::Unknown => SvgData::Str(svg_data_from_unknown(data)?),
            _ => return None,
        };
        let key = if is_loaded {
            None // already cached, return image is internal
        } else {
            Some(*key)
        };
        Some(IMAGES.image_task(async move { load(data, downscale) }, mode, key, None, None, mask))
    }

    fn is_data_proxy(&self) -> bool {
        true
    }
}

enum SvgData {
    Raw(Vec<u8>),
    Str(String),
}
fn load(data: SvgData, downscale: Option<ImageDownscale>) -> ImageSource {
    let options = resvg::usvg::Options::default();

    let tree = match data {
        SvgData::Raw(data) => resvg::usvg::Tree::from_data(&data, &options),
        SvgData::Str(data) => resvg::usvg::Tree::from_str(&data, &options),
    };
    match tree {
        Ok(tree) => {
            let mut size = tree.size().to_int_size();
            if let Some(d) = downscale {
                let s = d.resize_dimensions(PxSize::new(Px(size.width() as _), Px(size.height() as _)));
                match resvg::tiny_skia::IntSize::from_wh(s.width.0 as _, s.height.0 as _) {
                    Some(s) => size = s,
                    None => tracing::error!("cannot resize svg to zero size"),
                }
            }
            let mut pixmap = match resvg::tiny_skia::Pixmap::new(size.width(), size.height()) {
                Some(p) => p,
                None => return error(formatx!("can't allocate pixmap for {:?} svg", size)),
            };
            resvg::render(&tree, resvg::tiny_skia::Transform::identity(), &mut pixmap.as_mut());
            let size = PxSize::new(Px(pixmap.width() as _), Px(pixmap.height() as _));

            let mut data = pixmap.take();
            for pixel in data.chunks_exact_mut(4) {
                pixel.swap(0, 2);
            }

            ImageSource::Data(
                ImageHash::compute(&data),
                IpcBytes::from_vec_blocking(data).expect("cannot allocate IpcBytes"),
                ImageDataFormat::Bgra8 {
                    size,
                    density: Some(PxDensity2d::splat(options.dpi.ppi())),
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
