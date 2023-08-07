//! Mask properties, [`mask_image`], [`mask_mode`] and more.
//!
//! [`mask_image`]: fn@mask_image
//! [`mask_mode`]: fn@mask_mode

use crate::core::image::{ImageCacheMode, ImageMaskMode, ImageSource, IMAGES};
use crate::prelude::image::ImageRepeat;
use crate::prelude::{new_property::*, ImageFit};

/// Sets an image mask.
///
/// The image alpha channel is used as a mask for the widget and descendants.
///
/// This property is configured by contextual values set by [`mask_mode`], [`mask_image_cache`].
///
/// [`mask_mode`]: fn@mask_mode
/// [`mask_image_cache`]: fn@mask_image_cache
#[property(FILL-1)]
pub fn mask_image(child: impl UiNode, mask: impl IntoVar<ImageSource>) -> impl UiNode {
    let mask = mask.into_var();
    let mut img = None;
    let mut size = PxSize::zero();
    match_node(child, move |c, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var(&mask).sub_var(&MASK_MODE_VAR);
            let i = IMAGES.image(mask.get(), ImageCacheMode::Cache, None, None, Some(MASK_MODE_VAR.get()));
            let s = i.subscribe(UpdateOp::Render, WIDGET.id());
            img = Some((i, s));
        }
        UiNodeOp::Update { .. } => {
            if mask.is_new() || MASK_MODE_VAR.is_new() {
                let i = IMAGES.image(mask.get(), ImageCacheMode::Cache, None, None, Some(MASK_MODE_VAR.get()));
                let s = i.subscribe(UpdateOp::Render, WIDGET.id());
                img = Some((i, s));
                WIDGET.render();
            }
        }
        UiNodeOp::Layout { wl, final_size } => {
            *final_size = c.layout(wl);
            size = *final_size;
        }
        UiNodeOp::Render { frame } => {
            if let Some((img, _)) = &img {
                img.with(|img| {
                    frame.push_clip_mask(img, PxRect::from_size(size), |frame| {
                        c.render(frame);
                    })
                });
            }
        }
        _ => {}
    })
}

context_var! {
    /// Defines how the A8 image mask pixels are to be derived from a source mask image.
    pub static MASK_MODE_VAR: ImageMaskMode = ImageMaskMode::default();

    /// Defines how the mask image is loaded and cached.
    pub static MASK_IMAGE_CACHE_VAR: ImageCacheMode = ImageCacheMode::Cache;

    /// Defines how the mask image fits the widget bounds.
    pub static MASK_FIT_VAR: ImageFit = ImageFit::Fill;

    /// Defines how the mask image is repeated in the widget bounds.
    pub static MASK_REPEAT_VAR: ImageRepeat = ImageRepeat::None;

    /// Defines the spacing between repeated mask image copies.
    ///
    /// is [`Size::zero()`] by default.
    pub static MASK_REPEAT_SPACING_VAR: Size = Size::zero();

    /// Simple clip applied to the mask before layout.
    ///
    /// No cropping is done by default.
    pub static MASK_CROP_VAR: Rect = Rect::default();

    /// Align of the mask image in relation to the image widget final size.
    ///
    /// Is [`Align::CENTER`] by default.
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
pub fn mask_mode(child: impl UiNode, mode: impl IntoVar<ImageMaskMode>) -> impl UiNode {
    with_context_var(child, MASK_MODE_VAR, mode)
}

/// Defines how the mask images loaded and cached in all [`mask_image`] inside
/// the widget in descendants.
///
/// This property sets the [`MASK_IMAGE_CACHE_VAR`].
///
/// [`mask_image`]: fn@mask_image
#[property(CONTEXT, default(MASK_IMAGE_CACHE_VAR))]
pub fn mask_image_cache(child: impl UiNode, mode: impl IntoVar<ImageCacheMode>) -> impl UiNode {
    with_context_var(child, MASK_IMAGE_CACHE_VAR, mode)
}

/// Defines how the mask image fits the widget bounds in all [`mask_image`] inside
/// the widget in descendants.
///
/// This property sets the [`MASK_FIT_VAR`].
///
/// [`mask_image`]: fn@mask_image
#[property(CONTEXT, default(MASK_FIT_VAR))]
pub fn mask_fit(child: impl UiNode, fit: impl IntoVar<ImageFit>) -> impl UiNode {
    with_context_var(child, MASK_FIT_VAR, fit)
}

/// Defines how the mask image is repeated in the widget bounds in all [`mask_image`] inside
/// the widget in descendants.
///
/// This property sets the [`MASK_REPEAT_VAR`].
///
/// [`mask_image`]: fn@mask_image
#[property(CONTEXT, default(MASK_REPEAT_VAR))]
pub fn mask_repeat(child: impl UiNode, repeat: impl IntoVar<ImageRepeat>) -> impl UiNode {
    with_context_var(child, MASK_REPEAT_VAR, repeat)
}

/// Defines the spacing between repeated mask image copies in all [`mask_image`] inside
/// the widget in descendants.
///
/// This property sets the [`MASK_REPEAT_VAR`].
///
/// [`mask_image`]: fn@mask_image
#[property(CONTEXT, default(MASK_REPEAT_SPACING_VAR))]
pub fn mask_repeat_spacing(child: impl UiNode, spacing: impl IntoVar<Size>) -> impl UiNode {
    with_context_var(child, MASK_REPEAT_SPACING_VAR, spacing)
}

/// Defines a simple clip applied to the mask image before layout in all [`mask_image`] inside
/// the widget in descendants.
///
/// This property sets the [`MASK_CROP_VAR`].
///
/// [`mask_image`]: fn@mask_image
#[property(CONTEXT, default(MASK_CROP_VAR))]
pub fn mask_crop(child: impl UiNode, crop: impl IntoVar<Rect>) -> impl UiNode {
    with_context_var(child, MASK_CROP_VAR, crop)
}

/// Defines the align of the mask image in relation to the widget bounds in all [`mask_image`] inside
/// the widget in descendants.
///
/// This property sets the [`MASK_ALIGN_VAR`].
///
/// [`mask_image`]: fn@mask_image
#[property(CONTEXT, default(MASK_ALIGN_VAR))]
pub fn mask_align(child: impl UiNode, align: impl IntoVar<Align>) -> impl UiNode {
    with_context_var(child, MASK_ALIGN_VAR, align)
}

/// Defines the offset applied to the mask image after all measure and arrange. in all [`mask_image`] inside
/// the widget in descendants.
///
/// This property sets the [`MASK_OFFSET_VAR`].
///
/// [`mask_image`]: fn@mask_image
#[property(CONTEXT, default(MASK_OFFSET_VAR))]
pub fn mask_offset(child: impl UiNode, offset: impl IntoVar<Vector>) -> impl UiNode {
    with_context_var(child, MASK_OFFSET_VAR, offset)
}
