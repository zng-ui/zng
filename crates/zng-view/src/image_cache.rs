#![cfg_attr(not(feature = "image_any"), allow(unused))]

#[cfg(feature = "image_any")]
use image::ImageDecoder as _;
use std::{fmt, sync::Arc};
use zng_task::parking_lot::Mutex;

use webrender::api::ImageDescriptor;
use zng_txt::ToTxt as _;
use zng_txt::{Txt, formatx};

use zng_task::channel::{IpcBytes, IpcReceiver};
use zng_unit::{Px, PxDensity2d, PxSize};
use zng_view_api::{
    Event,
    image::{ImageDataFormat, ImageId, ImageLoadedData, ImageRequest},
};

use crate::{AppEvent, AppEventSender};
use rustc_hash::FxHashMap;

// Image data is provided to webrender directly from the BGRA8 shared memory.
// The `ExternalImageId` is the Arc pointer to ImageData.
mod capture;
mod decode;
mod encode;
mod external;
pub(crate) use external::{ImageUseMap, WrImageCache};

#[cfg(not(feature = "image_any"))]
pub(crate) mod lcms2 {
    pub struct Profile {}
}

pub(crate) const ENCODERS: &[&str] = &[
    #[cfg(feature = "image_png")]
    "png",
    #[cfg(feature = "image_jpeg")]
    "jpg",
    #[cfg(feature = "image_jpeg")]
    "jpeg",
    #[cfg(feature = "image_webp")]
    "webp",
    #[cfg(any(feature = "image_avif", zng_view_image_has_avif))]
    "avif",
    #[cfg(feature = "image_gif")]
    "gif",
    #[cfg(feature = "image_ico")]
    "ico",
    #[cfg(feature = "image_bmp")]
    "bmp",
    #[cfg(feature = "image_jpeg")]
    "jfif",
    #[cfg(feature = "image_exr")]
    "exr",
    #[cfg(feature = "image_hdr")]
    "hdr",
    #[cfg(feature = "image_pnm")]
    "pnm",
    #[cfg(feature = "image_qoi")]
    "qoi",
    #[cfg(feature = "image_ff")]
    "ff",
    #[cfg(feature = "image_ff")]
    "farbfeld",
];
pub(crate) const DECODERS: &[&str] = &[
    #[cfg(feature = "image_png")]
    "png",
    #[cfg(feature = "image_jpeg")]
    "jpg",
    #[cfg(feature = "image_jpeg")]
    "jpeg",
    #[cfg(feature = "image_webp")]
    "webp",
    #[cfg(any(feature = "image_avif", zng_view_image_has_avif))]
    "avif",
    #[cfg(feature = "image_gif")]
    "gif",
    #[cfg(feature = "image_ico")]
    "ico",
    #[cfg(feature = "image_bmp")]
    "bmp",
    #[cfg(feature = "image_jpeg")]
    "jfif",
    #[cfg(feature = "image_exr")]
    "exr",
    #[cfg(feature = "image_pnm")]
    "pnm",
    #[cfg(feature = "image_qoi")]
    "qoi",
    #[cfg(feature = "image_ff")]
    "ff",
    #[cfg(feature = "image_ff")]
    "farbfeld",
    #[cfg(feature = "image_dds")]
    "dds",
];

type ResizerCache = Mutex<fast_image_resize::Resizer>;

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
            ..
        }: ImageRequest<IpcBytes>,
    ) -> ImageId {
        let id = self.image_id_gen.incr();

        let app_sender = self.app_sender.clone();
        let resizer = self.resizer.clone();
        rayon::spawn(move || {
            let r = match format {
                ImageDataFormat::Bgra8 { size, density } => {
                    // is already decoded bgra8

                    let expected_len = size.width.0 as usize * size.height.0 as usize * 4;
                    if data.len() != expected_len {
                        Err(formatx!(
                            "pixels.len() is not width * height * 4, expected {expected_len}, found {}",
                            data.len()
                        ))
                    } else if let Some(mask) = mask {
                        // but is used as mask, convert to a8

                        Self::convert_bgra8_to_mask(size, &data, mask, density, downscale, &resizer).map_err(|e| e.to_txt())
                    } else {
                        // and is used as bgra8, downscale if needed
                        match Self::downscale_decoded(mask, downscale, &resizer, size, &data) {
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

                    let expected_len = size.width.0 as usize * size.height.0 as usize;
                    if data.len() != expected_len {
                        Err(formatx!(
                            "pixels.len() is not width * height, expected {expected_len}, found {}",
                            data.len()
                        ))
                    } else if mask.is_none() {
                        // but is used as mask, convert to bgra8
                        let r = Self::convert_decoded(
                            image::DynamicImage::ImageLuma8(
                                image::ImageBuffer::from_raw(size.width.0 as _, size.height.0 as _, data.to_vec()).unwrap(),
                            ),
                            None,
                            None,
                            None,
                            downscale,
                            &resizer,
                        );
                        match r {
                            Ok((pixels, size, _, is_opaque, _)) => Ok((pixels, size, None, is_opaque, false)),
                            Err(e) => Err(e.to_txt()),
                        }
                    } else {
                        // and is used as mask, downscale if needed
                        match Self::downscale_decoded(mask, downscale, &resizer, size, &data) {
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
                                if let Some(d) = downscale {
                                    size = d.resize_dimensions(size);
                                }
                                let _ = app_sender.send(AppEvent::Notify(Event::ImageMetadataLoaded {
                                    image: id,
                                    size,
                                    density: h.density,
                                    is_mask: false,
                                }));

                                // decode
                                match Self::image_decode(&data[..], h.format, downscale, h.orientation) {
                                    // convert to bgra8 and downscale
                                    Ok(img) => match Self::convert_decoded(img, mask, h.density, h.icc_profile, downscale, &resizer) {
                                        Ok(r) => Ok(r),
                                        Err(e) => Err(e.to_txt()),
                                    },
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
                    let _ = app_sender.send(AppEvent::ImageLoaded(ImageLoadedData::new(
                        id, size, density, is_opaque, is_mask, pixels,
                    )));
                }
                Err(e) => {
                    let _ = app_sender.send(AppEvent::Notify(Event::ImageLoadError { image: id, error: e }));
                }
            }
        });

        id
    }

    pub fn add_pro(
        &mut self,
        ImageRequest {
            format: request_fmt,
            mut data,
            max_decoded_len,
            downscale,
            mask,
            ..
        }: ImageRequest<IpcReceiver<IpcBytes>>,
    ) -> ImageId {
        let id = self.image_id_gen.incr();
        let app_sender = self.app_sender.clone();
        let resizer = self.resizer.clone();
        rayon::spawn(move || {
            // crate `images` does not do progressive decode.
            let mut full = vec![];
            let mut size = None;
            let mut density = None;
            let mut icc_profile = None::<lcms2::Profile>;
            let mut is_encoded = true;
            let mut orientation = image::metadata::Orientation::NoTransforms;

            let mut format: Option<image::ImageFormat> = match &request_fmt {
                ImageDataFormat::Bgra8 { size: s, density: p } => {
                    is_encoded = false;
                    size = Some(*s);
                    density = *p;
                    None
                }
                ImageDataFormat::A8 { size: s } => {
                    is_encoded = false;
                    size = Some(*s);
                    None
                }
                #[cfg(feature = "image_any")]
                ImageDataFormat::FileExtension(ext) => image::ImageFormat::from_extension(ext.as_str()),
                #[cfg(feature = "image_any")]
                ImageDataFormat::MimeType(t) => t.strip_prefix("image/").and_then(image::ImageFormat::from_extension),
                ImageDataFormat::Unknown => None,
                _ => None,
            };

            let mut pending = true;
            while pending {
                match data.recv_blocking() {
                    Ok(d) => {
                        pending = !d.is_empty();

                        full.extend(d.iter().copied());

                        #[cfg(feature = "image_any")]
                        if let Some(fmt) = format {
                            if size.is_none() {
                                if let Ok(h) = Self::header_decode(&request_fmt, &full) {
                                    size = Some(h.size);
                                    orientation = h.orientation;
                                    format = Some(h.format);
                                    density = h.density;
                                    icc_profile = h.icc_profile;
                                }
                                if let Ok(mut d) = image::ImageReader::with_format(std::io::Cursor::new(&full), fmt).into_decoder() {
                                    use image::metadata::Orientation::*;

                                    let (mut w, mut h) = d.dimensions();
                                    orientation = d.orientation().unwrap_or(NoTransforms);

                                    if matches!(orientation, Rotate90 | Rotate270 | Rotate90FlipH | Rotate270FlipH) {
                                        std::mem::swap(&mut w, &mut h)
                                    }

                                    size = Some(PxSize::new(Px(w as i32), Px(h as i32)));
                                }

                                if let Some(s) = size {
                                    let decoded_len = s.width.0 as u64 * s.height.0 as u64 * 4;
                                    if decoded_len > max_decoded_len {
                                        let error = formatx!(
                                            "image {size:?} needs to allocate {decoded_len} bytes, but max allowed size is {max_decoded_len} bytes",
                                        );
                                        let _ = app_sender.send(AppEvent::Notify(Event::ImageLoadError { image: id, error }));
                                        return;
                                    }
                                }
                            }
                        } else if is_encoded {
                            format = image::guess_format(&full).ok();
                        }
                    }
                    Err(_) => {
                        // cancelled?
                        return;
                    }
                }
            }

            if let Some(fmt) = format {
                #[cfg(not(feature = "image_any"))]
                let _ = (
                    fmt,
                    max_decoded_len,
                    downscale,
                    mask,
                    &mut icc_profile,
                    &mut orientation,
                    &mut format,
                );

                #[cfg(feature = "image_any")]
                match Self::image_decode(&full[..], fmt, downscale, orientation) {
                    Ok(img) => match Self::convert_decoded(img, mask, density, icc_profile, downscale, &resizer) {
                        Ok((pixels, size, density, is_opaque, is_mask)) => {
                            let _ = app_sender.send(AppEvent::ImageLoaded(ImageLoadedData::new(
                                id, size, density, is_opaque, is_mask, pixels,
                            )));
                        }
                        Err(e) => {
                            let _ = app_sender.send(AppEvent::Notify(Event::ImageLoadError {
                                image: id,
                                error: e.to_txt(),
                            }));
                        }
                    },
                    Err(e) => {
                        let _ = app_sender.send(AppEvent::Notify(Event::ImageLoadError {
                            image: id,
                            error: e.to_txt(),
                        }));
                    }
                }
            } else if !is_encoded {
                let pixels = match IpcBytes::from_vec_blocking(full) {
                    Ok(p) => p,
                    Err(e) => {
                        let _ = app_sender.send(AppEvent::Notify(Event::ImageLoadError {
                            image: id,
                            error: e.to_txt(),
                        }));
                        return;
                    }
                };
                let is_opaque = pixels.chunks_exact(4).all(|c| c[3] == 255);
                let _ = app_sender.send(AppEvent::ImageLoaded(ImageLoadedData::new(
                    id,
                    size.unwrap(),
                    density,
                    is_opaque,
                    false,
                    pixels,
                )));
            } else {
                let _ = app_sender.send(AppEvent::Notify(Event::ImageLoadError {
                    image: id,
                    error: Txt::from_static("unknown format"),
                }));
            }
        });
        id
    }

    pub fn forget(&mut self, id: ImageId) {
        self.images.remove(&id);
    }

    pub fn get(&self, id: ImageId) -> Option<&Image> {
        self.images.get(&id)
    }

    /// Called after receive and decode completes correctly.
    pub(crate) fn loaded(&mut self, data: ImageLoadedData) {
        self.images.insert(
            data.id,
            Image(Arc::new(ImageData::RawData {
                size: data.size,
                range: 0..data.pixels.len(),
                pixels: data.pixels.clone(),
                is_opaque: data.is_opaque,
                density: data.density,
            })),
        );

        let _ = self.app_sender.send(AppEvent::Notify(Event::ImageLoaded(data)));
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
}
