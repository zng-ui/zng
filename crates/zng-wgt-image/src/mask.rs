//! Mask properties, [`mask_image`], [`mask_mode`] and more.
//!
//! [`mask_image`]: fn@mask_image
//! [`mask_mode`]: fn@mask_mode

use zng_ext_image::{
    IMAGES, ImageCacheMode, ImageDownscaleMode, ImageEntriesMode, ImageLimits, ImageMaskMode, ImageOptions, ImageRenderArgs, ImageSource,
};
use zng_wgt::prelude::*;

use crate::ImageFit;

/// Sets an image mask.
///
/// This property is configured by contextual values set by the properties in the [`mask`] module.
/// By default the image alpha channel is used as mask, this can be changed by the [`mask_mode`] property.
///
/// [`mask`]: crate::mask
/// [`mask_mode`]: fn@mask_mode
#[property(BORDER-1)]
pub fn mask_image(child: impl IntoUiNode, source: impl IntoVar<ImageSource>) -> UiNode {
    let source = source.into_var();
    let mut img = None;
    let mut img_size = PxSize::zero();
    let mut rect = PxRect::zero();

    match_node(child, move |c, op| match op {
        UiNodeOp::Init => {
            // load
            WIDGET
                .sub_var(&source)
                .sub_var(&MASK_MODE_VAR)
                .sub_var(&MASK_IMAGE_CACHE_VAR)
                .sub_var(&MASK_IMAGE_DOWNSCALE_VAR);

            let mode = if MASK_IMAGE_CACHE_VAR.get() {
                ImageCacheMode::Cache
            } else {
                ImageCacheMode::Ignore
            };
            let limits = MASK_IMAGE_LIMITS_VAR.get();
            let downscale = MASK_IMAGE_DOWNSCALE_VAR.get();
            let mask_mode = MASK_MODE_VAR.get();

            let mut source = source.get();
            if let ImageSource::Render(_, args) = &mut source {
                *args = Some(ImageRenderArgs::new(WINDOW.id()));
            }
            let opt = ImageOptions::new(mode, downscale, Some(mask_mode), ImageEntriesMode::PRIMARY);
            let i = IMAGES.image(source, opt, limits);
            let s = i.subscribe(UpdateOp::Update, WIDGET.id());
            img = Some((i, s));

            // present

            WIDGET
                .sub_var_layout(&MASK_FIT_VAR)
                .sub_var_layout(&MASK_ALIGN_VAR)
                .sub_var_layout(&MASK_OFFSET_VAR);
        }
        UiNodeOp::Deinit => {
            c.deinit();
            img = None;
        }
        UiNodeOp::Update { .. } => {
            // load
            if source.is_new() || MASK_MODE_VAR.is_new() || MASK_IMAGE_DOWNSCALE_VAR.is_new() {
                let mut source = source.get();

                if let ImageSource::Render(_, args) = &mut source {
                    *args = Some(ImageRenderArgs::new(WINDOW.id()));
                }

                let mode = if MASK_IMAGE_CACHE_VAR.get() {
                    ImageCacheMode::Cache
                } else {
                    ImageCacheMode::Ignore
                };
                let limits = MASK_IMAGE_LIMITS_VAR.get();
                let downscale = MASK_IMAGE_DOWNSCALE_VAR.get();
                let mask_mode = MASK_MODE_VAR.get();
                let opt = ImageOptions::new(mode, downscale, Some(mask_mode), ImageEntriesMode::PRIMARY);
                let i = IMAGES.image(source, opt, limits);
                let s = i.subscribe(UpdateOp::Layout, WIDGET.id());
                img = Some((i, s));

                WIDGET.layout();
            } else if let Some(enabled) = MASK_IMAGE_CACHE_VAR.get_new() {
                // cache-mode update:
                let is_cached = img.as_ref().unwrap().0.with(|i| IMAGES.is_cached(i));
                if enabled != is_cached {
                    let i = if is_cached {
                        // must not cache, but is cached, detach from cache.

                        let img = img.take().unwrap().0;
                        IMAGES.detach(img)
                    } else {
                        // must cache, but image is not cached, get source again.

                        let source = source.get();
                        let limits = MASK_IMAGE_LIMITS_VAR.get();
                        let downscale = MASK_IMAGE_DOWNSCALE_VAR.get();
                        let mask_mode = MASK_MODE_VAR.get();
                        let opt = ImageOptions::new(ImageCacheMode::Cache, downscale, Some(mask_mode), ImageEntriesMode::PRIMARY);
                        IMAGES.image(source, opt, limits)
                    };

                    let s = i.subscribe(UpdateOp::Update, WIDGET.id());
                    img = Some((i, s));

                    WIDGET.layout();
                }
            } else if let Some(img) = img.as_ref().unwrap().0.get_new() {
                let s = img.size();
                if s != img_size {
                    img_size = s;
                    WIDGET.layout().render();
                } else {
                    WIDGET.render();
                }
            }
        }
        UiNodeOp::Layout { wl, final_size } => {
            *final_size = c.layout(wl);

            let wgt_size = *final_size;
            let constraints = PxConstraints2d::new_fill_size(wgt_size);
            LAYOUT.with_constraints(constraints, || {
                let mut img_size = img_size;
                let mut img_origin = PxPoint::zero();

                let mut fit = MASK_FIT_VAR.get();
                if let ImageFit::ScaleDown = fit {
                    if img_size.width < wgt_size.width && img_size.height < wgt_size.height {
                        fit = ImageFit::None;
                    } else {
                        fit = ImageFit::Contain;
                    }
                }

                let mut align = MASK_ALIGN_VAR.get();
                match fit {
                    ImageFit::Fill => {
                        align = Align::FILL;
                    }
                    ImageFit::Contain => {
                        let container = wgt_size.to_f32();
                        let content = img_size.to_f32();
                        let scale = (container.width / content.width).min(container.height / content.height).fct();
                        img_size *= scale;
                    }
                    ImageFit::Cover => {
                        let container = wgt_size.to_f32();
                        let content = img_size.to_f32();
                        let scale = (container.width / content.width).max(container.height / content.height).fct();
                        img_size *= scale;
                    }
                    ImageFit::None => {}
                    ImageFit::ScaleDown => unreachable!(),
                }

                if align.is_fill_x() {
                    let factor = wgt_size.width.0 as f32 / img_size.width.0 as f32;
                    img_size.width *= factor;
                } else {
                    let diff = wgt_size.width - img_size.width;
                    let offset = diff * align.x(LAYOUT.direction());
                    img_origin.x += offset;
                }
                if align.is_fill_y() {
                    let factor = wgt_size.height.0 as f32 / img_size.height.0 as f32;
                    img_size.height *= factor;
                } else {
                    let diff = wgt_size.height - img_size.height;
                    let offset = diff * align.y();
                    img_origin.y += offset;
                }

                img_origin += MASK_OFFSET_VAR.layout();

                let new_rect = PxRect::new(img_origin, img_size);
                if rect != new_rect {
                    rect = new_rect;
                    WIDGET.render();
                }
            });
        }
        UiNodeOp::Render { frame } => {
            if rect.size.is_empty() {
                return;
            }
            img.as_ref().unwrap().0.with(|img| {
                img.with_best_reduce(rect.size, |img| {
                    frame.push_mask(img, rect, |frame| c.render(frame));
                });
            });
        }
        _ => {}
    })
}

context_var! {
    /// Defines how the A8 image mask pixels are to be derived from a source mask image.
    pub static MASK_MODE_VAR: ImageMaskMode = ImageMaskMode::default();

    /// Defines if the mask image is cached.
    pub static MASK_IMAGE_CACHE_VAR: bool = true;

    /// Custom mask image load and decode limits.
    ///
    /// Set to `None` to use the `IMAGES::limits`.
    pub static MASK_IMAGE_LIMITS_VAR: Option<ImageLimits> = None;

    /// Custom resize applied during mask image decode.
    ///
    /// Is `None` by default.
    pub static MASK_IMAGE_DOWNSCALE_VAR: Option<ImageDownscaleMode> = None;

    /// Defines what mask images are decoded from multi image containers.
    pub static MASK_IMAGE_ENTRIES_MODE_VAR: ImageEntriesMode = ImageEntriesMode::PRIMARY;

    /// Defines how the mask image fits the widget bounds.
    pub static MASK_FIT_VAR: ImageFit = ImageFit::Fill;

    /// Align of the mask image in relation to the image widget final size.
    ///
    /// Is `Align::CENTER` by default.
    pub static MASK_ALIGN_VAR: Align = Align::CENTER;

    /// Offset applied to the mask image after all measure and arrange.
    pub static MASK_OFFSET_VAR: Vector = Vector::default();
}

/// Defines how the A8 image mask pixels are to be derived from a source mask image in all [`mask_image`] inside
/// the widget in descendants.
///
/// This property sets the [`MASK_MODE_VAR`].
///
/// [`mask_image`]: fn@mask_image
#[property(CONTEXT, default(MASK_MODE_VAR))]
pub fn mask_mode(child: impl IntoUiNode, mode: impl IntoVar<ImageMaskMode>) -> UiNode {
    with_context_var(child, MASK_MODE_VAR, mode)
}

/// Defines if the mask images loaded in all [`mask_image`] inside
/// the widget in descendants are cached.
///
/// This property sets the [`MASK_IMAGE_CACHE_VAR`].
///
/// [`mask_image`]: fn@mask_image
#[property(CONTEXT, default(MASK_IMAGE_CACHE_VAR))]
pub fn mask_image_cache(child: impl IntoUiNode, enabled: impl IntoVar<bool>) -> UiNode {
    with_context_var(child, MASK_IMAGE_CACHE_VAR, enabled)
}

/// Sets custom mask image load and decode limits.
///
/// If not set or set to `None` the [`IMAGES.limits`] is used.
///
/// This property sets the [`MASK_IMAGE_LIMITS_VAR`].
///
/// [`IMAGES.limits`]: zng_ext_image::IMAGES::limits
#[property(CONTEXT, default(MASK_IMAGE_LIMITS_VAR))]
pub fn mask_image_limits(child: impl IntoUiNode, limits: impl IntoVar<Option<ImageLimits>>) -> UiNode {
    with_context_var(child, MASK_IMAGE_LIMITS_VAR, limits)
}

/// Custom pixel resize applied during mask image load/decode.
///
/// Note that this resize affects the image actual pixel size directly when it is loading, it can also generate multiple image entries.
///
/// If the image is smaller than the requested size it is not upscaled. If multiple downscale samples are requested they are generated as
/// synthetic [`ImageEntryKind::Reduced`].
///
/// Changing this value after an image is already loaded or loading will cause the mask image to reload, image cache allocates different
/// entries for different downscale values, prefer setting samples of all possible sizes at once to
/// avoid generating multiple image entries in the cache.
///
/// This property sets the [`MASK_IMAGE_DOWNSCALE_VAR`].
///
/// [`IMAGES.limits`]: zng_ext_image::IMAGES::limits
/// [`img_limits`]: fn@crate::img_limits
/// [`ImageEntryKind::Reduced`]: zng_ext_image::ImageEntryKind
#[property(CONTEXT, default(MASK_IMAGE_DOWNSCALE_VAR))]
pub fn mask_image_downscale(child: impl IntoUiNode, downscale: impl IntoVar<Option<ImageDownscaleMode>>) -> UiNode {
    with_context_var(child, MASK_IMAGE_DOWNSCALE_VAR, downscale)
}

/// Defines what mask images are decoded from multi image containers.
///
/// By default container types like TIFF or ICO only decode the first/largest image, this property
/// defines if other contained images are also requested.
///
/// If the image contains a [`Reduced`] alternate the best size is used during rendering, this is particularly
/// useful for displaying icon files that have symbolic alternates that are more readable at a smaller size.
///
///
/// This property sets the [`MASK_IMAGE_ENTRIES_MODE_VAR`].
///
/// [`Reduced`]: zng_ext_image::ImageEntryKind::Reduced
/// [`img_downscale`]: fn@[`img_downscale`]
#[property(CONTEXT, default(MASK_IMAGE_ENTRIES_MODE_VAR))]
pub fn mask_image_entries_mode(child: impl IntoUiNode, mode: impl IntoVar<ImageEntriesMode>) -> UiNode {
    with_context_var(child, MASK_IMAGE_ENTRIES_MODE_VAR, mode)
}

/// Defines how the mask image fits the widget bounds in all [`mask_image`] inside
/// the widget in descendants.
///
/// This property sets the [`MASK_FIT_VAR`].
///
/// [`mask_image`]: fn@mask_image
#[property(CONTEXT, default(MASK_FIT_VAR))]
pub fn mask_fit(child: impl IntoUiNode, fit: impl IntoVar<ImageFit>) -> UiNode {
    with_context_var(child, MASK_FIT_VAR, fit)
}

/// Defines the align of the mask image in relation to the widget bounds in all [`mask_image`] inside
/// the widget in descendants.
///
/// This property sets the [`MASK_ALIGN_VAR`].
///
/// [`mask_image`]: fn@mask_image
#[property(CONTEXT, default(MASK_ALIGN_VAR))]
pub fn mask_align(child: impl IntoUiNode, align: impl IntoVar<Align>) -> UiNode {
    with_context_var(child, MASK_ALIGN_VAR, align)
}

/// Defines the offset applied to the mask image after all measure and arrange. in all [`mask_image`] inside
/// the widget in descendants.
///
/// This property sets the [`MASK_OFFSET_VAR`].
///
/// [`mask_image`]: fn@mask_image
#[property(CONTEXT, default(MASK_OFFSET_VAR))]
pub fn mask_offset(child: impl IntoUiNode, offset: impl IntoVar<Vector>) -> UiNode {
    with_context_var(child, MASK_OFFSET_VAR, offset)
}
