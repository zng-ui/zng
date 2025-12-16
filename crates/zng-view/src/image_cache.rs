#![cfg_attr(not(feature = "image_any"), allow(unused))]

use std::{fmt, sync::Arc};
use zng_task::parking_lot::Mutex;
use zng_view_api::image::{ColorType, ImageDownscaleMode, ImageEntriesMode, ImageEntryKind, ImageFormat, ImageMaskMode};

use webrender::api::ImageDescriptor;
use zng_txt::ToTxt as _;
use zng_txt::formatx;

use zng_task::channel::{IpcBytes, IpcReceiver};
use zng_unit::{Px, PxDensity2d, PxSize};
use zng_view_api::{
    Event,
    image::{ImageDataFormat, ImageDecoded, ImageId, ImageMetadata, ImageRequest},
};

use crate::{AppEvent, AppEventSender};
use rustc_hash::FxHashMap;

// Image data is provided to webrender directly from the BGRA8 shared memory.
// The `ExternalImageId` is the Arc pointer to ImageData.
mod capture;
mod decode;
mod encode;
mod external;
mod ipc_dyn_image;
pub(crate) use external::{ImageUseMap, WrImageCache};

#[cfg(not(feature = "image_any"))]
pub(crate) mod lcms2 {
    pub struct Profile {}
}

pub(crate) const FORMATS: &[ImageFormat] = &[
    #[cfg(any(feature = "image_avif", zng_view_image_has_avif))]
    ImageFormat::from_static("AVIF", "avif", "avif", false),
    #[cfg(feature = "image_bmp")]
    ImageFormat::from_static("BMP", "bmp", "bmp,dib", true),
    #[cfg(feature = "image_dds")]
    ImageFormat::from_static("DirectDraw Surface", "vnd-ms.dds,x-direct-draw-surface", "dds", false),
    #[cfg(feature = "image_exr")]
    ImageFormat::from_static("OpenEXR", "x-exr", "exr", false),
    // https://www.wikidata.org/wiki/Q28206109
    #[cfg(feature = "image_ff")]
    ImageFormat::from_static("Farbfeld", "x-farbfeld", "ff,ff.bz2", false),
    #[cfg(feature = "image_gif")]
    ImageFormat::from_static("GIF", "gif", "gif", false),
    #[cfg(feature = "image_hdr")]
    ImageFormat::from_static("Radiance HDR", "vnd.radiance", "hdr", false),
    #[cfg(feature = "image_ico")]
    ImageFormat::from_static("ICO", "x-icon,vnd.microsoft.icon", "ico", true),
    #[cfg(feature = "image_jpeg")]
    ImageFormat::from_static("JPEG", "jpeg", "jpg,jpeg", true),
    #[cfg(feature = "image_png")]
    ImageFormat::from_static("PNG", "png", "png", true),
    #[cfg(feature = "image_pnm")]
    ImageFormat::from_static(
        "PNM",
        "x-portable-bitmap,x-portable-graymap,x-portable-pixmap,x-portable-anymap",
        "pbm,pgm,ppm,pam",
        false,
    ),
    // https://github.com/phoboslab/qoi/issues/167
    #[cfg(feature = "image_qoi")]
    ImageFormat::from_static("QOI", "x-qoi", "qoi", true),
    #[cfg(feature = "image_tga")]
    ImageFormat::from_static("TGA", "x-tga,x-targa", "tga,icb,vda,vst", false),
    #[cfg(feature = "image_tiff")]
    ImageFormat::from_static("TIFF", "tiff", "tif,tiff", true),
    #[cfg(feature = "image_tiff")]
    ImageFormat::from_static("WebP", "webp", "webp", true),
];

pub(crate) type ResizerCache = Mutex<fast_image_resize::Resizer>;

/// Decode and cache image resources.
pub(crate) struct ImageCache {
    app_sender: AppEventSender,
    images: FxHashMap<ImageId, Image>,
    image_id_gen: ImageId,
    resizer: Arc<ResizerCache>,
}
impl ImageCache {
    pub fn new(app_sender: AppEventSender) -> Self {
        Self {
            app_sender,
            images: FxHashMap::default(),
            image_id_gen: ImageId::first(),
            resizer: Arc::new(Mutex::new(fast_image_resize::Resizer::new())),
        }
    }

    pub fn add(
        &mut self,
        ImageRequest {
            format,
            data,
            max_decoded_len,
            downscale,
            mask,
            entries,
            parent,
            ..
        }: ImageRequest<IpcBytes>,
    ) -> ImageId {
        let id = self.image_id_gen.incr();

        let app_sender = self.app_sender.clone();
        let resizer = self.resizer.clone();
        rayon::spawn(move || {
            Self::add_impl(
                app_sender,
                resizer,
                id,
                false,
                format,
                data,
                max_decoded_len,
                downscale,
                mask,
                entries,
                parent,
            );
        });

        id
    }

    pub fn add_pro(
        &mut self,
        ImageRequest {
            format,
            mut data,
            max_decoded_len,
            downscale,
            mask,
            entries,
            parent,
            ..
        }: ImageRequest<IpcReceiver<IpcBytes>>,
    ) -> ImageId {
        let id = self.image_id_gen.incr();
        let app_sender = self.app_sender.clone();
        let resizer = self.resizer.clone();
        rayon::spawn(move || {
            // image crate does not implement progressive decoding, just receive all payloads and continue as `add` for now

            let mut notified_header = false;

            let mut all_data = match data.recv_blocking() {
                Ok(f) => f,
                Err(e) => {
                    let _ = app_sender.send(AppEvent::Notify(Event::ImageDecodeError {
                        image: id,
                        error: formatx!("no image data, {e}"),
                    }));
                    return;
                }
            };
            if let Ok(n) = data.recv_blocking() {
                // try parse header early at least
                #[cfg(feature = "image_any")]
                if let ImageDataFormat::FileExtension(_) | ImageDataFormat::MimeType(_) | ImageDataFormat::Unknown = &format
                    && let Ok(h) = Self::header_decode(&format, &all_data[..])
                {
                    let mut size = h.size;
                    let decoded_len = size.width.0 as u64 * size.height.0 as u64 * 4;
                    if decoded_len > max_decoded_len {
                        let _ = app_sender.send(AppEvent::Notify(Event::ImageDecodeError {
                            image: id,
                            error: formatx!(
                                "image {size:?} needs to allocate {decoded_len} bytes, but max allowed size is {max_decoded_len} bytes",
                            ),
                        }));
                        return;
                    } else {
                        // notify metadata already

                        let (d_size, _) = downscale_sizes(downscale.as_ref(), h.size, &[]);
                        size = d_size.unwrap_or(h.size);
                        let og_color_size = image_color_type_to_vp(h.og_color_type);
                        let mut meta = ImageMetadata::new(id, size, mask.is_some(), og_color_size.clone());
                        meta.density = h.density;

                        let _ = app_sender.send(AppEvent::Notify(Event::ImageMetadataDecoded(meta)));
                        notified_header = true;
                    }
                }

                let mut w = IpcBytes::new_writer_blocking();
                let try_result = (|| -> std::io::Result<IpcBytes> {
                    use std::io::Write as _;
                    w.write_all(&all_data[..])?;
                    w.write_all(&n[..])?;
                    while let Ok(n) = data.recv_blocking() {
                        w.write_all(&n[..])?;
                    }
                    w.finish()
                })();
                match try_result {
                    Ok(d) => all_data = d,
                    Err(e) => {
                        let _ = app_sender.send(AppEvent::Notify(Event::ImageDecodeError {
                            image: id,
                            error: formatx!("cannot receive image data, {e}"),
                        }));
                        return;
                    }
                }
            }

            Self::add_impl(
                app_sender,
                resizer,
                id,
                notified_header,
                format,
                all_data,
                max_decoded_len,
                downscale,
                mask,
                entries,
                parent,
            );
        });
        id
    }

    #[allow(clippy::too_many_arguments)]
    fn add_impl(
        app_sender: AppEventSender,
        resizer: Arc<ResizerCache>,
        id: ImageId,
        notified_meta: bool,

        format: ImageDataFormat,
        data: IpcBytes,
        max_decoded_len: u64,
        downscale: Option<ImageDownscaleMode>,
        mask: Option<ImageMaskMode>,
        entries: ImageEntriesMode,
        parent: Option<(ImageId, ImageEntryKind)>,
    ) {
        let mut og_color_size = ColorType::BGRA8;
        let mut downscale_sizes = (None, vec![]);
        let r = match format {
            ImageDataFormat::Bgra8 {
                size,
                density,
                original_color_type,
            } => {
                // is already decoded bgra8

                downscale_sizes = self::downscale_sizes(downscale.as_ref(), size, &[]);
                og_color_size = original_color_type;

                let expected_len = size.width.0 as usize * size.height.0 as usize * 4;
                if data.len() != expected_len {
                    Err(formatx!(
                        "pixels.len() is not width * height * 4, expected {expected_len}, found {}",
                        data.len()
                    ))
                } else if let Some(mask) = mask {
                    // but is used as mask, convert to a8

                    Self::convert_bgra8_to_mask(size, &data, mask, density, downscale_sizes.0, &resizer).map_err(|e| e.to_txt())
                } else {
                    // and is used as bgra8, downscale if needed
                    match Self::downscale_decoded(mask, downscale_sizes.0, &resizer, size, &data) {
                        Ok(downscaled) => match downscaled {
                            Some((size, data_mut)) => match data_mut.finish_blocking() {
                                Ok(data) => {
                                    let is_opaque = data.chunks_exact(4).all(|c| c[3] == 255);
                                    Ok((data, size, None, is_opaque, true))
                                }
                                Err(e) => Err(e.to_txt()),
                            },
                            None => {
                                let is_opaque = data.chunks_exact(4).all(|c| c[3] == 255);
                                Ok((data, size, None, is_opaque, true))
                            }
                        },
                        Err(e) => Err(e.to_txt()),
                    }
                }
            }
            ImageDataFormat::A8 { size } => {
                // is already decoded mask

                downscale_sizes = self::downscale_sizes(downscale.as_ref(), size, &[]);
                og_color_size = ColorType::A8;

                let expected_len = size.width.0 as usize * size.height.0 as usize;
                if data.len() != expected_len {
                    Err(formatx!(
                        "pixels.len() is not width * height, expected {expected_len}, found {}",
                        data.len()
                    ))
                } else if mask.is_none() {
                    // but is used as mask, convert to bgra8
                    Self::convert_a8_to_bgra8(size, &data, None, downscale_sizes.0, &resizer).map_err(|e| e.to_txt())
                } else {
                    // and is used as mask, downscale if needed
                    match Self::downscale_decoded(mask, downscale_sizes.0, &resizer, size, &data) {
                        Ok(downscaled) => match downscaled {
                            Some((size, data_mut)) => match data_mut.finish_blocking() {
                                Ok(data) => {
                                    let is_opaque = data.iter().all(|&c| c == 255);
                                    Ok((data, size, None, is_opaque, true))
                                }
                                Err(e) => Err(e.to_txt()),
                            },
                            None => {
                                let is_opaque = data.iter().all(|&c| c == 255);
                                Ok((data, size, None, is_opaque, true))
                            }
                        },
                        Err(e) => Err(e.to_txt()),
                    }
                }
            }
            fmt => {
                // needs decoding

                #[cfg(not(feature = "image_any"))]
                {
                    let _ = (max_decoded_len, downscale);
                    Err(zng_txt::formatx!("no decoder for {fmt:?}"))
                }

                // identify codec and parse header metadata
                #[cfg(feature = "image_any")]
                match Self::header_decode(&fmt, &data[..]) {
                    Ok(h) => {
                        let mut size = h.size;
                        let decoded_len = size.width.0 as u64 * size.height.0 as u64 * 4;
                        if decoded_len > max_decoded_len {
                            Err(formatx!(
                                "image {size:?} needs to allocate {decoded_len} bytes, but max allowed size is {max_decoded_len} bytes",
                            ))
                        } else {
                            // notify metadata already

                            downscale_sizes = self::downscale_sizes(downscale.as_ref(), h.size, &[]);
                            size = downscale_sizes.0.unwrap_or(h.size);
                            og_color_size = image_color_type_to_vp(h.og_color_type);
                            let mut meta = ImageMetadata::new(id, size, mask.is_some(), og_color_size.clone());
                            meta.density = h.density;

                            if !notified_meta {
                                let _ = app_sender.send(AppEvent::Notify(Event::ImageMetadataDecoded(meta)));
                            }

                            // decode
                            match Self::image_decode(&data[..], h.format, 0) {
                                // convert to bgra8, downscale and rotate
                                Ok(img) => {
                                    match Self::convert_decoded(
                                        img,
                                        mask,
                                        h.density,
                                        h.icc_profile,
                                        downscale_sizes.0,
                                        h.orientation,
                                        &resizer,
                                    ) {
                                        Ok(r) => Ok(r),
                                        Err(e) => Err(e.to_txt()),
                                    }
                                }
                                Err(e) => Err(e.to_txt()),
                            }
                        }
                    }
                    Err(e) => Err(e),
                }
            }
        };

        match r {
            Ok((pixels, size, density, is_opaque, is_mask)) => {
                let mut meta = ImageMetadata::new(id, size, is_mask, og_color_size);
                meta.density = density;
                let _ = app_sender.send(AppEvent::ImageDecoded(ImageDecoded::new(meta, pixels, is_opaque)));
            }
            Err(e) => {
                let _ = app_sender.send(AppEvent::Notify(Event::ImageDecodeError { image: id, error: e }));
            }
        }
    }

    pub fn forget(&mut self, id: ImageId) {
        self.images.remove(&id);
    }

    pub fn get(&self, id: ImageId) -> Option<&Image> {
        self.images.get(&id)
    }

    /// Called after receive and decode completes correctly.
    pub(crate) fn loaded(&mut self, data: ImageDecoded) {
        self.images.insert(
            data.meta.id,
            Image(Arc::new(ImageData::RawData {
                size: data.meta.size,
                range: 0..data.pixels.len(),
                pixels: data.pixels.clone(),
                is_opaque: data.is_opaque,
                density: data.meta.density,
                stripes: Mutex::new(Box::new([])),
            })),
        );

        let _ = self.app_sender.send(AppEvent::Notify(Event::ImageDecoded(data)));
    }

    pub(crate) fn on_low_memory(&mut self) {
        // app-process controls what images are dropped so hopefully it will respond the
        // memory pressure event
        self.resizer.lock().reset_internal_buffers();
    }

    pub(crate) fn clear(&mut self) {
        self.images.clear();
    }
}
#[cfg(feature = "image_any")]
struct ImageHeader {
    format: image::ImageFormat,
    size: PxSize,
    orientation: image::metadata::Orientation,
    density: Option<PxDensity2d>,
    icc_profile: Option<lcms2::Profile>,
    og_color_type: image::ExtendedColorType,
}

fn image_color_type_to_vp(color_type: image::ExtendedColorType) -> ColorType {
    let channels = color_type.channel_count();
    ColorType::new(
        format!("{color_type:?}").to_uppercase().into(),
        (color_type.bits_per_pixel() / channels as u16) as u8,
        channels,
    )
}

/// (pixels, size, density, is_opaque, is_mask)
type RawLoadedImg = (IpcBytes, PxSize, Option<PxDensity2d>, bool, bool);

pub(crate) enum ImageData {
    RawData {
        size: PxSize,
        pixels: IpcBytes,
        is_opaque: bool,
        density: Option<PxDensity2d>,
        range: std::ops::Range<usize>,
        stripes: Mutex<Box<[Image]>>,
    },
    NativeTexture {
        uv: webrender::api::units::TexelRect,
        texture: gleam::gl::GLuint,
    },
}
impl ImageData {
    pub fn is_opaque(&self) -> bool {
        match self {
            ImageData::RawData { is_opaque, .. } => *is_opaque,
            ImageData::NativeTexture { .. } => false,
        }
    }

    pub fn is_mask(&self) -> bool {
        match self {
            ImageData::RawData { size, range, .. } => size.width.0 as usize * size.height.0 as usize == range.len(),
            ImageData::NativeTexture { .. } => false,
        }
    }
}

#[derive(Clone)]
pub(crate) struct Image(Arc<ImageData>);
impl fmt::Debug for Image {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &*self.0 {
            ImageData::RawData {
                size,
                pixels,
                is_opaque,
                density,
                range,
                ..
            } => f
                .debug_struct("Image")
                .field("size", size)
                .field("is_opaque", is_opaque)
                .field("density", density)
                .field("pixels", &format_args!("<{} of {} shared bytes>", range.len(), pixels.len()))
                .finish(),
            ImageData::NativeTexture { .. } => unreachable!(),
        }
    }
}
impl Image {
    pub fn descriptor(&self) -> ImageDescriptor {
        match &*self.0 {
            ImageData::RawData {
                size, is_opaque, range, ..
            } => {
                // no Webrender mipmaps here, thats only for the GPU,
                // it does not help with performance rendering gigapixel images scaled to fit
                let mut flags = webrender::api::ImageDescriptorFlags::empty();
                if *is_opaque {
                    flags |= webrender::api::ImageDescriptorFlags::IS_OPAQUE;
                }
                let is_mask = size.width.0 as usize * size.height.0 as usize == range.len();
                ImageDescriptor {
                    format: if is_mask {
                        webrender::api::ImageFormat::R8
                    } else {
                        webrender::api::ImageFormat::BGRA8
                    },
                    size: size.cast().cast_unit(),
                    stride: None,
                    offset: 0,
                    flags,
                }
            }
            ImageData::NativeTexture { .. } => unreachable!(),
        }
    }

    #[allow(unused)]
    pub fn is_opaque(&self) -> bool {
        match &*self.0 {
            ImageData::RawData { is_opaque, .. } => *is_opaque,
            ImageData::NativeTexture { .. } => unreachable!(),
        }
    }

    #[allow(unused)]
    pub fn size(&self) -> PxSize {
        match &*self.0 {
            ImageData::RawData { size, .. } => *size,
            ImageData::NativeTexture { .. } => unreachable!(),
        }
    }

    #[allow(unused)]
    pub fn pixels(&self) -> &IpcBytes {
        match &*self.0 {
            ImageData::RawData { pixels, .. } => pixels,
            ImageData::NativeTexture { .. } => unreachable!(),
        }
    }

    #[allow(unused)]
    pub fn density(&self) -> Option<PxDensity2d> {
        match &*self.0 {
            ImageData::RawData { density, .. } => *density,
            _ => unreachable!(),
        }
    }

    #[allow(unused)]
    pub fn range(&self) -> std::ops::Range<usize> {
        match &*self.0 {
            ImageData::RawData { range, .. } => range.clone(),
            _ => unreachable!(),
        }
    }

    /// If this is `true` needs to replace with `wr_stripes`
    pub fn overflows_wr(&self) -> bool {
        self.pixels().len() > Self::MAX_LEN
    }

    /// Returns the image split in "stripes" that fit the Webrender buffer length constraints.
    ///
    /// If the image cannot be split into stripes returns an empty list. This only happens if the image width is absurdly wide.
    pub fn wr_stripes(&self) -> Box<[Image]> {
        if !self.overflows_wr() {
            return Box::new([self.clone()]);
        }

        match &*self.0 {
            ImageData::RawData {
                size,
                pixels,
                is_opaque,
                density,
                range,
                stripes,
                ..
            } => {
                debug_assert_eq!(range.len(), pixels.len());

                let mut stripes = stripes.lock();
                if stripes.is_empty() {
                    *stripes = self.generate_stripes(*size, pixels, *is_opaque, *density);
                }
                (*stripes).clone()
            }
            _ => unreachable!(),
        }
    }
    const MAX_LEN: usize = i32::MAX as usize;
    fn generate_stripes(&self, full_size: PxSize, pixels: &IpcBytes, is_opaque: bool, density: Option<PxDensity2d>) -> Box<[Image]> {
        let w = full_size.width.0 as usize * 4;
        if w > Self::MAX_LEN {
            tracing::error!("renderer does not support images with width * 4 > {}", Self::MAX_LEN);
            return Box::new([]);
        }

        // find proportional split that fits, to avoid having the last stripe be to thin
        let full_height = full_size.height.0 as usize;
        let mut stripe_height = full_height / 2;
        while w * stripe_height > Self::MAX_LEN {
            stripe_height /= 2;
        }
        let stripe_len = w * stripe_height;
        let stripes_len = full_height.div_ceil(stripe_height);
        let stripe_height = Px(stripe_height as _);

        let mut stripes = Vec::with_capacity(stripes_len);

        for i in 0..stripes_len {
            let y = stripe_height * Px(i as _);
            let mut size = full_size;
            size.height = stripe_height.min(full_size.height - y);

            let offset = stripe_len * i;
            let range = offset..((offset + stripe_len).min(pixels.len()));

            let stripe = Image(Arc::new(ImageData::RawData {
                size,
                pixels: pixels.clone(),
                is_opaque,
                density,
                range,
                // always empty
                stripes: Mutex::new(Box::new([])),
            }));

            stripes.push(stripe);
        }

        stripes.into_boxed_slice()
    }
}

fn downscale_sizes(downscale: Option<&ImageDownscaleMode>, page_size: PxSize, reduced_sizes: &[PxSize]) -> (Option<PxSize>, Vec<PxSize>) {
    match downscale {
        Some(d) => d.sizes(page_size, reduced_sizes),
        None => (None, vec![]),
    }
}
