//! Background/foreground image properties.
//!
//! Implemented on this case instead of `zng-wgt-fill` to avoid cyclic dependencies.

use zng_ext_image::ImageSource;
use zng_wgt::prelude::*;
use zng_wgt_fill::{background, foreground};

use crate::{ImageFit, ImageRepeat};

/// Background image.
///
/// This property applies an image as [`background`].
///
/// The image can be configured by the same `img_*` contextual properties that affect [`Image!`] widgets,
/// a subset of image properties have `background_img_*` equivalent that only affect background images. You
/// can also adjust the opacity of the image using [`background_img_opacity`].
///
/// Note that you can use you can always set [`background`] or [`background_fn`] to an [`Image!`] widget directly to
/// access advanced image configuration properties.
///
/// [`background`]: fn@background
/// [`background_fn`]: fn@zng_wgt_fill::background
/// [`background_img_opacity`]: fn@background_img_opacity
/// [`Image!`]: struct@crate::Image
#[property(FILL)]
pub fn background_img(child: impl IntoUiNode, source: impl IntoVar<ImageSource>) -> UiNode {
    background(
        child,
        fill_img_node(
            source.into_var(),
            BACKGROUND_IMG_FIT_VAR,
            BACKGROUND_IMG_ALIGN_VAR,
            BACKGROUND_IMG_OFFSET_VAR,
            BACKGROUND_IMG_CROP_VAR,
            BACKGROUND_IMG_REPEAT_VAR,
            BACKGROUND_IMG_REPEAT_SPACING_VAR,
            BACKGROUND_IMG_OPACITY_VAR,
        ),
    )
}

/// Sets the background image fit.
///
/// This property sets the [`BACKGROUND_IMG_FIT_VAR`] and overrides the [`img_fit`] value for the background images.
///
/// [`img_fit`]: fn@crate::img_fit
#[property(CONTEXT, default(BACKGROUND_IMG_FIT_VAR))]
pub fn background_img_fit(child: impl IntoUiNode, fit: impl IntoVar<ImageFit>) -> UiNode {
    with_context_var(child, BACKGROUND_IMG_FIT_VAR, fit)
}

/// Sets the background image alignment.
///
/// This property sets the [`BACKGROUND_IMG_ALIGN_VAR`] and overrides the [`img_align`] value for the background images.
///
/// [`img_align`]: fn@crate::img_align
#[property(CONTEXT, default(BACKGROUND_IMG_ALIGN_VAR))]
pub fn background_img_align(child: impl IntoUiNode, align: impl IntoVar<Align>) -> UiNode {
    with_context_var(child, BACKGROUND_IMG_ALIGN_VAR, align)
}

/// Sets the background image offset.
///
/// This property sets the [`BACKGROUND_IMG_OFFSET_VAR`] and overrides the [`img_offset`] value for the background images.
///
/// [`img_offset`]: fn@crate::img_offset
#[property(CONTEXT, default(BACKGROUND_IMG_OFFSET_VAR))]
pub fn background_img_offset(child: impl IntoUiNode, offset: impl IntoVar<Vector>) -> UiNode {
    with_context_var(child, BACKGROUND_IMG_OFFSET_VAR, offset)
}

/// Sets the background image crop.
///
/// This property sets the [`BACKGROUND_IMG_CROP_VAR`] and overrides the [`img_crop`] value for the background images.
///
/// [`img_crop`]: fn@crate::img_crop
#[property(CONTEXT, default(BACKGROUND_IMG_CROP_VAR))]
pub fn background_img_crop(child: impl IntoUiNode, crop: impl IntoVar<Rect>) -> UiNode {
    with_context_var(child, BACKGROUND_IMG_CROP_VAR, crop)
}

/// Sets the background image repeat.
///
/// This property sets the [`BACKGROUND_IMG_REPEAT_VAR`] and overrides the [`img_repeat`] value for the background images.
///
/// [`img_repeat`]: fn@crate::img_repeat
#[property(CONTEXT, default(BACKGROUND_IMG_REPEAT_VAR))]
pub fn background_img_repeat(child: impl IntoUiNode, repeat: impl IntoVar<ImageRepeat>) -> UiNode {
    with_context_var(child, BACKGROUND_IMG_REPEAT_VAR, repeat)
}

/// Sets the background image repeat spacing.
///
/// This property sets the [`BACKGROUND_IMG_REPEAT_SPACING_VAR`] and overrides the [`img_repeat_spacing`] value for the background images.
///
/// [`img_repeat_spacing`]: fn@crate::img_repeat_spacing
#[property(CONTEXT, default(BACKGROUND_IMG_REPEAT_SPACING_VAR))]
pub fn background_img_repeat_spacing(child: impl IntoUiNode, spacing: impl IntoVar<Size>) -> UiNode {
    with_context_var(child, BACKGROUND_IMG_REPEAT_SPACING_VAR, spacing)
}

/// Sets the background image opacity.
///
/// This property sets the [`BACKGROUND_IMG_OPACITY_VAR`].
///
/// [`img_repeat_spacing`]: fn@crate::img_repeat_spacing
#[property(CONTEXT, default(BACKGROUND_IMG_OPACITY_VAR))]
pub fn background_img_opacity(child: impl IntoUiNode, alpha: impl IntoVar<Factor>) -> UiNode {
    with_context_var(child, BACKGROUND_IMG_OPACITY_VAR, alpha)
}

context_var! {
    /// The background image layout mode.
    ///
    /// Is [`IMAGE_FIT_VAR`] by default.
    ///
    /// [`IMAGE_FIT_VAR`]: crate::IMAGE_FIT_VAR
    pub static BACKGROUND_IMG_FIT_VAR: ImageFit = crate::IMAGE_FIT_VAR;

    /// Align of the background image in relation to the widget final size.
    ///
    /// Is [`IMAGE_ALIGN_VAR`] by default.
    ///
    /// [`IMAGE_ALIGN_VAR`]: crate::IMAGE_ALIGN_VAR
    pub static BACKGROUND_IMG_ALIGN_VAR: Align = crate::IMAGE_ALIGN_VAR;

    /// Offset applied to the background image after all measure and arrange.
    ///
    /// Is [`IMAGE_OFFSET_VAR`] by default.
    ///
    /// [`IMAGE_OFFSET_VAR`]: crate::IMAGE_OFFSET_VAR
    pub static BACKGROUND_IMG_OFFSET_VAR: Vector = crate::IMAGE_OFFSET_VAR;

    /// Simple clip applied to the background image before layout.
    ///
    /// Is [`IMAGE_CROP_VAR`] by default.
    ///
    /// [`IMAGE_CROP_VAR`]: crate::IMAGE_CROP_VAR
    pub static BACKGROUND_IMG_CROP_VAR: Rect = crate::IMAGE_CROP_VAR;

    /// Pattern repeat applied on the final background image.
    ///
    /// Is [`IMAGE_REPEAT_VAR`] by default.
    ///
    /// [`IMAGE_REPEAT_VAR`]: crate::IMAGE_REPEAT_VAR
    pub static BACKGROUND_IMG_REPEAT_VAR: ImageRepeat = crate::IMAGE_REPEAT_VAR;

    /// Spacing between repeated background image copies.
    ///
    /// Is [`IMAGE_REPEAT_SPACING_VAR`] by default.
    ///
    /// [`IMAGE_REPEAT_SPACING_VAR`]: crate::IMAGE_REPEAT_SPACING_VAR
    pub static BACKGROUND_IMG_REPEAT_SPACING_VAR: Size = crate::IMAGE_REPEAT_SPACING_VAR;

    /// Opacity of the background image.
    ///
    /// Is `100.pct()` by default.
    pub static BACKGROUND_IMG_OPACITY_VAR: Factor = 100.pct();
}

/// Foreground image.
///
/// This property applies an image as [`foreground`].
///
/// The image can be configured by the same `img_*` contextual properties that affect [`Image!`] widgets,
/// a subset of image properties have `foreground_img_*` equivalent that only affect foreground images. You
/// can also adjust the opacity of the image using [`foreground_img_opacity`].
///
/// Note that you can use you can always set [`foreground`] or [`foreground_fn`] to an [`Image!`] widget directly to
/// access advanced image configuration properties.
///
/// [`foreground`]: fn@foreground
/// [`foreground_fn`]: fn@zng_wgt_fill::foreground
/// [`foreground_img_opacity`]: fn@foreground_img_opacity
/// [`Image!`]: struct@crate::Image
#[property(FILL)]
pub fn foreground_img(child: impl IntoUiNode, source: impl IntoVar<ImageSource>) -> UiNode {
    foreground(
        child,
        fill_img_node(
            source.into_var(),
            FOREGROUND_IMG_FIT_VAR,
            FOREGROUND_IMG_ALIGN_VAR,
            FOREGROUND_IMG_OFFSET_VAR,
            FOREGROUND_IMG_CROP_VAR,
            FOREGROUND_IMG_REPEAT_VAR,
            FOREGROUND_IMG_REPEAT_SPACING_VAR,
            FOREGROUND_IMG_OPACITY_VAR,
        ),
    )
}

/// Sets the foreground image fit.
///
/// This property sets the [`FOREGROUND_IMG_FIT_VAR`] and overrides the [`img_fit`] value for the foreground images.
///
/// [`img_fit`]: fn@crate::img_fit
#[property(CONTEXT, default(FOREGROUND_IMG_FIT_VAR))]
pub fn foreground_img_fit(child: impl IntoUiNode, fit: impl IntoVar<ImageFit>) -> UiNode {
    with_context_var(child, FOREGROUND_IMG_FIT_VAR, fit)
}

/// Sets the foreground image alignment.
///
/// This property sets the [`FOREGROUND_IMG_ALIGN_VAR`] and overrides the [`img_align`] value for the foreground images.
///
/// [`img_align`]: fn@crate::img_align
#[property(CONTEXT, default(FOREGROUND_IMG_ALIGN_VAR))]
pub fn foreground_img_align(child: impl IntoUiNode, align: impl IntoVar<Align>) -> UiNode {
    with_context_var(child, FOREGROUND_IMG_ALIGN_VAR, align)
}

/// Sets the foreground image offset.
///
/// This property sets the [`FOREGROUND_IMG_OFFSET_VAR`] and overrides the [`img_offset`] value for the foreground images.
///
/// [`img_offset`]: fn@crate::img_offset
#[property(CONTEXT, default(FOREGROUND_IMG_OFFSET_VAR))]
pub fn foreground_img_offset(child: impl IntoUiNode, offset: impl IntoVar<Vector>) -> UiNode {
    with_context_var(child, FOREGROUND_IMG_OFFSET_VAR, offset)
}

/// Sets the foreground image crop.
///
/// This property sets the [`FOREGROUND_IMG_CROP_VAR`] and overrides the [`img_crop`] value for the foreground images.
///
/// [`img_crop`]: fn@crate::img_crop
#[property(CONTEXT, default(FOREGROUND_IMG_CROP_VAR))]
pub fn foreground_img_crop(child: impl IntoUiNode, crop: impl IntoVar<Rect>) -> UiNode {
    with_context_var(child, FOREGROUND_IMG_CROP_VAR, crop)
}

/// Sets the foreground image repeat.
///
/// This property sets the [`FOREGROUND_IMG_REPEAT_VAR`] and overrides the [`img_repeat`] value for the foreground images.
///
/// [`img_repeat`]: fn@crate::img_repeat
#[property(CONTEXT, default(FOREGROUND_IMG_REPEAT_VAR))]
pub fn foreground_img_repeat(child: impl IntoUiNode, repeat: impl IntoVar<ImageRepeat>) -> UiNode {
    with_context_var(child, FOREGROUND_IMG_REPEAT_VAR, repeat)
}

/// Sets the foreground image repeat spacing.
///
/// This property sets the [`FOREGROUND_IMG_REPEAT_SPACING_VAR`] and overrides the [`img_repeat_spacing`] value for the foreground images.
///
/// [`img_repeat_spacing`]: fn@crate::img_repeat_spacing
#[property(CONTEXT, default(FOREGROUND_IMG_REPEAT_SPACING_VAR))]
pub fn foreground_img_repeat_spacing(child: impl IntoUiNode, spacing: impl IntoVar<Size>) -> UiNode {
    with_context_var(child, FOREGROUND_IMG_REPEAT_SPACING_VAR, spacing)
}

/// Sets the foreground image opacity.
///
/// This property sets the [`FOREGROUND_IMG_OPACITY_VAR`].
///
/// [`img_repeat_spacing`]: fn@crate::img_repeat_spacing
#[property(CONTEXT, default(FOREGROUND_IMG_OPACITY_VAR))]
pub fn foreground_img_opacity(child: impl IntoUiNode, alpha: impl IntoVar<Factor>) -> UiNode {
    with_context_var(child, FOREGROUND_IMG_OPACITY_VAR, alpha)
}

context_var! {
    /// The foreground image layout mode.
    ///
    /// Is [`IMAGE_FIT_VAR`] by default.
    ///
    /// [`IMAGE_FIT_VAR`]: crate::IMAGE_FIT_VAR
    pub static FOREGROUND_IMG_FIT_VAR: ImageFit = crate::IMAGE_FIT_VAR;

    /// Align of the foreground image in relation to the widget final size.
    ///
    /// Is [`IMAGE_ALIGN_VAR`] by default.
    ///
    /// [`IMAGE_ALIGN_VAR`]: crate::IMAGE_ALIGN_VAR
    pub static FOREGROUND_IMG_ALIGN_VAR: Align = crate::IMAGE_ALIGN_VAR;

    /// Offset applied to the foreground image after all measure and arrange.
    ///
    /// Is [`IMAGE_OFFSET_VAR`] by default.
    ///
    /// [`IMAGE_OFFSET_VAR`]: crate::IMAGE_OFFSET_VAR
    pub static FOREGROUND_IMG_OFFSET_VAR: Vector = crate::IMAGE_OFFSET_VAR;

    /// Simple clip applied to the foreground image before layout.
    ///
    /// Is [`IMAGE_CROP_VAR`] by default.
    ///
    /// [`IMAGE_CROP_VAR`]: crate::IMAGE_CROP_VAR
    pub static FOREGROUND_IMG_CROP_VAR: Rect = crate::IMAGE_CROP_VAR;

    /// Pattern repeat applied on the final foreground image.
    ///
    /// Is [`IMAGE_REPEAT_VAR`] by default.
    ///
    /// [`IMAGE_REPEAT_VAR`]: crate::IMAGE_REPEAT_VAR
    pub static FOREGROUND_IMG_REPEAT_VAR: ImageRepeat = crate::IMAGE_REPEAT_VAR;

    /// Spacing between repeated foreground image copies.
    ///
    /// Is [`IMAGE_REPEAT_SPACING_VAR`] by default.
    ///
    /// [`IMAGE_REPEAT_SPACING_VAR`]: crate::IMAGE_REPEAT_SPACING_VAR
    pub static FOREGROUND_IMG_REPEAT_SPACING_VAR: Size = crate::IMAGE_REPEAT_SPACING_VAR;

    /// Opacity of the foreground image.
    ///
    /// Is `100.pct()` by default.
    pub static FOREGROUND_IMG_OPACITY_VAR: Factor = 100.pct();
}

#[allow(clippy::too_many_arguments)]
fn fill_img_node(
    source: impl IntoVar<ImageSource>,
    img_fit: impl IntoVar<ImageFit>,
    img_align: impl IntoVar<Align>,
    img_offset: impl IntoVar<Vector>,
    img_crop: impl IntoVar<Rect>,
    img_repeat: impl IntoVar<ImageRepeat>,
    img_repeat_spacing: impl IntoVar<Size>,
    img_opacity: impl IntoVar<Factor>,
) -> UiNode {
    use crate::node;
    // child
    let child = node::image_presenter();
    let child = node::image_error_presenter(child);
    let child = node::image_loading_presenter(child);
    let child = opacity_node(child, img_opacity);

    // event
    let child = node::image_source(child, source);

    // context
    let child = crate::img_fit(child, img_fit);
    let child = crate::img_align(child, img_align);
    let child = crate::img_offset(child, img_offset);
    let child = crate::img_crop(child, img_crop);
    let child = crate::img_repeat(child, img_repeat);
    crate::img_repeat_spacing(child, img_repeat_spacing)
}

// very similar to `child_opacity`, code copied here to avoid importing the full `zng-wgt-filter`.
fn opacity_node(child: impl IntoUiNode, alpha: impl IntoVar<Factor>) -> UiNode {
    let frame_key = FrameValueKey::new_unique();
    let alpha = alpha.into_var();

    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_render_update(&alpha);
        }
        UiNodeOp::Render { frame } => {
            let opacity = frame_key.bind_var(&alpha, |f| f.0);
            frame.push_opacity(opacity, |frame| child.render(frame));
        }
        UiNodeOp::RenderUpdate { update } => {
            update.update_f32_opt(frame_key.update_var(&alpha, |f| f.0));
            child.render_update(update);
        }
        _ => {}
    })
}
