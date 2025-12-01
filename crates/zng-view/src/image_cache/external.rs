use std::{collections::hash_map::Entry, sync::Arc};

use rustc_hash::FxHashMap;
use webrender::{
    RenderApi,
    api::{
        DocumentId, ExternalImage, ExternalImageData, ExternalImageHandler, ExternalImageId, ExternalImageSource, ExternalImageType,
        ImageKey,
        units::{ImageDirtyRect, TexelRect},
    },
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
    fn lock(&mut self, key: ExternalImageId, _channel_index: u8, _is_composited: bool) -> ExternalImage<'_> {
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
            normalized_uvs: false,
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
