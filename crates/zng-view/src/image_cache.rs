use std::{fmt, sync::Arc};

use webrender::api::{ImageDescriptor, ImageDescriptorFlags, ImageFormat};
use winit::{
    event_loop::ActiveEventLoop,
    window::{CustomCursor, Icon},
};
use zng_txt::{formatx, ToTxt, Txt};
use zng_unit::{Px, PxPoint, PxSize};
use zng_view_api::{
    image::{ImageDataFormat, ImageDownscale, ImageId, ImageLoadedData, ImageMaskMode, ImagePpi, ImageRequest},
    ipc::{IpcBytes, IpcBytesReceiver},
    Event,
};

use crate::{AppEvent, AppEventSender};
use rustc_hash::FxHashMap;

#[allow(unexpected_cfgs)]
pub(crate) const ENCODERS: &[&str] = &[
    "png",
    "jpg",
    "jpeg",
    "webp",
    #[cfg(feature = "avif")]
    "avif",
    "gif",
    "ico",
    "bmp",
    "jfif",
    "exr",
    "hdr",
    "pnm",
    "qoi",
    "ff",
    "farbfeld",
];
#[allow(unexpected_cfgs)]
pub(crate) const DECODERS: &[&str] = &[
    "png",
    "jpg",
    "jpeg",
    "webp",
    #[cfg(feature = "avif")]
    "avif",
    "gif",
    "ico",
    "bmp",
    "jfif",
    "exr",
    "pnm",
    "qoi",
    "ff",
    "farbfeld",
    "dds",
];

/// Decode and cache image resources.
pub(crate) struct ImageCache {
    app_sender: AppEventSender,
    images: FxHashMap<ImageId, Image>,
    image_id_gen: ImageId,
}
impl ImageCache {
    pub fn new(app_sender: AppEventSender) -> Self {
        Self {
            app_sender,
            images: FxHashMap::default(),
            image_id_gen: ImageId::first(),
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
        }: ImageRequest<IpcBytes>,
    ) -> ImageId {
        let id = self.image_id_gen.incr();

        let app_sender = self.app_sender.clone();
        rayon::spawn(move || {
            let r = match format {
                ImageDataFormat::Bgra8 { size, ppi } => {
                    let expected_len = size.width.0 as usize * size.height.0 as usize * 4;
                    if data.len() != expected_len {
                        Err(formatx!(
                            "pixels.len() is not width * height * 4, expected {expected_len}, found {}",
                            data.len()
                        ))
                    } else if mask.is_some() {
                        let (pixels, size, _, is_opaque, _) = Self::convert_decoded(
                            image::DynamicImage::ImageLuma8(
                                image::ImageBuffer::from_raw(size.width.0 as _, size.height.0 as _, data.to_vec()).unwrap(),
                            ),
                            mask,
                        );
                        Ok((pixels, size, ppi, is_opaque, true))
                    } else {
                        let is_opaque = data.chunks_exact(4).all(|c| c[3] == 255);
                        Ok((data, size, ppi, is_opaque, false))
                    }
                }
                ImageDataFormat::A8 { size } => {
                    let expected_len = size.width.0 as usize * size.height.0 as usize;
                    if data.len() != expected_len {
                        Err(formatx!(
                            "pixels.len() is not width * height, expected {expected_len}, found {}",
                            data.len()
                        ))
                    } else if mask.is_none() {
                        let (pixels, size, _, is_opaque, _) = Self::convert_decoded(
                            image::DynamicImage::ImageLuma8(
                                image::ImageBuffer::from_raw(size.width.0 as _, size.height.0 as _, data.to_vec()).unwrap(),
                            ),
                            None,
                        );
                        Ok((pixels, size, None, is_opaque, false))
                    } else {
                        let is_opaque = data.iter().all(|&c| c == 255);
                        Ok((data, size, None, is_opaque, true))
                    }
                }
                fmt => match Self::get_format_and_size(&fmt, &data[..]) {
                    Ok((fmt, mut size)) => {
                        let decoded_len = size.width.0 as u64 * size.height.0 as u64 * 4;
                        if decoded_len > max_decoded_len {
                            Err(formatx!(
                                "image {size:?} needs to allocate {decoded_len} bytes, but max allowed size is {max_decoded_len} bytes",
                            ))
                        } else {
                            if let Some(d) = downscale {
                                size = d.resize_dimensions(size);
                            }
                            let _ = app_sender.send(AppEvent::Notify(Event::ImageMetadataLoaded {
                                image: id,
                                size,
                                ppi: None,
                                is_mask: false,
                            }));
                            match Self::image_decode(&data[..], fmt, downscale) {
                                Ok(img) => Ok(Self::convert_decoded(img, mask)),
                                Err(e) => Err(e.to_txt()),
                            }
                        }
                    }
                    Err(e) => Err(e),
                },
            };

            match r {
                Ok((pixels, size, ppi, is_opaque, is_mask)) => {
                    let _ = app_sender.send(AppEvent::ImageLoaded(ImageLoadedData {
                        id,
                        pixels,
                        size,
                        ppi,
                        is_opaque,
                        is_mask,
                    }));
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
            format,
            data,
            max_decoded_len,
            downscale,
            mask,
        }: ImageRequest<IpcBytesReceiver>,
    ) -> ImageId {
        let id = self.image_id_gen.incr();
        let app_sender = self.app_sender.clone();
        rayon::spawn(move || {
            // crate `images` does not do progressive decode.
            let mut full = vec![];
            let mut size = None;
            let mut ppi = None;
            let mut is_encoded = true;

            let mut format = match format {
                ImageDataFormat::Bgra8 { size: s, ppi: p } => {
                    is_encoded = false;
                    size = Some(s);
                    ppi = p;
                    None
                }
                ImageDataFormat::A8 { size: s } => {
                    is_encoded = false;
                    size = Some(s);
                    None
                }
                ImageDataFormat::FileExtension(ext) => image::ImageFormat::from_extension(ext.as_str()),
                ImageDataFormat::MimeType(t) => t.strip_prefix("image/").and_then(image::ImageFormat::from_extension),
                ImageDataFormat::Unknown => None,
            };

            let mut pending = true;
            while pending {
                match data.recv() {
                    Ok(d) => {
                        pending = !d.is_empty();

                        full.extend(d);

                        if let Some(fmt) = format {
                            if size.is_none() {
                                size = image::ImageReader::with_format(std::io::Cursor::new(&full), fmt)
                                    .into_dimensions()
                                    .ok()
                                    .map(|(w, h)| PxSize::new(Px(w as i32), Px(h as i32)));
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
                match Self::image_decode(&full[..], fmt, downscale) {
                    Ok(img) => {
                        let (pixels, size, ppi, is_opaque, is_mask) = Self::convert_decoded(img, mask);
                        let _ = app_sender.send(AppEvent::ImageLoaded(ImageLoadedData {
                            id,
                            pixels,
                            size,
                            ppi,
                            is_opaque,
                            is_mask,
                        }));
                    }
                    Err(e) => {
                        let _ = app_sender.send(AppEvent::Notify(Event::ImageLoadError {
                            image: id,
                            error: e.to_txt(),
                        }));
                    }
                }
            } else if !is_encoded {
                let pixels = IpcBytes::from_vec(full);
                let is_opaque = pixels.chunks_exact(4).all(|c| c[3] == 255);
                let _ = app_sender.send(AppEvent::ImageLoaded(ImageLoadedData {
                    id,
                    pixels,
                    size: size.unwrap(),
                    ppi,
                    is_opaque,
                    is_mask: false,
                }));
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
        let mut flags = ImageDescriptorFlags::empty(); //ImageDescriptorFlags::ALLOW_MIPMAPS;
        if data.is_opaque {
            flags |= ImageDescriptorFlags::IS_OPAQUE
        }

        self.images.insert(
            data.id,
            Image(Arc::new(ImageData::RawData {
                size: data.size,
                pixels: data.pixels.clone(),
                descriptor: ImageDescriptor::new(
                    data.size.width.0,
                    data.size.height.0,
                    if data.is_mask { ImageFormat::R8 } else { ImageFormat::BGRA8 },
                    flags,
                ),
                ppi: data.ppi,
            })),
        );

        let _ = self.app_sender.send(AppEvent::Notify(Event::ImageLoaded(data)));
    }

    fn get_format_and_size(fmt: &ImageDataFormat, data: &[u8]) -> Result<(image::ImageFormat, PxSize), Txt> {
        let fmt = match fmt {
            ImageDataFormat::FileExtension(ext) => image::ImageFormat::from_extension(ext.as_str()),
            ImageDataFormat::MimeType(t) => t.strip_prefix("image/").and_then(image::ImageFormat::from_extension),
            ImageDataFormat::Unknown => None,
            ImageDataFormat::Bgra8 { .. } => unreachable!(),
            ImageDataFormat::A8 { .. } => unreachable!(),
        };

        let reader = match fmt {
            Some(fmt) => image::ImageReader::with_format(std::io::Cursor::new(data), fmt),
            None => image::ImageReader::new(std::io::Cursor::new(data))
                .with_guessed_format()
                .map_err(|e| e.to_txt())?,
        };

        match reader.format() {
            Some(fmt) => {
                let (w, h) = reader.into_dimensions().map_err(|e| e.to_string())?;
                Ok((fmt, PxSize::new(Px(w as i32), Px(h as i32))))
            }
            None => Err(Txt::from_static("unknown format")),
        }
    }

    fn image_decode(buf: &[u8], format: image::ImageFormat, downscale: Option<ImageDownscale>) -> image::ImageResult<image::DynamicImage> {
        let buf = std::io::Cursor::new(buf);

        let mut reader = image::ImageReader::new(buf);
        reader.set_format(format);
        reader.no_limits();
        let mut image = reader.decode()?;

        if let Some(s) = downscale {
            let (img_w, img_h) = (image.width(), image.height());
            match s {
                ImageDownscale::Fit(s) => {
                    let w = img_w.min(s.width.0 as u32);
                    let h = img_h.min(s.height.0 as u32);
                    if w != img_w || h != img_h {
                        image = image.resize(w, h, image::imageops::FilterType::Triangle);
                    }
                }
                ImageDownscale::Fill(s) => {
                    let w = img_w.min(s.width.0 as u32);
                    let h = img_h.min(s.height.0 as u32);
                    if w != img_w && h != img_h {
                        image = image.resize_to_fill(w, h, image::imageops::FilterType::Triangle);
                    }
                }
            }
        }

        Ok(image)
    }

    fn convert_decoded(image: image::DynamicImage, mask: Option<ImageMaskMode>) -> RawLoadedImg {
        use image::DynamicImage::*;

        let mut is_opaque = true;

        let (size, pixels) = match image {
            ImageLuma8(img) => (
                img.dimensions(),
                if mask.is_some() {
                    let r = img.into_raw();
                    is_opaque = !r.iter().any(|&a| a < 255);
                    r
                } else {
                    img.into_raw().into_iter().flat_map(|l| [l, l, l, 255]).collect()
                },
            ),
            ImageLumaA8(img) => (
                img.dimensions(),
                if let Some(mask) = mask {
                    match mask {
                        ImageMaskMode::A => img
                            .into_raw()
                            .chunks(2)
                            .map(|la| {
                                if la[1] < 255 {
                                    is_opaque = false;
                                }
                                la[1]
                            })
                            .collect(),
                        ImageMaskMode::B | ImageMaskMode::G | ImageMaskMode::R | ImageMaskMode::Luminance => img
                            .into_raw()
                            .chunks(2)
                            .map(|la| {
                                if la[0] < 255 {
                                    is_opaque = false;
                                }
                                la[0]
                            })
                            .collect(),
                    }
                } else {
                    img.into_raw()
                        .chunks(2)
                        .flat_map(|la| {
                            if la[1] < 255 {
                                is_opaque = false;
                                let l = la[0] as f32 * la[1] as f32 / 255.0;
                                let l = l as u8;
                                [l, l, l, la[1]]
                            } else {
                                let l = la[0];
                                [l, l, l, la[1]]
                            }
                        })
                        .collect()
                },
            ),
            ImageRgb8(img) => (
                img.dimensions(),
                if let Some(mask) = mask {
                    match mask {
                        ImageMaskMode::Luminance | ImageMaskMode::A => img
                            .into_raw()
                            .chunks(3)
                            .map(|c| {
                                let c = luminance(c);
                                if c < 255 {
                                    is_opaque = false;
                                }
                                c
                            })
                            .collect(),
                        mask => {
                            let channel = match mask {
                                ImageMaskMode::B => 2,
                                ImageMaskMode::G => 1,
                                ImageMaskMode::R => 0,
                                _ => unreachable!(),
                            };
                            img.into_raw()
                                .chunks(3)
                                .map(|c| {
                                    let c = c[channel];
                                    if c < 255 {
                                        is_opaque = false;
                                    }
                                    c
                                })
                                .collect()
                        }
                    }
                } else {
                    img.into_raw().chunks(3).flat_map(|c| [c[2], c[1], c[0], 255]).collect()
                },
            ),
            ImageRgba8(img) => (
                img.dimensions(),
                if let Some(mask) = mask {
                    match mask {
                        ImageMaskMode::Luminance => img
                            .into_raw()
                            .chunks(4)
                            .map(|c| {
                                let c = luminance(&c[..3]);
                                if c < 255 {
                                    is_opaque = false;
                                }
                                c
                            })
                            .collect(),
                        mask => {
                            let channel = match mask {
                                ImageMaskMode::A => 3,
                                ImageMaskMode::B => 2,
                                ImageMaskMode::G => 1,
                                ImageMaskMode::R => 0,
                                _ => unreachable!(),
                            };
                            img.into_raw()
                                .chunks(4)
                                .map(|c| {
                                    let c = c[channel];
                                    if c < 255 {
                                        is_opaque = false;
                                    }
                                    c
                                })
                                .collect()
                        }
                    }
                } else {
                    let mut buf = img.into_raw();
                    buf.chunks_mut(4).for_each(|c| {
                        if c[3] < 255 {
                            is_opaque = false;
                            let a = c[3] as f32 / 255.0;
                            c[0..3].iter_mut().for_each(|c| *c = (*c as f32 * a) as u8);
                        }
                        c.swap(0, 2);
                    });
                    buf
                },
            ),
            ImageLuma16(img) => (
                img.dimensions(),
                if mask.is_some() {
                    img.into_raw()
                        .into_iter()
                        .map(|l| {
                            let l = (l as f32 / u16::MAX as f32 * 255.0) as u8;
                            if l < 255 {
                                is_opaque = false;
                            }
                            l
                        })
                        .collect()
                } else {
                    img.into_raw()
                        .into_iter()
                        .flat_map(|l| {
                            let l = (l as f32 / u16::MAX as f32 * 255.0) as u8;
                            [l, l, l, 255]
                        })
                        .collect()
                },
            ),
            ImageLumaA16(img) => (
                img.dimensions(),
                if let Some(mask) = mask {
                    match mask {
                        ImageMaskMode::A => img
                            .into_raw()
                            .chunks(2)
                            .map(|la| {
                                if la[1] < u16::MAX {
                                    is_opaque = false;
                                }
                                let max = u16::MAX as f32;
                                let l = la[1] as f32 / max * 255.0;
                                l as u8
                            })
                            .collect(),
                        ImageMaskMode::B | ImageMaskMode::G | ImageMaskMode::R | ImageMaskMode::Luminance => img
                            .into_raw()
                            .chunks(2)
                            .map(|la| {
                                if la[0] < u16::MAX {
                                    is_opaque = false;
                                }
                                let max = u16::MAX as f32;
                                let l = la[0] as f32 / max * 255.0;
                                l as u8
                            })
                            .collect(),
                    }
                } else {
                    img.into_raw()
                        .chunks(2)
                        .flat_map(|la| {
                            let max = u16::MAX as f32;
                            let l = la[0] as f32 / max;
                            let a = la[1] as f32 / max * 255.0;

                            if la[1] < u16::MAX {
                                is_opaque = false;
                                let l = (l * a) as u8;
                                [l, l, l, a as u8]
                            } else {
                                let l = (l * 255.0) as u8;
                                [l, l, l, a as u8]
                            }
                        })
                        .collect()
                },
            ),
            ImageRgb16(img) => (
                img.dimensions(),
                if let Some(mask) = mask {
                    match mask {
                        ImageMaskMode::Luminance | ImageMaskMode::A => img
                            .into_raw()
                            .chunks(3)
                            .map(|c| {
                                let c = luminance_16(c);
                                if c < 255 {
                                    is_opaque = false;
                                }
                                c
                            })
                            .collect(),
                        mask => {
                            let channel = match mask {
                                ImageMaskMode::B => 2,
                                ImageMaskMode::G => 1,
                                ImageMaskMode::R => 0,
                                _ => unreachable!(),
                            };
                            img.into_raw()
                                .chunks(3)
                                .map(|c| {
                                    let c = c[channel];
                                    if c < u16::MAX {
                                        is_opaque = false;
                                    }

                                    (c as f32 / u16::MAX as f32 * 255.0) as u8
                                })
                                .collect()
                        }
                    }
                } else {
                    img.into_raw()
                        .chunks(3)
                        .flat_map(|c| {
                            let to_u8 = 255.0 / u16::MAX as f32;
                            [
                                (c[2] as f32 * to_u8) as u8,
                                (c[1] as f32 * to_u8) as u8,
                                (c[0] as f32 * to_u8) as u8,
                                255,
                            ]
                        })
                        .collect()
                },
            ),
            ImageRgba16(img) => (
                img.dimensions(),
                if let Some(mask) = mask {
                    match mask {
                        ImageMaskMode::Luminance => img
                            .into_raw()
                            .chunks(4)
                            .map(|c| {
                                let c = luminance_16(&c[..3]);
                                if c < 255 {
                                    is_opaque = false;
                                }
                                c
                            })
                            .collect(),
                        mask => {
                            let channel = match mask {
                                ImageMaskMode::A => 3,
                                ImageMaskMode::B => 2,
                                ImageMaskMode::G => 1,
                                ImageMaskMode::R => 0,
                                _ => unreachable!(),
                            };
                            img.into_raw()
                                .chunks(4)
                                .map(|c| {
                                    let c = c[channel];
                                    if c < 255 {
                                        is_opaque = false;
                                    }
                                    (c as f32 / u16::MAX as f32 * 255.0) as u8
                                })
                                .collect()
                        }
                    }
                } else {
                    img.into_raw()
                        .chunks(4)
                        .flat_map(|c| {
                            if c[3] < u16::MAX {
                                is_opaque = false;
                                let max = u16::MAX as f32;
                                let a = c[3] as f32 / max * 255.0;
                                [
                                    (c[2] as f32 / max * a) as u8,
                                    (c[1] as f32 / max * a) as u8,
                                    (c[0] as f32 / max * a) as u8,
                                    a as u8,
                                ]
                            } else {
                                let to_u8 = 255.0 / u16::MAX as f32;
                                [
                                    (c[2] as f32 * to_u8) as u8,
                                    (c[1] as f32 * to_u8) as u8,
                                    (c[0] as f32 * to_u8) as u8,
                                    255,
                                ]
                            }
                        })
                        .collect()
                },
            ),
            ImageRgb32F(img) => (
                img.dimensions(),
                if let Some(mask) = mask {
                    match mask {
                        ImageMaskMode::Luminance | ImageMaskMode::A => img
                            .into_raw()
                            .chunks(3)
                            .map(|c| {
                                let c = luminance_f32(c);
                                if c < 255 {
                                    is_opaque = false;
                                }
                                c
                            })
                            .collect(),
                        mask => {
                            let channel = match mask {
                                ImageMaskMode::B => 2,
                                ImageMaskMode::G => 1,
                                ImageMaskMode::R => 0,
                                _ => unreachable!(),
                            };
                            img.into_raw()
                                .chunks(3)
                                .map(|c| {
                                    let c = (c[channel] * 255.0) as u8;
                                    if c < 255 {
                                        is_opaque = false;
                                    }
                                    c
                                })
                                .collect()
                        }
                    }
                } else {
                    img.into_raw()
                        .chunks(3)
                        .flat_map(|c| [(c[2] * 255.0) as u8, (c[1] * 255.0) as u8, (c[0] * 255.0) as u8, 255])
                        .collect()
                },
            ),
            ImageRgba32F(img) => (
                img.dimensions(),
                if let Some(mask) = mask {
                    match mask {
                        ImageMaskMode::Luminance => img
                            .into_raw()
                            .chunks(4)
                            .map(|c| {
                                let c = luminance_f32(&c[..3]);
                                if c < 255 {
                                    is_opaque = false;
                                }
                                c
                            })
                            .collect(),
                        mask => {
                            let channel = match mask {
                                ImageMaskMode::A => 3,
                                ImageMaskMode::B => 2,
                                ImageMaskMode::G => 1,
                                ImageMaskMode::R => 0,
                                _ => unreachable!(),
                            };
                            img.into_raw()
                                .chunks(4)
                                .map(|c| {
                                    let c = (c[channel] * 255.0) as u8;
                                    if c < 255 {
                                        is_opaque = false;
                                    }
                                    c
                                })
                                .collect()
                        }
                    }
                } else {
                    img.into_raw()
                        .chunks(4)
                        .flat_map(|c| {
                            if c[3] < 1.0 {
                                is_opaque = false;
                                let a = c[3] * 255.0;
                                [(c[2] * a) as u8, (c[1] * a) as u8, (c[0] * a) as u8, a as u8]
                            } else {
                                [(c[2] * 255.0) as u8, (c[1] * 255.0) as u8, (c[0] * 255.0) as u8, 255]
                            }
                        })
                        .collect()
                },
            ),
            _ => unreachable!(),
        };

        (
            IpcBytes::from_vec(pixels),
            PxSize::new(Px(size.0 as i32), Px(size.1 as i32)),
            None,
            is_opaque,
            mask.is_some(), // is_mask
        )
    }

    pub fn encode(&self, id: ImageId, format: Txt) {
        if !ENCODERS.contains(&format.as_str()) {
            let error = formatx!("cannot encode `{id:?}` to `{format}`, unknown format");
            let _ = self
                .app_sender
                .send(AppEvent::Notify(Event::ImageEncodeError { image: id, format, error }));
            return;
        }

        if let Some(img) = self.get(id) {
            let fmt = image::ImageFormat::from_extension(format.as_str()).unwrap();
            debug_assert!(fmt.can_write());

            let img = img.clone();
            let sender = self.app_sender.clone();
            rayon::spawn(move || {
                let mut data = vec![];
                match img.encode(fmt, &mut data) {
                    Ok(_) => {
                        let _ = sender.send(AppEvent::Notify(Event::ImageEncoded {
                            image: id,
                            format,
                            data: IpcBytes::from_vec(data),
                        }));
                    }
                    Err(e) => {
                        let error = formatx!("failed to encode `{id:?}` to `{format}`, {e}");
                        let _ = sender.send(AppEvent::Notify(Event::ImageEncodeError { image: id, format, error }));
                    }
                }
            })
        } else {
            let error = formatx!("cannot encode `{id:?}` to `{format}`, image not found");
            let _ = self
                .app_sender
                .send(AppEvent::Notify(Event::ImageEncodeError { image: id, format, error }));
        }
    }

    pub(crate) fn on_low_memory(&mut self) {
        // app-process controls this cache
    }

    pub(crate) fn clear(&mut self) {
        self.images.clear();
    }
}

/// (pixels, size, ppi, is_opaque, is_mask)
type RawLoadedImg = (IpcBytes, PxSize, Option<ImagePpi>, bool, bool);
pub(crate) enum ImageData {
    RawData {
        size: PxSize,
        pixels: IpcBytes,
        descriptor: ImageDescriptor,
        ppi: Option<ImagePpi>,
    },
    NativeTexture {
        uv: webrender::api::units::TexelRect,
        texture: gleam::gl::GLuint,
    },
}
impl ImageData {
    pub fn is_opaque(&self) -> bool {
        match self {
            ImageData::RawData { descriptor, .. } => descriptor.flags.contains(ImageDescriptorFlags::IS_OPAQUE),
            ImageData::NativeTexture { .. } => false,
        }
    }

    pub fn is_mask(&self) -> bool {
        match self {
            ImageData::RawData { descriptor, .. } => descriptor.format == ImageFormat::R8,
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
                descriptor,
                ppi,
            } => f
                .debug_struct("Image")
                .field("size", size)
                .field("descriptor", descriptor)
                .field("ppi", ppi)
                .field("pixels", &format_args!("<{} shared bytes>", pixels.len()))
                .finish(),
            ImageData::NativeTexture { .. } => unreachable!(),
        }
    }
}
impl Image {
    pub fn descriptor(&self) -> ImageDescriptor {
        match &*self.0 {
            ImageData::RawData { descriptor, .. } => *descriptor,
            ImageData::NativeTexture { .. } => unreachable!(),
        }
    }

    /// Generate a window icon from the image.
    pub fn icon(&self) -> Option<Icon> {
        let (size, pixels) = match &*self.0 {
            ImageData::RawData { size, pixels, .. } => (size, pixels),
            ImageData::NativeTexture { .. } => unreachable!(),
        };

        let width = size.width.0 as u32;
        let height = size.height.0 as u32;
        if width == 0 || height == 0 || self.0.is_mask() {
            None
        } else {
            let r = if width > 255 || height > 255 {
                // resize to max 255
                let mut buf = pixels[..].to_vec();
                // BGRA to RGBA
                buf.chunks_exact_mut(4).for_each(|c| c.swap(0, 2));
                let img = image::ImageBuffer::from_raw(width, height, buf).unwrap();
                let img = image::DynamicImage::ImageRgba8(img);
                let img = img.resize(255, 255, image::imageops::FilterType::Lanczos3);

                use image::GenericImageView;
                let (width, height) = img.dimensions();
                let buf = img.into_rgba8().into_raw();
                winit::window::Icon::from_rgba(buf, width, height)
            } else {
                let mut buf = pixels[..].to_vec();
                // BGRA to RGBA
                buf.chunks_exact_mut(4).for_each(|c| c.swap(0, 2));
                winit::window::Icon::from_rgba(buf, width, height)
            };
            match r {
                Ok(i) => Some(i),
                Err(e) => {
                    tracing::error!("failed to convert image to custom icon, {e}");
                    None
                }
            }
        }
    }

    /// Generate a cursor from the image.
    pub fn cursor(&self, hotspot: PxPoint, event_loop: &ActiveEventLoop) -> Option<CustomCursor> {
        let (size, pixels) = match &*self.0 {
            ImageData::RawData { size, pixels, .. } => (size, pixels),
            ImageData::NativeTexture { .. } => unreachable!(),
        };

        let width: u16 = size.width.0.try_into().ok()?;
        let height: u16 = size.height.0.try_into().ok()?;
        let hotspot_x: u16 = hotspot.x.0.try_into().ok()?;
        let hotspot_y: u16 = hotspot.y.0.try_into().ok()?;

        if width == 0 || height == 0 || hotspot_x > width || hotspot_y > height || self.0.is_mask() {
            None
        } else {
            let mut buf = pixels[..].to_vec();
            // BGRA to RGBA
            buf.chunks_exact_mut(4).for_each(|c| c.swap(0, 2));
            match CustomCursor::from_rgba(buf, width, height, hotspot_x, hotspot_y) {
                Ok(c) => Some(event_loop.create_custom_cursor(c)),
                Err(e) => {
                    tracing::error!("failed to convert image to custom cursor, {e}");
                    None
                }
            }
        }
    }

    pub fn encode(&self, format: image::ImageFormat, buffer: &mut Vec<u8>) -> image::ImageResult<()> {
        let (size, pixels, ppi) = match &*self.0 {
            ImageData::RawData { size, pixels, ppi, .. } => (size, pixels, ppi),
            ImageData::NativeTexture { .. } => unreachable!(),
        };

        if size.width <= Px(0) || size.height <= Px(0) {
            return Err(image::ImageError::IoError(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "cannot encode zero sized image",
            )));
        }

        use image::*;

        if self.0.is_mask() {
            let width = size.width.0 as u32;
            let height = size.height.0 as u32;
            let is_opaque = self.0.is_opaque();
            let r8 = pixels[..].to_vec();

            let mut img = image::DynamicImage::ImageLuma8(image::ImageBuffer::from_raw(width, height, r8).unwrap());
            if is_opaque {
                img = image::DynamicImage::ImageRgb8(img.to_rgb8());
            }
            img.write_to(&mut std::io::Cursor::new(buffer), format)?;

            return Ok(());
        }

        // invert rows, `image` only supports top-to-bottom buffers.
        let mut buf = pixels[..].to_vec();
        // BGRA to RGBA
        buf.chunks_exact_mut(4).for_each(|c| c.swap(0, 2));
        let rgba = buf;

        let width = size.width.0 as u32;
        let height = size.height.0 as u32;
        let is_opaque = self.0.is_opaque();

        match format {
            ImageFormat::Jpeg => {
                let mut jpg = codecs::jpeg::JpegEncoder::new(buffer);
                if let Some(ppi) = ppi {
                    jpg.set_pixel_density(codecs::jpeg::PixelDensity {
                        density: (ppi.x as u16, ppi.y as u16),
                        unit: codecs::jpeg::PixelDensityUnit::Inches,
                    });
                }
                jpg.encode(&rgba, width, height, ColorType::Rgba8.into())?;
            }
            ImageFormat::Png => {
                let mut img = image::DynamicImage::ImageRgba8(image::ImageBuffer::from_raw(width, height, rgba).unwrap());
                if is_opaque {
                    img = image::DynamicImage::ImageRgb8(img.to_rgb8());
                }
                if let Some(ppi) = ppi {
                    let mut png_bytes = vec![];

                    img.write_to(&mut std::io::Cursor::new(&mut png_bytes), ImageFormat::Png)?;

                    let mut png = img_parts::png::Png::from_bytes(png_bytes.into()).unwrap();

                    let chunk_kind = *b"pHYs";
                    debug_assert!(png.chunk_by_type(chunk_kind).is_none());

                    use byteorder::*;
                    let mut chunk = Vec::with_capacity(4 * 2 + 1);

                    // ppi / inch_to_metric
                    let ppm_x = (ppi.x / 0.0254) as u32;
                    let ppm_y = (ppi.y / 0.0254) as u32;

                    chunk.write_u32::<BigEndian>(ppm_x).unwrap();
                    chunk.write_u32::<BigEndian>(ppm_y).unwrap();
                    chunk.write_u8(1).unwrap(); // metric

                    let chunk = img_parts::png::PngChunk::new(chunk_kind, chunk.into());
                    png.chunks_mut().insert(1, chunk);

                    png.encoder().write_to(buffer)?;
                } else {
                    img.write_to(&mut std::io::Cursor::new(buffer), ImageFormat::Png)?;
                }
            }
            _ => {
                // other formats that we don't with custom PPI meta.

                let mut img = image::DynamicImage::ImageRgba8(image::ImageBuffer::from_raw(width, height, rgba).unwrap());
                if is_opaque {
                    img = image::DynamicImage::ImageRgb8(img.to_rgb8());
                }
                img.write_to(&mut std::io::Cursor::new(buffer), format)?;
            }
        }

        Ok(())
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
}

// Image data is provided to webrender directly from the BGRA8 shared memory.
// The `ExternalImageId` is the Arc pointer to ImageData.
mod external {
    use std::{collections::hash_map::Entry, sync::Arc};

    use rustc_hash::FxHashMap;
    use webrender::{
        api::{
            units::{ImageDirtyRect, TexelRect},
            DocumentId, ExternalImage, ExternalImageData, ExternalImageHandler, ExternalImageId, ExternalImageSource, ExternalImageType,
            ImageKey,
        },
        RenderApi,
    };
    use zng_view_api::image::ImageTextureId;

    use super::{Image, ImageData};

    /// Implements [`ExternalImageHandler`].
    ///
    /// # Safety
    ///
    /// This is only safe if use with [`ImageUseMap`].
    pub(crate) struct WrImageCache {
        locked: Vec<Arc<ImageData>>,
    }
    impl WrImageCache {
        pub fn new_boxed() -> Box<dyn ExternalImageHandler> {
            Box::new(WrImageCache { locked: vec![] })
        }
    }
    impl ExternalImageHandler for WrImageCache {
        fn lock(&mut self, key: ExternalImageId, _channel_index: u8) -> ExternalImage {
            // SAFETY: this is safe because the Arc is kept alive in `ImageUseMap`.
            let img = unsafe {
                let ptr = key.0 as *const ImageData;
                Arc::increment_strong_count(ptr);
                Arc::<ImageData>::from_raw(ptr)
            };

            self.locked.push(img); // keep alive in case the image is removed mid-use

            match &**self.locked.last().unwrap() {
                ImageData::RawData { pixels, .. } => {
                    ExternalImage {
                        uv: TexelRect::invalid(), // `RawData` does not use `uv`.
                        source: ExternalImageSource::RawData(&pixels[..]),
                    }
                }
                ImageData::NativeTexture { uv, texture: id } => ExternalImage {
                    uv: *uv,
                    source: ExternalImageSource::NativeTexture(*id),
                },
            }
        }

        fn unlock(&mut self, key: ExternalImageId, _channel_index: u8) {
            if let Some(i) = self.locked.iter().position(|d| ExternalImageId(Arc::as_ptr(d) as _) == key) {
                self.locked.swap_remove(i);
            } else {
                debug_assert!(false);
            }
        }
    }

    impl Image {
        fn external_id(&self) -> ExternalImageId {
            ExternalImageId(Arc::as_ptr(&self.0) as u64)
        }

        fn data(&self) -> webrender::api::ImageData {
            webrender::api::ImageData::External(ExternalImageData {
                id: self.external_id(),
                channel_index: 0,
                image_type: ExternalImageType::Buffer,
            })
        }
    }

    /// Track and manage images used in a renderer.
    ///
    /// The renderer must use [`WrImageCache`] as the external image source.
    #[derive(Default)]
    pub(crate) struct ImageUseMap {
        id_tex: FxHashMap<ExternalImageId, (ImageTextureId, Image)>,
        tex_id: FxHashMap<ImageTextureId, ExternalImageId>,
    }
    impl ImageUseMap {
        pub fn new_use(&mut self, image: &Image, document_id: DocumentId, api: &mut RenderApi) -> ImageTextureId {
            let id = image.external_id();
            match self.id_tex.entry(id) {
                Entry::Occupied(e) => e.get().0,
                Entry::Vacant(e) => {
                    let key = api.generate_image_key();
                    let tex_id = ImageTextureId::from_raw(key.1);
                    e.insert((tex_id, image.clone())); // keep the image Arc alive, we expect this in `WrImageCache`.
                    self.tex_id.insert(tex_id, id);

                    let mut txn = webrender::Transaction::new();
                    txn.add_image(key, image.descriptor(), image.data(), None);
                    api.send_transaction(document_id, txn);

                    tex_id
                }
            }
        }

        /// Returns if needs to update.
        pub fn update_use(&mut self, texture_id: ImageTextureId, image: &Image, document_id: DocumentId, api: &mut RenderApi) {
            if let Entry::Occupied(mut e) = self.tex_id.entry(texture_id) {
                let id = image.external_id();
                if *e.get() != id {
                    let prev_id = e.insert(id);
                    self.id_tex.remove(&prev_id).unwrap();
                    self.id_tex.insert(id, (texture_id, image.clone()));

                    let mut txn = webrender::Transaction::new();
                    txn.update_image(
                        ImageKey(api.get_namespace_id(), texture_id.get()),
                        image.descriptor(),
                        image.data(),
                        &ImageDirtyRect::All,
                    );
                    api.send_transaction(document_id, txn);
                }
            }
        }

        pub fn delete(&mut self, texture_id: ImageTextureId, document_id: DocumentId, api: &mut RenderApi) {
            if let Some(id) = self.tex_id.remove(&texture_id) {
                let _img = self.id_tex.remove(&id); // remove but keep alive until the transaction is done.
                let mut txn = webrender::Transaction::new();
                txn.delete_image(ImageKey(api.get_namespace_id(), texture_id.get()));
                api.send_transaction(document_id, txn);
            }
        }
    }
}
pub(crate) use external::{ImageUseMap, WrImageCache};

mod capture {
    use std::sync::Arc;

    use webrender::api::{ImageDescriptor, ImageDescriptorFlags, ImageFormat};
    use zng_txt::formatx;
    use zng_unit::{Factor, PxRect};
    use zng_view_api::{
        image::{ImageDataFormat, ImageId, ImageLoadedData, ImageMaskMode, ImagePpi, ImageRequest},
        ipc::IpcBytes,
        window::{FrameId, WindowId},
        Event,
    };

    use crate::{
        image_cache::{Image, ImageData},
        AppEvent,
    };

    use super::ImageCache;

    impl ImageCache {
        /// Create frame_image for an `Api::frame_image` request.
        #[allow(clippy::too_many_arguments)]
        pub fn frame_image(
            &mut self,
            gl: &dyn gleam::gl::Gl,
            rect: PxRect,
            window_id: WindowId,
            frame_id: FrameId,
            scale_factor: Factor,
            mask: Option<ImageMaskMode>,
        ) -> ImageId {
            if frame_id == FrameId::INVALID {
                let id = self.image_id_gen.incr();
                let _ = self.app_sender.send(AppEvent::Notify(Event::ImageLoadError {
                    image: id,
                    error: formatx!("no frame rendered in window `{window_id:?}`"),
                }));
                let _ = self.app_sender.send(AppEvent::Notify(Event::FrameImageReady {
                    window: window_id,
                    frame: frame_id,
                    image: id,
                    selection: rect,
                }));
                return id;
            }

            let data = self.frame_image_data(gl, rect, scale_factor, mask);

            let id = data.id;

            let _ = self.app_sender.send(AppEvent::ImageLoaded(data));
            let _ = self.app_sender.send(AppEvent::Notify(Event::FrameImageReady {
                window: window_id,
                frame: frame_id,
                image: id,
                selection: rect,
            }));

            id
        }

        /// Create frame_image for a capture request in the FrameRequest.
        pub fn frame_image_data(
            &mut self,
            gl: &dyn gleam::gl::Gl,
            rect: PxRect,
            scale_factor: Factor,
            mask: Option<ImageMaskMode>,
        ) -> ImageLoadedData {
            let data = self.frame_image_data_impl(gl, rect, scale_factor, mask);

            let flags = if data.is_opaque {
                ImageDescriptorFlags::IS_OPAQUE
            } else {
                ImageDescriptorFlags::empty()
            };

            self.images.insert(
                data.id,
                Image(Arc::new(ImageData::RawData {
                    size: data.size,
                    pixels: data.pixels.clone(),
                    descriptor: ImageDescriptor::new(
                        data.size.width.0,
                        data.size.height.0,
                        if data.is_mask { ImageFormat::R8 } else { ImageFormat::BGRA8 },
                        flags,
                    ),
                    ppi: data.ppi,
                })),
            );

            data
        }

        fn frame_image_data_impl(
            &mut self,
            gl: &dyn gleam::gl::Gl,
            rect: PxRect,
            scale_factor: Factor,
            mask: Option<ImageMaskMode>,
        ) -> ImageLoadedData {
            let format = match gl.get_type() {
                gleam::gl::GlType::Gl => gleam::gl::BGRA,
                gleam::gl::GlType::Gles => gleam::gl::RGBA,
            };
            let pixels_flipped = gl.read_pixels(
                rect.origin.x.0,
                rect.origin.y.0,
                rect.size.width.0,
                rect.size.height.0,
                format,
                gleam::gl::UNSIGNED_BYTE,
            );
            let mut buf = vec![0u8; pixels_flipped.len()];
            assert_eq!(rect.size.width.0 as usize * rect.size.height.0 as usize * 4, buf.len());
            let stride = 4 * rect.size.width.0 as usize;
            for (px, buf) in pixels_flipped.chunks_exact(stride).rev().zip(buf.chunks_exact_mut(stride)) {
                buf.copy_from_slice(px);
            }

            if let Some(mask) = mask {
                if format == gleam::gl::BGRA {
                    for bgra in buf.chunks_exact_mut(4) {
                        bgra.swap(0, 3);
                    }
                }
                let (pixels, size, ppi, is_opaque, is_mask) = Self::convert_decoded(
                    image::DynamicImage::ImageRgba8(
                        image::ImageBuffer::from_raw(rect.size.width.0 as u32, rect.size.height.0 as u32, buf).unwrap(),
                    ),
                    Some(mask),
                );

                let id = self.add(ImageRequest {
                    format: ImageDataFormat::A8 { size },
                    data: pixels.clone(),
                    max_decoded_len: u64::MAX,
                    downscale: None,
                    mask: Some(mask),
                });

                ImageLoadedData {
                    id,
                    size,
                    ppi,
                    is_opaque,
                    is_mask,
                    pixels,
                }
            } else {
                if format == gleam::gl::RGBA {
                    for rgba in buf.chunks_exact_mut(4) {
                        rgba.swap(0, 3);
                    }
                }

                let is_opaque = buf.chunks_exact(4).all(|bgra| bgra[3] == 255);

                let data = IpcBytes::from_vec(buf);
                let ppi = 96.0 * scale_factor.0;
                let ppi = Some(ImagePpi::splat(ppi));
                let size = rect.size;

                let id = self.add(ImageRequest {
                    format: ImageDataFormat::Bgra8 { size, ppi },
                    data: data.clone(),
                    max_decoded_len: u64::MAX,
                    downscale: None,
                    mask,
                });

                ImageLoadedData {
                    id,
                    size,
                    ppi,
                    is_opaque,
                    pixels: data,
                    is_mask: false,
                }
            }
        }
    }
}

fn luminance(rgb: &[u8]) -> u8 {
    let r = rgb[0] as f32 / 255.0;
    let g = rgb[1] as f32 / 255.0;
    let b = rgb[2] as f32 / 255.0;

    let l = r * 0.2126 + g * 0.7152 + b * 0.0722;
    (l * 255.0) as u8
}

fn luminance_16(rgb: &[u16]) -> u8 {
    let max = u16::MAX as f32;
    let r = rgb[0] as f32 / max;
    let g = rgb[1] as f32 / max;
    let b = rgb[2] as f32 / max;

    let l = r * 0.2126 + g * 0.7152 + b * 0.0722;
    (l * 255.0) as u8
}

fn luminance_f32(rgb: &[f32]) -> u8 {
    let r = rgb[0];
    let g = rgb[1];
    let b = rgb[2];

    let l = r * 0.2126 + g * 0.7152 + b * 0.0722;
    (l * 255.0) as u8
}
