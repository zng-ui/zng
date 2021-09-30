use std::sync::Arc;

use webrender::api::{ImageDescriptor, ImageDescriptorFlags, ImageFormat};
use zero_ui_view_api::{units::PxSize, Event, ImageDataFormat, ImageId};

use crate::{AppEvent, AppEventSender};
use rustc_hash::FxHashMap;

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

    pub fn cache(&mut self, data: Vec<u8>, format: ImageDataFormat) -> ImageId {
        let mut id = self.image_id_gen.wrapping_add(1);
        if id == 0 {
            id = 1;
        }
        self.image_id_gen = id;

        match format {
            ImageDataFormat::Bgra8 { size, dpi } => self.loaded(id, data, size, dpi, None),
            ImageDataFormat::FileExt(ext) => self.load_file(data, ext),
            ImageDataFormat::Mime(mime) => self.load_web(data, mime),
            ImageDataFormat::Unknown => self.load_unknown(data),
        }

        id
    }

    pub fn uncache(&mut self, id: ImageId) {
        self.images.remove(&id);
    }

    pub fn get(&self, id: ImageId) -> Option<&Image> {
        self.images.get(&id)
    }

    pub fn loaded(&mut self, id: ImageId, bgra8: Vec<u8>, size: PxSize, dpi: (f32, f32), opaque: Option<bool>) {
        let expected_len = size.width.0 as usize * size.height.0 as usize * 4;
        if bgra8.len() != expected_len {
            let _ = self.app_sender.send(AppEvent::Notify(Event::ImageLoadError(
                id,
                format!("bgra8.len() is not width * height * 4, expected {}, found {}", expected_len, bgra8.len()),
            )));
            return;
        }

        let opaque = opaque.unwrap_or_else(|| bgra8.chunks(4).all(|c| c[3] == 255));

        let flags = if opaque {
            ImageDescriptorFlags::IS_OPAQUE
        } else {
            ImageDescriptorFlags::empty()
        };
        self.images.insert(id, Image {
            size,
            bgra8: Arc::new(bgra8),
            descriptor: ImageDescriptor::new(size.width.0, size.height.0, ImageFormat::BGRA8, flags),
            dpi,
        });

        let _ = self.app_sender.send(AppEvent::Notify(Event::ImageLoaded(id, size, dpi, opaque)));
    }

    fn load_file(&mut self, data: Vec<u8>, ext: String) {
        todo!()
    }

    fn load_web(&mut self, data: Vec<u8>, mime: String) {
        todo!()
    }

    fn load_unknown(&mut self, data: Vec<u8>) {
        todo!()
    }
}

pub(crate) struct Image {
    pub size: PxSize,
    pub bgra8: Arc<Vec<u8>>,
    pub descriptor: ImageDescriptor,
    pub dpi: (f32, f32),
}
impl Image {
    pub fn opaque(&self) -> bool {
        self.descriptor.flags.contains(ImageDescriptorFlags::IS_OPAQUE)
    }
}
