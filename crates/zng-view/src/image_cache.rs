#![cfg_attr(not(feature = "image_any"), allow(unused))]

#[cfg(feature = "image_any")]
use image::ImageDecoder as _;
use std::{fmt, sync::Arc};
use zng_task::parking_lot::Mutex;
use zng_view_api::image::ImageFormat;

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

pub(crate) const FORMATS: &[ImageFormat] = &[
    #[cfg(any(feature = "image_avif", zng_view_image_has_avif))]
    ImageFormat::from_static("AVIF", "avif", "avif", false),
    #[cfg(feature = "image_bmp")]
    ImageFormat::from_static("BMP", "bmp", "bmp,dib", true),
    #[cfg(feature = "image_dds")]
    ImageFormat::from_static("DirectDraw Surface", "x-direct-draw-surface", "dds", false),
    #[cfg(feature = "image_exr")]
    ImageFormat::from_static("OpenEXR", "x-exr", "exr", false),
    #[cfg(feature = "image_ff")]
    ImageFormat::from_static("Farbfeld", "farbfeld", "ff,farbfeld", false),
    #[cfg(feature = "image_gif")]
    ImageFormat::from_static("GIF", "gif", "gif", false),
    #[cfg(feature = "image_hdr")]
    ImageFormat::from_static("Radiance HDR", "vnd.radiance", "hdr", false),
    #[cfg(feature = "image_ico")]
    ImageFormat::from_static("ICO", "x-icon", "ico", true),
    #[cfg(feature = "image_jpeg")]
    ImageFormat::from_static("JPEG", "jpeg", "jpg,jpeg", true),
    #[cfg(feature = "image_png")]
    ImageFormat::from_static("PNG", "png", "png", true),
    #[cfg(feature = "image_pnm")]
    ImageFormat::from_static("PNM", "x-portable-bitmap", "pbm,pgm,ppm,pam", false),
    // https://github.com/phoboslab/qoi/issues/167
    #[cfg(feature = "image_qoi")]
    ImageFormat::from_static("QOI", "x-qoi", "qoi", true),
    #[cfg(feature = "image_tga")]
    ImageFormat::from_static("TGA", "x-tga", "tga,icb,vda,vst", false),
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

    pub fn resizer_cache(&self) -> Arc<ResizerCache> {
        self.resizer.clone()
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
                mipmap: Mutex::new(Box::new([])),
                stripes: Mutex::new(Box::new([])),
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
pub(crate) enum MipImage {
    None(PxSize),
    Generating,
    Generated(Image),
}
pub(crate) enum ImageData {
    RawData {
        size: PxSize,
        pixels: IpcBytes,
        is_opaque: bool,
        density: Option<PxDensity2d>,
        range: std::ops::Range<usize>,
        // each entry is half size of the previous
        mipmap: Mutex<Box<[MipImage]>>,
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

    /// Mipmap query and generation. Returns the best downscaled image that is greater than `image_size`.
    pub fn mip(&self, image_size: PxSize, resizer: &Arc<ResizerCache>) -> Image {
        match &*self.0 {
            ImageData::RawData { size, mipmap, .. } => {
                const MIN_MIP: Px = Px(256);

                let mip0_size = *size / Px(2);
                if image_size.width > mip0_size.width
                    || image_size.height > mip0_size.height
                    || mip0_size.width < MIN_MIP
                    || mip0_size.height < MIN_MIP
                {
                    // cannot downscale from first mip to target size
                    return self.clone();
                }

                fn iter_mips(mut size: PxSize) -> impl Iterator<Item = PxSize> {
                    std::iter::from_fn(move || {
                        size /= Px(2);
                        if size.width < MIN_MIP || size.height < MIN_MIP {
                            None
                        } else {
                            Some(size)
                        }
                    })
                }

                let mut mipmap = mipmap.lock();
                if mipmap.is_empty() {
                    *mipmap = iter_mips(*size).map(MipImage::None).collect();
                }

                let mut best_mip_i = 0;
                for (i, mip_size) in iter_mips(*size).enumerate() {
                    if image_size.width <= mip_size.width && image_size.height <= mip_size.height {
                        best_mip_i = i;
                    } else {
                        break;
                    }
                }

                let mut best_img = self;
                for mip in mipmap[..=best_mip_i].iter().rev() {
                    if let MipImage::Generated(img) = mip {
                        best_img = img;
                        break;
                    }
                }
                let best_img = best_img.clone();

                if let MipImage::None(best_mip_size) = &mipmap[best_mip_i] {
                    let best_mip_size = *best_mip_size;
                    let self_ = self.clone();
                    let source_img = best_img.clone();
                    let resizer = resizer.clone();
                    zng_task::spawn_wait(move || self_.generate_mip(source_img, best_mip_size, best_mip_i, resizer));
                    mipmap[best_mip_i] = MipImage::Generating;
                }

                best_img
            }
            _ => unreachable!(),
        }
    }
    fn generate_mip(self, source_img: Image, mip_size: PxSize, mip_i: usize, resizer: Arc<ResizerCache>) {
        if let Err(e) = self.try_generate_mip(source_img, mip_size, mip_i, resizer) {
            tracing::error!("cannot generate resized image, {e}");
            match &*self.0 {
                ImageData::RawData { mipmap, .. } => {
                    mipmap.lock()[mip_i] = MipImage::None(mip_size);
                }
                _ => unreachable!(),
            }
        }
    }
    fn try_generate_mip(&self, source_img: Image, mip_size: PxSize, mip_i: usize, resizer: Arc<ResizerCache>) -> std::io::Result<()> {
        use fast_image_resize as fr;

        let px_type = if source_img.0.is_mask() {
            fr::PixelType::U8
        } else {
            fr::PixelType::U8x4
        };
        let source_size = source_img.size();
        let source_pixels = source_img.pixels();
        let source = fr::images::ImageRef::new(source_size.width.0 as _, source_size.height.0 as _, source_pixels, px_type).unwrap();

        let mut dest_buf = IpcBytes::new_mut_blocking(mip_size.width.0 as usize * mip_size.height.0 as usize * px_type.size())?;
        let mut dest = fr::images::Image::from_slice_u8(mip_size.width.0 as _, mip_size.height.0 as _, &mut dest_buf[..], px_type).unwrap();

        let mut resize_opt = fr::ResizeOptions::new();
        // is already pre multiplied
        resize_opt.mul_div_alpha = false;
        // default, best quality
        resize_opt.algorithm = fr::ResizeAlg::Convolution(fr::FilterType::Lanczos3);
        // try to reuse cache
        match resizer.try_lock() {
            Some(mut r) => r.resize(&source, &mut dest, Some(&resize_opt)),
            None => fr::Resizer::new().resize(&source, &mut dest, Some(&resize_opt)),
        }
        .unwrap();

        let pixels = dest_buf.finish_blocking()?;
        let mip = Image(Arc::new(ImageData::RawData {
            size: mip_size,
            range: 0..pixels.len(),
            pixels,
            is_opaque: self.is_opaque(),
            density: None,
            mipmap: Mutex::new(Box::new([])),
            stripes: Mutex::new(Box::new([])),
        }));

        match &*self.0 {
            ImageData::RawData { mipmap, .. } => {
                mipmap.lock()[mip_i] = MipImage::Generated(mip);
            }
            _ => unreachable!(),
        }

        Ok(())
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
                mipmap: Mutex::new(Box::new([])),
                stripes: Mutex::new(Box::new([])),
            }));

            stripes.push(stripe);
        }

        stripes.into_boxed_slice()
    }
}
