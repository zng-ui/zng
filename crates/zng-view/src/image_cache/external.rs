use std::{collections::hash_map::Entry, sync::Arc};

use rustc_hash::FxHashMap;
use webrender::{
    RenderApi,
    api::{
        self as wr, DocumentId, ExternalImage, ExternalImageData, ExternalImageHandler, ExternalImageId, ExternalImageSource,
        ExternalImageType, ImageKey,
        units::{ImageDirtyRect, TexelRect},
    },
};
use zng_unit::{Px, PxPoint, PxRect, PxSize};
use zng_view_api::{ImageRendering, image::ImageTextureId};

use crate::{
    display_list::{DisplayListCache, SpaceAndClip},
    px_wr::PxToWr as _,
};

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
            ImageData::RawData { pixels, range, .. } => {
                ExternalImage {
                    uv: TexelRect::invalid(), // `RawData` does not use `uv`.
                    source: ExternalImageSource::RawData(&pixels[range.clone()]),
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

struct ImageUse {
    image: Image,
    texture_id: ImageTextureId,
    stripes: Box<[ImageTextureId]>,
    mipmap: Vec<ImageUse>,
}

/// Track and manage images used in a renderer.
///
/// The renderer must use [`WrImageCache`] as the external image source.
pub(crate) struct ImageUseMap {
    id_tex: FxHashMap<ExternalImageId, ImageUse>,
    tex_id: FxHashMap<ImageTextureId, ExternalImageId>,
}
impl ImageUseMap {
    pub fn new() -> Self {
        Self {
            id_tex: FxHashMap::default(),
            tex_id: FxHashMap::default(),
        }
    }
    pub fn new_use(&mut self, image: &Image, document_id: DocumentId, api: &mut RenderApi) -> ImageTextureId {
        let id = image.external_id();
        match self.id_tex.entry(id) {
            Entry::Occupied(e) => e.get().texture_id,
            Entry::Vacant(e) => {
                let key = api.generate_image_key();
                let tex_id = ImageTextureId::from_raw(key.1);
                e.insert(ImageUse {
                    // keep the image Arc alive, we expect this in `WrImageCache`.
                    image: image.clone(),
                    texture_id: tex_id,
                    stripes: Box::new([]),
                    mipmap: vec![],
                });
                self.tex_id.insert(tex_id, id);

                // register the top image, most common usage and it is cheap, but
                // during `push_display_list_img` may register other derived images
                // like gigapixel stripes or resized images.
                let mut txn = webrender::Transaction::new();
                txn.add_image(key, image.descriptor(), image.data(), None);
                api.send_transaction(document_id, txn);

                tex_id
            }
        }
    }

    pub fn update_use(
        &mut self,
        texture_id: ImageTextureId,
        image: &Image,
        dirty_rect: Option<PxRect>,
        document_id: DocumentId,
        api: &mut RenderApi,
    ) -> bool {
        if let Entry::Occupied(mut e) = self.tex_id.entry(texture_id) {
            let id = image.external_id();
            if *e.get() != id {
                let prev_image = self.id_tex.get(&id).unwrap();
                if prev_image.image.descriptor() != image.descriptor() {
                    tracing::error!("cannot update image use {texture_id:?}, new image has different dimensions");
                    return false;
                }

                let prev_id = e.insert(id);
                let prev_image = self.id_tex.remove(&prev_id).unwrap();
                let mut txn = webrender::Transaction::new();

                // update only the straight forward usage
                txn.update_image(
                    ImageKey(api.get_namespace_id(), texture_id.get()),
                    image.descriptor(),
                    image.data(),
                    &match dirty_rect {
                        Some(r) => ImageDirtyRect::Partial(r.to_box2d().cast().cast_unit()),
                        None => ImageDirtyRect::All,
                    },
                );

                // remove derived usages
                for stripe in prev_image.stripes {
                    txn.delete_image(ImageKey(api.get_namespace_id(), stripe.get()));
                }
                for mip in prev_image.mipmap {
                    txn.delete_image(ImageKey(api.get_namespace_id(), mip.texture_id.get()));
                    for stripe in mip.stripes {
                        txn.delete_image(ImageKey(api.get_namespace_id(), stripe.get()));
                    }
                }

                self.id_tex.insert(
                    id,
                    ImageUse {
                        image: image.clone(),
                        texture_id,
                        stripes: Box::new([]),
                        mipmap: vec![],
                    },
                );
                api.send_transaction(document_id, txn);
            }

            true
        } else {
            tracing::error!("cannot update image use, texture not found");
            false
        }
    }

    pub fn delete(&mut self, texture_id: ImageTextureId, document_id: DocumentId, api: &mut RenderApi) {
        if let Some(id) = self.tex_id.remove(&texture_id) {
            let image = self.id_tex.remove(&id).unwrap(); // remove but keep alive until the transaction is done.
            let mut txn = webrender::Transaction::new();

            txn.delete_image(ImageKey(api.get_namespace_id(), image.texture_id.get()));
            for stripe in image.stripes {
                txn.delete_image(ImageKey(api.get_namespace_id(), stripe.get()));
            }
            for mip in image.mipmap {
                txn.delete_image(ImageKey(api.get_namespace_id(), mip.texture_id.get()));
                for stripe in mip.stripes {
                    txn.delete_image(ImageKey(api.get_namespace_id(), stripe.get()));
                }
            }

            api.send_transaction(document_id, txn);
        }
    }

    #[expect(clippy::too_many_arguments)]
    pub fn push_display_list_img(
        &mut self,
        document_id: DocumentId,
        api: &mut RenderApi,
        wr_list: &mut wr::DisplayListBuilder,
        sc: &mut SpaceAndClip,
        cache: &DisplayListCache,
        clip_rect: PxRect,
        image_id: ImageTextureId,
        image_size: PxSize,
        rendering: ImageRendering,
        tile_size: PxSize,
        tile_spacing: PxSize,
    ) {
        let img = match self.tex_id.get(&image_id) {
            Some(id) => self.id_tex.get_mut(id).unwrap(),
            None => return,
        };

        if tile_spacing.is_empty() && tile_size == image_size {
            let full_size = img.image.size();

            if !img.image.overflows_wr() {
                // normal sized image
                let bounds = clip_rect.to_wr();
                let clip = sc.clip_chain_id(wr_list);
                let props = wr::CommonItemProperties {
                    clip_rect: bounds,
                    clip_chain_id: clip,
                    spatial_id: sc.spatial_id(),
                    flags: sc.primitive_flags(),
                };
                wr_list.push_image(
                    &props,
                    PxRect::from_size(image_size).to_wr(),
                    rendering.to_wr(),
                    wr::AlphaType::PremultipliedAlpha,
                    wr::ImageKey(cache.id_namespace(), img.texture_id.get()),
                    wr::ColorF::WHITE,
                );
            } else {
                if img.stripes.is_empty() {
                    let stripes = img.image.wr_stripes();
                    if stripes.is_empty() {
                        // error returns empty
                        return;
                    }

                    // register texture IDs for the stripes
                    let mut stripe_ids = Vec::with_capacity(stripes.len());
                    let mut txn = webrender::Transaction::new();
                    for stripe in stripes {
                        let key = api.generate_image_key();
                        let tex_id = ImageTextureId::from_raw(key.1);
                        stripe_ids.push(tex_id);
                        txn.add_image(key, stripe.descriptor(), stripe.data(), None);
                    }
                    api.send_transaction(document_id, txn);
                    img.stripes = stripe_ids.into_boxed_slice();
                }

                // gigantic image split into stripes
                let scale_x = full_size.width.0 as f32 / image_size.width.0 as f32;
                let scale_y = full_size.height.0 as f32 / image_size.height.0 as f32;

                let mut scaled_y = Px(0);
                for (stripe_id, img) in img.stripes.iter().zip(img.image.wr_stripes()) {
                    let scaled_stripe = {
                        let mut r = img.size();
                        r.width /= scale_x;
                        r.height /= scale_y;
                        r
                    };
                    if scaled_y != Px(0) {
                        let spatial_id = wr_list.push_reference_frame(
                            PxPoint::zero().to_wr(),
                            sc.spatial_id(),
                            wr::TransformStyle::Flat,
                            wr::PropertyBinding::Value(wr::units::LayoutTransform::translation(0.0, scaled_y.0 as f32, 0.0)),
                            wr::ReferenceFrameKind::Transform {
                                is_2d_scale_translation: true,
                                should_snap: false,
                                paired_with_perspective: false,
                            },
                            sc.next_view_process_frame_id().to_wr(),
                        );
                        sc.push_spatial(spatial_id);
                    }
                    let mut clip_rect = clip_rect;
                    clip_rect.origin.y -= scaled_y;
                    let clip = sc.clip_chain_id(wr_list);
                    let props = wr::CommonItemProperties {
                        clip_rect: clip_rect.to_wr(),
                        clip_chain_id: clip,
                        spatial_id: sc.spatial_id(),
                        flags: sc.primitive_flags(),
                    };
                    wr_list.push_image(
                        &props,
                        PxRect::from_size(scaled_stripe).to_wr(),
                        rendering.to_wr(),
                        wr::AlphaType::PremultipliedAlpha,
                        wr::ImageKey(cache.id_namespace(), stripe_id.get()),
                        wr::ColorF::WHITE,
                    );
                    if scaled_y != Px(0) {
                        wr_list.pop_reference_frame();
                        sc.pop_spatial();
                    }
                    scaled_y += scaled_stripe.height;
                }
            }
        } else {
            if img.image.overflows_wr() {
                tracing::error!("cannot tile or repeat image, too large");
                return;
            }

            let bounds = clip_rect.to_wr();
            let clip = sc.clip_chain_id(wr_list);
            let props = wr::CommonItemProperties {
                clip_rect: bounds,
                clip_chain_id: clip,
                spatial_id: sc.spatial_id(),
                flags: sc.primitive_flags(),
            };
            wr_list.push_repeating_image(
                &props,
                PxRect::from_size(image_size).to_wr(),
                tile_size.to_wr(),
                tile_spacing.to_wr(),
                rendering.to_wr(),
                wr::AlphaType::PremultipliedAlpha,
                wr::ImageKey(cache.id_namespace(), img.texture_id.get()),
                wr::ColorF::WHITE,
            );
        }
    }
}
