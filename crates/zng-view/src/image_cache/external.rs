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
use zng_unit::{Px, PxPoint, PxRect, PxSize, euclid};
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

/// Track and manage images used in a renderer.
///
/// The renderer must use [`WrImageCache`] as the external image source.
#[derive(Default)]
pub(crate) struct ImageUseMap {
    id_tex: FxHashMap<ExternalImageId, Box<[(ImageTextureId, Image)]>>,
    tex_id: FxHashMap<ImageTextureId, ExternalImageId>,
}
impl ImageUseMap {
    const MAX_LEN: usize = i32::MAX as usize;

    pub fn new_use(&mut self, image: &Image, document_id: DocumentId, api: &mut RenderApi) -> ImageTextureId {
        let id = image.external_id();
        match self.id_tex.entry(id) {
            Entry::Occupied(e) => e.get()[0].0,
            Entry::Vacant(e) => {
                if image.pixels().len() <= Self::MAX_LEN {
                    let key = api.generate_image_key();
                    let tex_id = ImageTextureId::from_raw(key.1);
                    e.insert(Box::new([(tex_id, image.clone())])); // keep the image Arc alive, we expect this in `WrImageCache`.
                    self.tex_id.insert(tex_id, id);

                    let mut txn = webrender::Transaction::new();
                    txn.add_image(key, image.descriptor(), image.data(), None);
                    api.send_transaction(document_id, txn);

                    tex_id
                } else {
                    // Webrender uses i32 offsets for manipulating images, to support gigapixel images
                    // we split the image into "stripes" that fit <=MAX_LEN bytes

                    let full_size = image.size();
                    let w = full_size.width.0 as usize * 4;
                    if w > Self::MAX_LEN {
                        tracing::error!("renderer does not support images with width * 4 > {}", Self::MAX_LEN);
                        return ImageTextureId::INVALID;
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

                    // generate ImageTextureId for each stripe, we use the first stripe ID to identify in
                    // the display list, when converted to Webrender display list the other stripes are injected
                    let mut txn = webrender::Transaction::new();
                    let mut stripes = Vec::with_capacity(stripes_len + 1);

                    // we store the full image reference for `update_use`
                    stripes.push((ImageTextureId::INVALID, image.clone()));

                    // generate stripe images and transaction that associates each with a ImageTextureId
                    for i in 0..stripes_len {
                        let key = api.generate_image_key();
                        let tex_id = ImageTextureId::from_raw(key.1);

                        let y = stripe_height * Px(i as _);
                        let mut size = full_size;
                        size.height = stripe_height.min(full_size.height - y);
                        let mut descriptor = image.descriptor();
                        descriptor.size = euclid::size2(size.width.0, size.height.0);

                        let offset = stripe_len * i;
                        let range = offset..((offset + stripe_len).min(image.pixels().len()));

                        let stripe = Image(Arc::new(ImageData::RawData {
                            size,
                            pixels: image.pixels().clone(),
                            descriptor,
                            density: image.density(),
                            range,
                        }));

                        txn.add_image(key, stripe.descriptor(), stripe.data(), None);

                        stripes.push((tex_id, stripe));
                    }

                    let tex_id = stripes[1].0;

                    e.insert(stripes.into_boxed_slice());
                    self.tex_id.insert(tex_id, id);

                    api.send_transaction(document_id, txn);

                    tex_id
                }
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
                if prev_image[0].1.descriptor() != image.descriptor() {
                    tracing::error!("cannot update image use {texture_id:?}, new image has different dimensions");
                    return false;
                }

                let prev_id = e.insert(id);
                let mut prev_image = self.id_tex.remove(&prev_id).unwrap();
                let mut txn = webrender::Transaction::new();

                if prev_image.len() == 1 {
                    prev_image[0] = (texture_id, image.clone());

                    txn.update_image(
                        ImageKey(api.get_namespace_id(), texture_id.get()),
                        image.descriptor(),
                        image.data(),
                        &match dirty_rect {
                            Some(r) => ImageDirtyRect::Partial(r.to_box2d().cast().cast_unit()),
                            None => ImageDirtyRect::All,
                        },
                    );
                } else {
                    prev_image[0].1 = image.clone();
                    let mut y = Px(0);
                    for stripe in &mut prev_image[1..] {
                        let img = ImageData::RawData {
                            size: stripe.1.size(),
                            pixels: image.pixels().clone(),
                            descriptor: stripe.1.descriptor(),
                            density: image.density(),
                            range: stripe.1.range(),
                        };
                        stripe.1 = Image(Arc::new(img));

                        txn.update_image(
                            ImageKey(api.get_namespace_id(), stripe.0.get()),
                            stripe.1.descriptor(),
                            stripe.1.data(),
                            &match dirty_rect {
                                Some(mut r) => {
                                    r.origin.y -= y;
                                    let mut r = r.to_box2d();
                                    r.min.y = r.min.y.max(Px(0));
                                    r.max.y = r.max.y.min(stripe.1.size().height);

                                    ImageDirtyRect::Partial(r.cast().cast_unit())
                                }
                                None => ImageDirtyRect::All,
                            },
                        );

                        y += stripe.1.size().height;
                    }
                }

                self.id_tex.insert(id, prev_image);
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
            if image.len() == 1 {
                txn.delete_image(ImageKey(api.get_namespace_id(), image[0].0.get()));
            } else {
                for (texture_id, _) in &image[1..] {
                    txn.delete_image(ImageKey(api.get_namespace_id(), texture_id.get()));
                }
            }
            api.send_transaction(document_id, txn);
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn push_display_list_img(
        &self,
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
            Some(id) => self.id_tex.get(id).unwrap(),
            None => return,
        };

        if tile_spacing.is_empty() && tile_size == image_size {
            let full_size = img[0].1.size();
            if full_size.width > image_size.width * Px(2)
                && full_size.height > image_size.height * Px(2)
                && full_size.width > Px(2_048)
                && full_size.height > Px(2_048)
            {
                tracing::warn!("!!: rendering large amount of texture quads scaled down +2x");
            }

            if img.len() == 1 {
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
                    wr::ImageKey(cache.id_namespace(), image_id.get()),
                    wr::ColorF::WHITE,
                );
            } else {
                // gigantic image split into stripes
                let scale_x = full_size.width.0 as f32 / image_size.width.0 as f32;
                let scale_y = full_size.height.0 as f32 / image_size.height.0 as f32;

                let mut scaled_y = Px(0);
                for (stripe_id, img) in &img[1..] {
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
            if img.len() > 1 {
                tracing::error!("tiling or repeating images cannot have len > {}", Self::MAX_LEN);
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
                wr::ImageKey(cache.id_namespace(), image_id.get()),
                wr::ColorF::WHITE,
            );
        }
    }
}
