use std::{fmt, sync::Arc};

use webrender::api::{ImageDescriptor, ImageDescriptorFlags, ImageFormat};
use winit::window::Icon;
use zero_ui_view_api::{
    units::{Px, PxSize},
    Event, ImageDataFormat, ImageId, ImageLoadedData, ImagePpi, IpcBytes, IpcBytesReceiver,
};

use crate::{AppEvent, AppEventSender};
use rustc_hash::FxHashMap;

pub(crate) const ENCODERS: &[&str] = &["png", "jpg", "jpeg", "gif", "ico", "bmp", "ff", "farbfeld"];
pub(crate) const DECODERS: &[&str] = ENCODERS;

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
            image_id_gen: 0,
        }
    }

    pub fn add(&mut self, format: ImageDataFormat, data: IpcBytes, max_decoded_size: u64) -> ImageId {
        let id = self.generate_image_id();

        let app_sender = self.app_sender.clone();
        rayon::spawn(move || {
            let r = match format {
                ImageDataFormat::Bgra8 { size, ppi } => {
                    let expected_len = size.width.0 as usize * size.height.0 as usize * 4;
                    if data.len() != expected_len {
                        Err(format!(
                            "bgra8.len() is not width * height * 4, expected {expected_len}, found {}",
                            data.len()
                        ))
                    } else {
                        let opaque = data.chunks_exact(4).all(|c| c[3] == 255);
                        Ok((data, size, ppi, opaque))
                    }
                }
                fmt => match Self::get_format_and_size(&fmt, &data[..]) {
                    Ok((fmt, size)) => {
                        let decoded_size = size.width.0 as u64 * size.height.0 as u64 * 4;
                        if decoded_size > max_decoded_size {
                            Err(format!(
                                "image {size:?} needs to allocate {decoded_size} bytes, but max allowed size is {max_decoded_size} bytes",
                            ))
                        } else {
                            let _ = app_sender.send(AppEvent::Notify(Event::ImageMetadataLoaded {
                                image: id,
                                size,
                                ppi: None,
                            }));
                            match image::load_from_memory_with_format(&data[..], fmt) {
                                Ok(img) => Ok(Self::convert_decoded(img)),
                                Err(e) => Err(e.to_string()),
                            }
                        }
                    }
                    Err(e) => Err(e),
                },
            };

            match r {
                Ok((bgra8, size, ppi, opaque)) => {
                    let _ = app_sender.send(AppEvent::ImageLoaded(ImageLoadedData {
                        id,
                        bgra8,
                        size,
                        ppi,
                        opaque,
                    }));
                }
                Err(e) => {
                    let _ = app_sender.send(AppEvent::Notify(Event::ImageLoadError { image: id, error: e }));
                }
            }
        });

        id
    }

    pub fn add_pro(&mut self, format: ImageDataFormat, data: IpcBytesReceiver, max_decoded_size: u64) -> ImageId {
        let id = self.generate_image_id();
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
                ImageDataFormat::FileExtension(ext) => image::ImageFormat::from_extension(ext),
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
                                size = image::io::Reader::with_format(std::io::Cursor::new(&full), fmt)
                                    .into_dimensions()
                                    .ok()
                                    .map(|(w, h)| PxSize::new(Px(w as i32), Px(h as i32)));
                                if let Some(s) = size {
                                    let decoded_size = s.width.0 as u64 * s.height.0 as u64 * 4;
                                    if decoded_size > max_decoded_size {
                                        let error = format!(
                                            "image {size:?} needs to allocate {decoded_size} bytes, but max allowed size is {max_decoded_size} bytes",
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
                match image::load_from_memory_with_format(&full[..], fmt) {
                    Ok(img) => {
                        let (bgra8, size, ppi, opaque) = Self::convert_decoded(img);
                        let _ = app_sender.send(AppEvent::ImageLoaded(ImageLoadedData {
                            id,
                            bgra8,
                            size,
                            ppi,
                            opaque,
                        }));
                    }
                    Err(e) => {
                        let _ = app_sender.send(AppEvent::Notify(Event::ImageLoadError {
                            image: id,
                            error: e.to_string(),
                        }));
                    }
                }
            } else if !is_encoded {
                let bgra8 = IpcBytes::from_vec(full);
                let opaque = bgra8.chunks_exact(4).all(|c| c[3] == 255);
                let _ = app_sender.send(AppEvent::ImageLoaded(ImageLoadedData {
                    id,
                    bgra8,
                    size: size.unwrap(),
                    ppi,
                    opaque,
                }));
            } else {
                let _ = app_sender.send(AppEvent::Notify(Event::ImageLoadError {
                    image: id,
                    error: "unknown format".to_string(),
                }));
            }
        });
        id
    }

    fn generate_image_id(&mut self) -> ImageId {
        let mut id = self.image_id_gen.wrapping_add(1);
        if id == 0 {
            id = 1;
        }
        self.image_id_gen = id;
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
        if data.opaque {
            flags |= ImageDescriptorFlags::IS_OPAQUE
        }

        self.images.insert(
            data.id,
            Image(Arc::new(ImageData {
                size: data.size,
                bgra8: data.bgra8.clone(),
                descriptor: ImageDescriptor::new(data.size.width.0, data.size.height.0, ImageFormat::BGRA8, flags),
                ppi: data.ppi,
            })),
        );

        let _ = self.app_sender.send(AppEvent::Notify(Event::ImageLoaded(data)));
    }

    fn get_format_and_size(fmt: &ImageDataFormat, data: &[u8]) -> Result<(image::ImageFormat, PxSize), String> {
        let fmt = match fmt {
            ImageDataFormat::FileExtension(ext) => image::ImageFormat::from_extension(ext),
            ImageDataFormat::MimeType(t) => t.strip_prefix("image/").and_then(image::ImageFormat::from_extension),
            ImageDataFormat::Unknown => None,
            ImageDataFormat::Bgra8 { .. } => unreachable!(),
        };

        let reader = match fmt {
            Some(fmt) => image::io::Reader::with_format(std::io::Cursor::new(data), fmt),
            None => image::io::Reader::new(std::io::Cursor::new(data))
                .with_guessed_format()
                .map_err(|e| e.to_string())?,
        };

        match reader.format() {
            Some(fmt) => {
                let (w, h) = reader.into_dimensions().map_err(|e| e.to_string())?;
                Ok((fmt, PxSize::new(Px(w as i32), Px(h as i32))))
            }
            None => Err("unknown format".to_string()),
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
            ImageRgb32F(img) => (
                img.dimensions(),
                img.into_raw()
                    .chunks(3)
                    .flat_map(|c| [(c[2] * 255.0) as u8, (c[1] * 255.0) as u8, (c[0] * 255.0) as u8, 255])
                    .collect(),
            ),
            ImageRgba32F(img) => (
                img.dimensions(),
                img.into_raw()
                    .chunks(4)
                    .flat_map(|c| {
                        if c[3] < 1.0 {
                            opaque = false;
                            let a = c[3] * 255.0;
                            [(c[2] * a) as u8, (c[1] * a) as u8, (c[0] * a) as u8, a as u8]
                        } else {
                            [(c[2] * 255.0) as u8, (c[1] * 255.0) as u8, (c[0] * 255.0) as u8, 255]
                        }
                    })
                    .collect(),
            ),
            _ => todo!(),
        };

        (
            IpcBytes::from_vec(bgra),
            PxSize::new(Px(size.0 as i32), Px(size.1 as i32)),
            None,
            opaque,
        )
    }

    pub fn encode(&self, id: ImageId, format: String) {
        if !ENCODERS.contains(&format.as_str()) {
            let error = format!("cannot encode `{id}` to `{format}`, unknown format");
            let _ = self
                .app_sender
                .send(AppEvent::Notify(Event::ImageEncodeError { image: id, format, error }));
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
                        let _ = sender.send(AppEvent::Notify(Event::ImageEncoded { image: id, format, data }));
                    }
                    Err(e) => {
                        let error = format!("failed to encode `{id}` to `{format}`, {e}");
                        let _ = sender.send(AppEvent::Notify(Event::ImageEncodeError { image: id, format, error }));
                    }
                }
            })
        } else {
            let error = format!("cannot encode `{id}` to `{format}`, image not found");
            let _ = self
                .app_sender
                .send(AppEvent::Notify(Event::ImageEncodeError { image: id, format, error }));
        }
    }
}

type RawLoadedImg = (IpcBytes, PxSize, ImagePpi, bool);
struct ImageData {
    size: PxSize,
    bgra8: IpcBytes,
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
impl fmt::Debug for Image {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Image")
            .field("size", &self.0.size)
            .field("descriptor", &self.0.descriptor)
            .field("ppi", &self.0.ppi)
            .field("bgra8", &format_args!("<{} shared bytes>", self.0.bgra8.len()))
            .finish()
    }
}
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
            let mut buf = self.0.bgra8.as_ref().to_vec();
            // BGRA to RGBA
            buf.chunks_exact_mut(4).for_each(|c| c.swap(0, 2));
            let img = image::ImageBuffer::from_raw(width, height, buf).unwrap();
            let img = image::DynamicImage::ImageRgba8(img);
            img.resize(255, 255, image::imageops::FilterType::Triangle);

            use image::GenericImageView;
            let (width, height) = img.dimensions();
            let buf = img.into_rgba8().into_raw();
            winit::window::Icon::from_rgba(buf, width, height).ok()
        } else {
            let mut buf = self.0.bgra8.as_ref().to_vec();
            // BGRA to RGBA
            buf.chunks_exact_mut(4).for_each(|c| c.swap(0, 2));
            winit::window::Icon::from_rgba(buf, width, height).ok()
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
        let mut buf = self.0.bgra8[..].to_vec();
        // BGRA to RGBA
        buf.chunks_exact_mut(4).for_each(|c| c.swap(0, 2));
        let rgba = buf;

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
                jpg.encode(&rgba, width, height, ColorType::Rgba8)?;
            }
            ImageFormat::Png => {
                let mut img = image::DynamicImage::ImageRgba8(image::ImageBuffer::from_raw(width, height, rgba).unwrap());
                if opaque {
                    img = image::DynamicImage::ImageRgb8(img.to_rgb8());
                }
                if let Some((ppi_x, ppi_y)) = self.0.ppi {
                    let mut png_bytes = vec![];

                    img.write_to(&mut std::io::Cursor::new(&mut png_bytes), ImageFormat::Png)?;

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
                    img.write_to(&mut std::io::Cursor::new(buffer), ImageFormat::Png)?;
                }
            }
            _ => {
                // other formats that we don't with custom PPI meta.

                let mut img = image::DynamicImage::ImageRgba8(image::ImageBuffer::from_raw(width, height, rgba).unwrap());
                if opaque {
                    img = image::DynamicImage::ImageRgb8(img.to_rgb8());
                }
                img.write_to(&mut std::io::Cursor::new(buffer), format)?;
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
            debug_assert!(self.locked.is_none());

            // SAFETY: this is safe because the Arc is kept alive in `ImageUseMap`.
            let img = unsafe {
                let ptr = key.0 as *const ImageData;
                Arc::increment_strong_count(ptr);
                Arc::<ImageData>::from_raw(ptr)
            };

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

mod capture {
    use std::sync::Arc;

    use webrender::{
        api::{ImageDescriptor, ImageDescriptorFlags, ImageFormat},
        Renderer,
    };
    use zero_ui_view_api::{
        units::{Px, PxRect, PxSize, PxToWr, WrToPx},
        Event, FrameId, ImageDataFormat, ImageId, ImageLoadedData, IpcBytes, WindowId,
    };

    use crate::{
        image_cache::{Image, ImageData},
        AppEvent,
    };

    use super::ImageCache;

    impl ImageCache {
        /// Create frame_image for a `Api::frame_image` request.
        pub fn frame_image(
            &mut self,
            renderer: &mut Renderer,
            rect: PxRect,
            capture_mode: bool,
            window_id: WindowId,
            frame_id: FrameId,
            scale_factor: f32,
        ) -> ImageId {
            if frame_id == FrameId::INVALID {
                let id = self.generate_image_id();
                let _ = self.app_sender.send(AppEvent::Notify(Event::ImageLoadError {
                    image: id,
                    error: format!("no frame rendered in window `{window_id}`"),
                }));
                let _ = self.app_sender.send(AppEvent::Notify(Event::FrameImageReady {
                    window: window_id,
                    frame: frame_id,
                    image: id,
                    selection: rect,
                }));
                return id;
            }

            let data = self.frame_image_data(renderer, rect, capture_mode, scale_factor);

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
            renderer: &mut Renderer,
            rect: PxRect,
            capture_mode: bool,
            scale_factor: f32,
        ) -> ImageLoadedData {
            let data = self.frame_image_data_impl(renderer, rect, capture_mode, scale_factor);

            let flags = if data.opaque {
                ImageDescriptorFlags::IS_OPAQUE
            } else {
                ImageDescriptorFlags::empty()
            };

            self.images.insert(
                data.id,
                Image(Arc::new(ImageData {
                    size: data.size,
                    bgra8: data.bgra8.clone(),
                    descriptor: ImageDescriptor::new(data.size.width.0, data.size.height.0, ImageFormat::BGRA8, flags),
                    ppi: data.ppi,
                })),
            );

            data
        }

        pub fn frame_image_data_impl(
            &mut self,
            renderer: &mut Renderer,
            rect: PxRect,
            capture_mode: bool,
            scale_factor: f32,
        ) -> ImageLoadedData {
            // Firefox uses this API here:
            // https://searchfox.org/mozilla-central/source/gfx/webrender_bindings/RendererScreenshotGrabber.cpp#87
            let (handle, s) = renderer.get_screenshot_async(rect.to_wr_device(), rect.size.to_wr_device(), ImageFormat::BGRA8);
            let mut buf = vec![0; s.width as usize * s.height as usize * 4];
            if renderer.map_and_recycle_screenshot(handle, &mut buf, s.width as usize * 4) {
                if !capture_mode {
                    renderer.release_profiler_structures();
                }

                let opaque = buf.chunks_exact(4).all(|bgra| bgra[3] == 255);

                let data = IpcBytes::from_vec(buf);
                let ppi = 96.0 * scale_factor;
                let ppi = Some((ppi, ppi));
                let id = self.add(
                    ImageDataFormat::Bgra8 {
                        size: PxSize::new(Px(s.width), Px(s.height)),
                        ppi,
                    },
                    data.clone(),
                    u64::MAX,
                );

                let size = s.to_px();

                ImageLoadedData {
                    id,
                    size,
                    ppi,
                    opaque,
                    bgra8: data,
                }
            } else {
                panic!("map_and_recycle_screenshot failed");
            }
        }
    }
}
