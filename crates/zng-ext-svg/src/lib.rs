#![doc(html_favicon_url = "https://zng-ui.github.io/res/zng-logo-icon.png")]
#![doc(html_logo_url = "https://zng-ui.github.io/res/zng-logo.png")]
//!
//! SVG image support.
//!
//! This extension installs a [`IMAGES`] extension on init that handles SVG rendering.
//!
//! # Crate
//!
#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]
#![warn(unused_extern_crates)]
#![warn(missing_docs)]

use std::io::{self, Read, Seek};

use zng_app::{APP, hn};
use zng_ext_image::*;
use zng_task::channel::{IpcBytesMut, IpcReadBlocking, IpcReadHandle};
use zng_txt::{Txt, formatx};
use zng_unit::{ByteLength, ByteUnits as _, Px, PxDensity2d, PxDensityUnits as _, PxSize};
use zng_var::const_var;

zng_env::on_process_start!(|args| {
    if args.yield_until_app() {
        return;
    }

    APP.on_init(hn!(|_| {
        tracing::trace!("register SVG extension");
        IMAGES.extend(Box::new(SvgRenderExtension::default()));
    }));
});

/// Image service extension that handlers SVG requests.
#[derive(Default)]
#[non_exhaustive]
pub struct SvgRenderExtension {}
impl ImagesExtension for SvgRenderExtension {
    fn image_data(
        &mut self,
        max_decoded_len: zng_unit::ByteLength,
        _key: &ImageHash,
        data: &IpcReadHandle,
        format: &ImageDataFormat,
        options: &ImageOptions,
    ) -> Option<ImageVar> {
        let data = match format {
            ImageDataFormat::FileExtension(txt) if txt == "svg" || txt == "svgz" => SvgData::Raw(data.duplicate().ok()?),
            ImageDataFormat::MimeType(txt) if txt == "image/svg+xml" => SvgData::Raw(data.duplicate().ok()?),
            ImageDataFormat::Unknown => SvgData::Str(svg_data_from_unknown(data)?),
            _ => return None,
        };
        tracing::trace!("svg request intercepted");
        let mut options = options.clone();
        let downscale = options.downscale.take();
        options.cache_mode = ImageCacheMode::Ignore;
        let limits = ImageLimits::none().with_max_decoded_len(max_decoded_len);
        Some(IMAGES.image_task(async move { load_render(max_decoded_len, data, downscale) }, options, Some(limits)))
    }

    fn available_formats(&self, formats: &mut Vec<ImageFormat>) {
        let svg = ImageFormat::from_static2("SVG", "svg+xml", "svg", "", ImageFormatCapability::empty());
        formats.push(svg);
    }
}

enum SvgData {
    Raw(IpcReadHandle),
    Str(String),
}
fn load_render(max_decoded_len: ByteLength, data: SvgData, downscale: Option<ImageDownscaleMode>) -> ImageSource {
    let options = resvg::usvg::Options::default();

    let tree = match data {
        SvgData::Raw(data) => match data.read_to_bytes_blocking() {
            Ok(data) => resvg::usvg::Tree::from_data(&data, &options),
            Err(e) => {
                tracing::error!("cannot read svg image data, {e}");
                Err(resvg::usvg::Error::NotAnUtf8Str) // no custom error branch
            }
        },
        SvgData::Str(data) => resvg::usvg::Tree::from_str(&data, &options),
    };
    match tree {
        Ok(tree) => {
            let mut size = tree.size().to_int_size();
            let mut entry_sizes = vec![];

            fn to_skia_size(size: PxSize) -> Option<resvg::tiny_skia::IntSize> {
                match resvg::tiny_skia::IntSize::from_wh(size.width.0 as _, size.height.0 as _) {
                    Some(s) => Some(s),
                    None => {
                        tracing::error!("cannot resize svg to zero size");
                        None
                    }
                }
            }
            if let Some(d) = downscale {
                let size_px = PxSize::new(Px(size.width() as _), Px(size.height() as _));

                let (full_size, entries) = d.sizes(size_px, &[]);
                size = full_size.and_then(to_skia_size).unwrap_or(size);

                for entry in entries {
                    if let Some(s) = to_skia_size(entry) {
                        entry_sizes.push(s);
                    }
                }
            }

            let render = |size: resvg::tiny_skia::IntSize| -> ImageSource {
                if size.width() as usize * size.height() as usize * 4 > max_decoded_len.bytes() {
                    return error(formatx!("cannot render svg, would exceed max {max_decoded_len} allowed"));
                }
                let mut data = match IpcBytesMut::new_blocking(size.width() as usize * size.height() as usize * 4) {
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
            };

            let primary = render(size);
            if entry_sizes.is_empty() {
                primary
            } else {
                let entries = entry_sizes
                    .into_iter()
                    .map(|s| (ImageEntryKind::Reduced { synthetic: true }, render(s)))
                    .collect();
                ImageSource::Entries {
                    primary: Box::new(primary),
                    entries,
                }
            }
        }
        Err(e) => error(formatx!("{e}")),
    }
}

fn error(error: Txt) -> ImageSource {
    ImageSource::Image(const_var(ImageEntry::new_error(error)))
}

fn svg_data_from_unknown(data: &IpcReadHandle) -> Option<String> {
    let mut data = data.duplicate().ok()?.read_blocking().ok()?;
    let mut buf = [0u8; 2];
    data.read_exact(&mut buf).ok()?;
    data.seek(io::SeekFrom::Start(0)).ok()?;

    // 3KB should allow for some comments at beginning before <svg
    let header_len = 3.kilobytes().bytes();

    if buf == [0x1f, 0x8b] {
        // gzip magic number
        // resvg::usvg::decompress_svgz(&[u8]) uses flate2::read::GzDecoder

        let mut data = flate2::read::GzDecoder::new(data);
        let mut buf = vec![];

        data.by_ref().take(header_len as u64).read_to_end(&mut buf).ok()?;
        find_open_svg(&buf)?;
        data.read_to_end(&mut buf).ok()?;
        String::from_utf8(buf).ok()
    } else {
        match data {
            IpcReadBlocking::File(mut r) => {
                let len = r.get_mut().metadata().ok()?.len();
                let mut buf = String::with_capacity(usize::try_from(len).ok()?);
                r.read_to_string(&mut buf).ok()?;
                Some(buf)
            }
            IpcReadBlocking::Bytes(b) => {
                let b = b.get_ref();
                let header_len = b.len().min(header_len);
                find_open_svg(&b[..header_len])?;
                Some(str::from_utf8(b).ok()?.to_owned())
            }
            _ => None,
        }
    }
}
fn find_open_svg(buf: &[u8]) -> Option<usize> {
    let len = buf.len().saturating_sub(3);

    for i in 0..len {
        if buf[i] == b'<' {
            // ASCII lowercase via OR 0x20
            let b1 = buf[i + 1] | 0x20;
            let b2 = buf[i + 2] | 0x20;
            let b3 = buf[i + 3] | 0x20;
            if b1 == b's' && b2 == b'v' && b3 == b'g' {
                return Some(i);
            }
        }
    }

    None
}
