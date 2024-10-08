#![doc(html_favicon_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo-icon.png")]
#![doc(html_logo_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo.png")]
//!
//! SVG image support.
//!
//! # Crate
//!
#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]
#![warn(unused_extern_crates)]
#![warn(missing_docs)]

/*
!!: TODO

* Verify how proxy can let service handlers (down)loading.
* Allow resize, size picking (render images don't handle this either? Same image can be used in multiple sizes).
* Add test image in example.
* Update example screenshot.

*/

use std::sync::Arc;

use zng_app::AppExtension;
use zng_ext_image::*;
use zng_txt::{formatx, Txt};
use zng_unit::{Px, PxSize};
use zng_var::{var, Var};

/// Application extension that installs SVG handling.
///
/// This extension installs the [`SvgRenderCache`] in [`IMAGES`] on init.
#[derive(Default)]
pub struct SvgManager {}

impl AppExtension for SvgManager {
    fn init(&mut self) {
        IMAGES.install_proxy(Box::new(SvgRenderCache::default()));
    }
}

/// Image cache proxy that handlers SVG requests.
#[derive(Default)]
pub struct SvgRenderCache {}

impl SvgRenderCache {
    fn get_data(
        &mut self,
        key: &ImageHash,
        data: &[u8],
        format: &ImageDataFormat,
        mode: ImageCacheMode,
        downscale: Option<ImageDownscale>,
        mask: Option<ImageMaskMode>,
    ) -> ProxyGetResult {
        match format {
            ImageDataFormat::FileExtension(txt) if txt == "svg" => self.load(key, data, true, mode, downscale, mask),
            ImageDataFormat::MimeType(txt) if txt == "image/svg+xml" => self.load(key, data, true, mode, downscale, mask),
            ImageDataFormat::Unknown => self.load(key, data, false, mode, downscale, mask),
            _ => ProxyGetResult::None,
        }
    }

    fn load(
        &mut self,
        key: &ImageHash,
        data: &[u8],
        known_type_svg: bool,
        mode: ImageCacheMode,
        downscale: Option<ImageDownscale>,
        mask: Option<ImageMaskMode>,
    ) -> ProxyGetResult {
        let options = resvg::usvg::Options::default();
        match resvg::usvg::Tree::from_data(data, &options) {
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
                    None => return Self::error(formatx!("can't allocate pixmap for {:?} svg", size)),
                };
                resvg::render(&tree, resvg::tiny_skia::Transform::identity(), &mut pixmap.as_mut());
                let size = PxSize::new(Px(pixmap.width() as _), Px(pixmap.height() as _));

                let mut data = pixmap.take();
                for pixel in data.chunks_exact_mut(4) {
                    pixel.swap(0, 2);
                }

                ProxyGetResult::Cache(
                    ImageSource::Data(
                        *key,
                        Arc::new(data),
                        ImageDataFormat::Bgra8 {
                            size,
                            ppi: Some(ImagePpi::splat(options.dpi)),
                        },
                    ),
                    mode,
                    None,
                    mask,
                )
            }
            Err(e) => {
                if known_type_svg {
                    Self::error(formatx!("{e}"))
                } else {
                    tracing::debug!("cannot parse image of unknown format as svg, {e}");
                    ProxyGetResult::None
                }
            }
        }
    }

    fn error(error: Txt) -> ProxyGetResult {
        ProxyGetResult::Image(var(Img::dummy(Some(error))).read_only())
    }
}

impl ImageCacheProxy for SvgRenderCache {
    fn get(
        &mut self,
        key: &ImageHash,
        source: &ImageSource,
        mode: ImageCacheMode,
        downscale: Option<ImageDownscale>,
        mask: Option<ImageMaskMode>,
    ) -> ProxyGetResult {
        match source {
            ImageSource::Static(_, d, f) => self.get_data(key, d, f, mode, downscale, mask),
            ImageSource::Data(_, d, f) => self.get_data(key, d, f, mode, downscale, mask),
            // let IMAGES handle Read/Download
            _ => ProxyGetResult::None,
        }
    }

    fn clear(&mut self, _: bool) {}
}
