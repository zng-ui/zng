use std::sync::Arc;

use glutin::window::Icon;
use webrender::api::{ImageDescriptor, ImageDescriptorFlags, ImageFormat};
use zero_ui_view_api::{
    units::{Px, PxSize},
    Event, ImageDataFormat, ImageId, ImagePpi, IpcSharedMemory,
};

use crate::{AppEvent, AppEventSender};
use rustc_hash::FxHashMap;

pub(crate) const ENCODERS: &[&str] = &["jpg", "jpeg", "png", "gif", "ico", "bmp", "ff", "farbfeld"];
pub(crate) const DECODERS: &[&str] = ENCODERS;

/// Decode and cache image resources.
pub(crate) struct ImageCache<S> {
    app_sender: S,
    images: FxHashMap<ImageId, Image>,
    image_id_gen: ImageId,
}
impl<S: AppEventSender> ImageCache<S> {
    pub fn new(app_sender: S) -> Self {
        Self {
            app_sender,
            images: FxHashMap::default(),
            image_id_gen: 0,
        }
    }

    pub fn add(&mut self, data: IpcSharedMemory, format: ImageDataFormat) -> ImageId {
        let mut id = self.image_id_gen.wrapping_add(1);
        if id == 0 {
            id = 1;
        }
        self.image_id_gen = id;

        let app_sender = self.app_sender.clone();

        rayon::spawn(move || {
            let r = match format {
                ImageDataFormat::Bgra8 { size, ppi } => {
                    let expected_len = size.width.0 as usize * size.height.0 as usize * 4;
                    if data.len() != expected_len {
                        Err(format!(
                            "bgra8.len() is not width * height * 4, expected {}, found {}",
                            expected_len,
                            data.len()
                        ))
                    } else {
                        let opaque = data.chunks_exact(4).all(|c| c[3] == 255);
                        Ok((data, size, ppi, opaque))
                    }
                }
                ImageDataFormat::FileExtension(ext) => Self::load_file(data, ext),
                ImageDataFormat::MimeType(mime) => Self::load_web(data, mime),
                ImageDataFormat::Unknown => Self::load_unknown(data),
            };

            match r {
                Ok((bgra8, size, ppi, opaque)) => {
                    let _ = app_sender.send(AppEvent::ImageLoaded(id, bgra8, size, ppi, opaque));
                }
                Err(e) => {
                    let _ = app_sender.send(AppEvent::Notify(Event::ImageLoadError(id, e)));
                }
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
    pub fn loaded(&mut self, id: ImageId, bgra8: IpcSharedMemory, size: PxSize, ppi: ImagePpi, opaque: bool) {
        let flags = if opaque {
            ImageDescriptorFlags::IS_OPAQUE
        } else {
            ImageDescriptorFlags::empty()
        };
        let bgra8 = IpcSharedMemory::from_bytes(&bgra8);
        self.images.insert(
            id,
            Image(Arc::new(ImageData {
                size,
                bgra8: bgra8.clone(),
                descriptor: ImageDescriptor::new(size.width.0, size.height.0, ImageFormat::BGRA8, flags),
                ppi,
            })),
        );

        let _ = self
            .app_sender
            .send(AppEvent::Notify(Event::ImageLoaded(id, size, ppi, opaque, bgra8)));
    }

    fn load_file(data: IpcSharedMemory, ext: String) -> Result<RawLoadedImg, String> {
        if let Some(f) = image::ImageFormat::from_extension(ext) {
            if !f.can_read() {
                return Err(format!("not supported, cannot decode `{:?}` images", f.extensions_str()));
            }
            match image::load_from_memory_with_format(&data, f) {
                Ok(img) => Ok(Self::convert_decoded(img)),
                Err(e) => Err(format!("{:?}", e)),
            }
        } else {
            Self::load_unknown(data)
        }
    }

    fn load_web(data: IpcSharedMemory, mime: String) -> Result<RawLoadedImg, String> {
        if let Some(format) = mime.strip_prefix("image/") {
            Self::load_file(data, format.to_owned())
        } else {
            Self::load_unknown(data)
        }
    }

    fn load_unknown(data: IpcSharedMemory) -> Result<RawLoadedImg, String> {
        match image::load_from_memory(&data) {
            Ok(img) => Ok(Self::convert_decoded(img)),
            Err(e) => Err(format!("{:?}", e)),
        }
    }

    fn convert_decoded(image: image::DynamicImage) -> RawLoadedImg {
        use image::DynamicImage::*;

        let mut opaque = true;
        let (size, bgra) = match image {
            ImageLuma8(img) => (img.dimensions(), img.into_raw().into_iter().flat_map(|l| [l, l, l, 255]).collect()),
            ImageLumaA8(img) => (
                img.dimensions(),
                img.into_raw()
                    .chunks(2)
                    .flat_map(|la| {
                        if la[1] < 255 {
                            opaque = false;
                            let l = la[0] as f32 * la[1] as f32 / 255.0;
                            let l = l as u8;
                            [l, l, l, la[1]]
                        } else {
                            let l = la[0];
                            [l, l, l, la[1]]
                        }
                    })
                    .collect(),
            ),
            ImageRgb8(img) => (
                img.dimensions(),
                img.into_raw().chunks(3).flat_map(|c| [c[2], c[1], c[0], 255]).collect(),
            ),
            ImageRgba8(img) => (img.dimensions(), {
                let mut buf = img.into_raw();
                buf.chunks_mut(4).for_each(|c| {
                    if c[3] < 255 {
                        opaque = false;
                        let a = c[3] as f32 / 255.0;
                        c[0..3].iter_mut().for_each(|c| *c = (*c as f32 * a) as u8);
                    }
                    c.swap(0, 2);
                });
                buf
            }),
            ImageBgr8(img) => (
                img.dimensions(),
                img.into_raw().chunks(3).flat_map(|c| [c[0], c[1], c[2], 255]).collect(),
            ),
            ImageBgra8(img) => (img.dimensions(), {
                let mut buf = img.into_raw();
                buf.chunks_mut(4).for_each(|c| {
                    if c[3] < 255 {
                        opaque = false;
                        let a = c[3] as f32 / 255.0;
                        c[0..3].iter_mut().for_each(|c| *c = (*c as f32 * a) as u8);
                    }
                });
                buf
            }),
            ImageLuma16(img) => (
                img.dimensions(),
                img.into_raw()
                    .into_iter()
                    .flat_map(|l| {
                        let l = (l as f32 / u16::MAX as f32 * 255.0) as u8;
                        [l, l, l, 255]
                    })
                    .collect(),
            ),
            ImageLumaA16(img) => (
                img.dimensions(),
                img.into_raw()
                    .chunks(2)
                    .flat_map(|la| {
                        let max = u16::MAX as f32;
                        let l = la[0] as f32 / max;
                        let a = la[1] as f32 / max * 255.0;

                        if la[1] < u16::MAX {
                            opaque = false;
                            let l = (l * a) as u8;
                            [l, l, l, a as u8]
                        } else {
                            let l = (l * 255.0) as u8;
                            [l, l, l, a as u8]
                        }
                    })
                    .collect(),
            ),
            ImageRgb16(img) => (
                img.dimensions(),
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
                    .collect(),
            ),
            ImageRgba16(img) => (
                img.dimensions(),
                img.into_raw()
                    .chunks(4)
                    .flat_map(|c| {
                        if c[3] < u16::MAX {
                            opaque = false;
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
                    .collect(),
            ),
        };

        (
            IpcSharedMemory::from_bytes(&bgra),
            PxSize::new(Px(size.0 as i32), Px(size.1 as i32)),
            None,
            opaque,
        )
    }

    pub fn encode(&self, id: ImageId, format: String) {
        if !ENCODERS.contains(&format.as_str()) {
            let error = format!("cannot encode `{}` to `{}`, unknown format", id, format);
            let _ = self.app_sender.send(AppEvent::Notify(Event::ImageEncodeError(id, format, error)));
            return;
        }

        if let Some(img) = self.get(id) {
            let fmt = image::ImageFormat::from_extension(&format).unwrap();
            debug_assert!(fmt.can_write());

            let img = img.clone();
            let sender = self.app_sender.clone();
            rayon::spawn(move || {
                let mut data = vec![];
                match img.encode(fmt, &mut data) {
                    Ok(_) => {
                        let _ = sender.send(AppEvent::Notify(Event::ImageEncoded(id, format, data)));
                    }
                    Err(e) => {
                        let error = format!("failed to encode `{}` to `{}`, {}", id, format, e);
                        let _ = sender.send(AppEvent::Notify(Event::ImageEncodeError(id, format, error)));
                    }
                }
            })
        } else {
            let error = format!("cannot encode `{}` to `{}`, image not found", id, format);
            let _ = self.app_sender.send(AppEvent::Notify(Event::ImageEncodeError(id, format, error)));
        }
    }
}

type RawLoadedImg = (IpcSharedMemory, PxSize, ImagePpi, bool);
struct ImageData {
    size: PxSize,
    bgra8: IpcSharedMemory,
    descriptor: ImageDescriptor,
    ppi: ImagePpi,
}
impl ImageData {
    pub fn opaque(&self) -> bool {
        self.descriptor.flags.contains(ImageDescriptorFlags::IS_OPAQUE)
    }
}
#[derive(Clone)]
pub(crate) struct Image(Arc<ImageData>);
impl Image {
    pub fn descriptor(&self) -> ImageDescriptor {
        self.0.descriptor
    }

    /// Generate a window icon from the image.
    pub fn icon(&self) -> Option<Icon> {
        let width = self.0.size.width.0 as u32;
        let height = self.0.size.height.0 as u32;
        if width == 0 || height == 0 {
            None
        } else if width > 255 || height > 255 {
            // resize to max 255
            let img = image::ImageBuffer::from_raw(width, height, self.0.bgra8.to_vec()).unwrap();
            let img = image::DynamicImage::ImageBgra8(img);
            img.resize(255, 255, image::imageops::FilterType::Triangle);

            use image::GenericImageView;
            let (width, height) = img.dimensions();
            let buf = img.to_rgba8().into_raw();
            glutin::window::Icon::from_rgba(buf, width, height).ok()
        } else {
            let mut buf = self.0.bgra8.to_vec();
            // BGRA to RGBA
            buf.chunks_exact_mut(4).for_each(|c| c.swap(0, 2));
            glutin::window::Icon::from_rgba(buf, width, height).ok()
        }
    }

    pub fn encode(&self, format: image::ImageFormat, buffer: &mut Vec<u8>) -> image::ImageResult<()> {
        if self.0.size.width <= Px(0) || self.0.size.height <= Px(0) {
            return Err(image::ImageError::IoError(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "cannot encode zero sized image",
            )));
        }

        use image::*;

        // invert rows, `image` only supports top-to-bottom buffers.
        let bgra: Vec<_> = self
            .0
            .bgra8
            .rchunks_exact(self.0.size.width.0 as usize * 4)
            .flatten()
            .copied()
            .collect();

        let width = self.0.size.width.0 as u32;
        let height = self.0.size.height.0 as u32;
        let opaque = self.0.opaque();

        match format {
            ImageFormat::Jpeg => {
                let mut jpg = codecs::jpeg::JpegEncoder::new(buffer);
                if let Some((ppi_x, ppi_y)) = self.0.ppi {
                    jpg.set_pixel_density(codecs::jpeg::PixelDensity {
                        density: (ppi_x as u16, ppi_y as u16),
                        unit: codecs::jpeg::PixelDensityUnit::Inches,
                    });
                }
                jpg.encode(&bgra, width, height, ColorType::Bgra8)?;
            }
            ImageFormat::Farbfeld => {
                let mut pixels = Vec::with_capacity(bgra.len() * 2);
                for bgra in bgra.chunks(4) {
                    fn c(c: u8) -> [u8; 2] {
                        let c = (c as f32 / 255.0) * u16::MAX as f32;
                        (c as u16).to_ne_bytes()
                    }
                    pixels.extend(c(bgra[2]));
                    pixels.extend(c(bgra[1]));
                    pixels.extend(c(bgra[0]));
                    pixels.extend(c(bgra[3]));
                }

                let ff = codecs::farbfeld::FarbfeldEncoder::new(buffer);
                ff.encode(&pixels, width, height)?;
            }
            ImageFormat::Tga => {
                let tga = codecs::tga::TgaEncoder::new(buffer);
                tga.encode(&bgra, width, height, ColorType::Bgra8)?;
            }
            rgb_only => {
                let mut pixels;
                let color_type;
                if opaque {
                    color_type = ColorType::Rgb8;
                    pixels = Vec::with_capacity(width as usize * height as usize * 3);
                    for bgra in bgra.chunks(4) {
                        pixels.push(bgra[2]);
                        pixels.push(bgra[1]);
                        pixels.push(bgra[0]);
                    }
                } else {
                    color_type = ColorType::Rgba8;
                    pixels = bgra;
                    for pixel in pixels.chunks_mut(4) {
                        pixel.swap(0, 2);
                    }
                }

                match rgb_only {
                    ImageFormat::Png => {
                        if let Some((ppi_x, ppi_y)) = self.0.ppi {
                            let mut png_bytes = vec![];
                            let png = codecs::png::PngEncoder::new(&mut png_bytes);
                            png.encode(&pixels, width, height, color_type)?;

                            let mut png = img_parts::png::Png::from_bytes(png_bytes.into()).unwrap();

                            let chunk_kind = *b"pHYs";
                            debug_assert!(png.chunk_by_type(chunk_kind).is_none());

                            use byteorder::*;
                            let mut chunk = Vec::with_capacity(4 * 2 + 1);

                            // ppi / inch_to_metric
                            let ppm_x = (ppi_x / 0.0254) as u32;
                            let ppm_y = (ppi_y / 0.0254) as u32;

                            chunk.write_u32::<BigEndian>(ppm_x).unwrap();
                            chunk.write_u32::<BigEndian>(ppm_y).unwrap();
                            chunk.write_u8(1).unwrap(); // metric

                            let chunk = img_parts::png::PngChunk::new(chunk_kind, chunk.into());
                            png.chunks_mut().insert(1, chunk);

                            png.encoder().write_to(buffer)?;
                        } else {
                            let png = codecs::png::PngEncoder::new(buffer);
                            png.encode(&pixels, width, height, color_type)?;
                        }
                    }
                    ImageFormat::Tiff => {
                        // TODO set ResolutionUnit to 2 (inch) and set both XResolution and YResolution
                        let mut seek_buf = std::io::Cursor::new(vec![]);
                        let tiff = codecs::tiff::TiffEncoder::new(&mut seek_buf);

                        tiff.encode(&pixels, width, height, color_type)?;

                        *buffer = seek_buf.into_inner();
                    }
                    ImageFormat::Gif => {
                        let mut gif = codecs::gif::GifEncoder::new(buffer);

                        gif.encode(&pixels, width, height, color_type)?;
                    }
                    ImageFormat::Bmp => {
                        // TODO set biXPelsPerMeter and biYPelsPerMeter
                        let mut bmp = codecs::bmp::BmpEncoder::new(buffer);

                        bmp.encode(&pixels, width, height, color_type)?;
                    }
                    ImageFormat::Ico => {
                        // TODO set density in the inner PNG?
                        let ico = codecs::ico::IcoEncoder::new(buffer);

                        ico.encode(&pixels, width, height, color_type)?;
                    }
                    unsuported => {
                        use image::error::*;
                        let hint = ImageFormatHint::Exact(unsuported);
                        return Err(ImageError::Unsupported(UnsupportedError::from_format_and_kind(
                            hint.clone(),
                            UnsupportedErrorKind::Format(hint),
                        )));
                    }
                }
            }
        }

        Ok(())
    }
}

// Image data is provided to webrender directly from the BGRA8 shared memory.
// The [`ExternalImageId`] is the Arc pointer to ImageData.
mod external {
    use std::{collections::hash_map::Entry, sync::Arc};

    use rustc_hash::FxHashMap;
    use webrender::{
        api::{
            units::{ImageDirtyRect, TexelRect},
            DocumentId, ExternalImage, ExternalImageData, ExternalImageHandler, ExternalImageId, ExternalImageSource, ExternalImageType,
            ImageKey, ImageRendering,
        },
        RenderApi,
    };

    use super::{Image, ImageData};

    /// Implements [`ExternalImageHandler`].
    ///
    /// # Safety
    ///
    /// This is only safe if use with [`ImageUseMap`].
    pub(crate) struct WrImageCache {
        locked: Option<Arc<ImageData>>,
    }
    impl WrImageCache {
        pub fn new_boxed() -> Box<dyn ExternalImageHandler> {
            Box::new(WrImageCache { locked: None })
        }
    }
    impl ExternalImageHandler for WrImageCache {
        fn lock(&mut self, key: ExternalImageId, _channel_index: u8, _rendering: ImageRendering) -> ExternalImage {
            // SAFETY: this is safe the Arc is kept alive in `ImageUseMap`.
            let img = unsafe { Arc::<ImageData>::from_raw(key.0 as *const _) };
            self.locked = Some(img); // keep alive just in case the image is removed mid-use?
            ExternalImage {
                uv: TexelRect::new(0.0, 0.0, 1.0, 1.0),
                source: ExternalImageSource::RawData(&self.locked.as_ref().unwrap().bgra8[..]),
            }
        }

        fn unlock(&mut self, _key: ExternalImageId, _channel_index: u8) {
            self.locked = None;
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
        id_key: FxHashMap<ExternalImageId, (ImageKey, Image)>,
        key_id: FxHashMap<ImageKey, ExternalImageId>,
    }
    impl ImageUseMap {
        pub fn new_use(&mut self, image: &Image, document_id: DocumentId, api: &mut RenderApi) -> ImageKey {
            let id = image.external_id();
            match self.id_key.entry(id) {
                Entry::Occupied(e) => e.get().0,
                Entry::Vacant(e) => {
                    let key = api.generate_image_key();
                    e.insert((key, image.clone())); // keep the image Arc alive, we expect this in `WrImageCache`.
                    self.key_id.insert(key, id);

                    let mut txn = webrender::Transaction::new();
                    txn.add_image(key, image.descriptor(), image.data(), None);
                    api.send_transaction(document_id, txn);

                    key
                }
            }
        }

        /// Returns if needs to update.
        pub fn update_use(&mut self, key: ImageKey, image: &Image, document_id: DocumentId, api: &mut RenderApi) {
            if let Entry::Occupied(mut e) = self.key_id.entry(key) {
                let id = image.external_id();
                if *e.get() != id {
                    let prev_id = e.insert(id);
                    self.id_key.remove(&prev_id).unwrap();
                    self.id_key.insert(id, (key, image.clone()));

                    let mut txn = webrender::Transaction::new();
                    txn.update_image(key, image.descriptor(), image.data(), &ImageDirtyRect::All);
                    api.send_transaction(document_id, txn);
                }
            }
        }

        pub fn delete(&mut self, key: ImageKey, document_id: DocumentId, api: &mut RenderApi) {
            if let Some(id) = self.key_id.remove(&key) {
                let _img = self.id_key.remove(&id); // remove but keep alive until the transaction is done.
                let mut txn = webrender::Transaction::new();
                txn.delete_image(key);
                api.send_transaction(document_id, txn);
            }
        }
    }
}
pub(crate) use external::{ImageUseMap, WrImageCache};
