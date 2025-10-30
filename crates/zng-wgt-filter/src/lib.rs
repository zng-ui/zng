#![doc(html_favicon_url = "https://zng-ui.github.io/res/zng-logo-icon.png")]
#![doc(html_logo_url = "https://zng-ui.github.io/res/zng-logo.png")]
//!
//! Color filter properties, [`opacity`](fn@opacity), [`filter`](fn@filter) and more.
//!
//! # Crate
//!
#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]
#![warn(unused_extern_crates)]
#![warn(missing_docs)]

use zng_color::filter::{ColorMatrix, Filter};
use zng_wgt::prelude::*;

/// Color filter, or combination of filters.
///
/// This property allows setting multiple filters at once, there is also a property for every
/// filter for easier value updating.
///
/// # Performance
///
/// The performance for setting specific filter properties versus this one is the same, except for [`opacity`]
/// which can be animated using only frame updates instead of generating a new frame every change.
///
/// [`opacity`]: fn@opacity
#[property(CONTEXT, default(Filter::default()))]
pub fn filter(child: impl IntoUiNode, filter: impl IntoVar<Filter>) -> UiNode {
    filter_any(child, filter, false)
}

/// Backdrop filter, or combination of filters.
///
/// This property allows setting multiple filters at once, there is also a property for every
/// filter for easier value updating.
///
/// The filters are applied to everything rendered behind the widget.
///
/// # Performance
///
/// The performance for setting specific filter properties versus this one is the same.
///
/// [`opacity`]: fn@opacity
#[property(CONTEXT, default(Filter::default()))]
pub fn backdrop_filter(child: impl IntoUiNode, filter: impl IntoVar<Filter>) -> UiNode {
    backdrop_filter_any(child, filter)
}

/// Color filter, or combination of filters targeting the widget's descendants and not the widget itself.
///
/// This property allows setting multiple filters at once, there is also a property for every
/// filter for easier value updating.
///
/// # Performance
///
/// The performance for setting specific filter properties versus this one is the same, except for [`child_opacity`]
/// which can be animated using only frame updates instead of generating a new frame every change.
///
/// [`child_opacity`]: fn@child_opacity
#[property(CHILD_CONTEXT, default(Filter::default()))]
pub fn child_filter(child: impl IntoUiNode, filter: impl IntoVar<Filter>) -> UiNode {
    filter_any(child, filter, true)
}

/// Inverts the colors of the widget.
///
/// Zero does not invert, one fully inverts.
///
/// This property is a shorthand way of setting [`filter`] to [`Filter::new_invert`] using variable mapping.
///
/// [`filter`]: fn@filter
/// [`Filter::new_invert`]: zng_color::filter::Filter::new_invert
#[property(CONTEXT, default(false))]
pub fn invert_color(child: impl IntoUiNode, amount: impl IntoVar<Factor>) -> UiNode {
    filter_render(child, amount.into_var().map(|&a| Filter::new_invert(a)), false)
}

/// Inverts the colors of everything behind the widget.
///
/// Zero does not invert, one fully inverts.
///
/// This property is a shorthand way of setting [`backdrop_filter`] to [`Filter::new_invert`] using variable mapping.
///
/// [`backdrop_filter`]: fn@backdrop_filter
/// [`Filter::new_invert`]: zng_color::filter::Filter::new_invert
#[property(CONTEXT, default(false))]
pub fn backdrop_invert(child: impl IntoUiNode, amount: impl IntoVar<Factor>) -> UiNode {
    backdrop_filter_render(child, amount.into_var().map(|&a| Filter::new_invert(a)))
}

/// Blur the widget.
///
/// This property is a shorthand way of setting [`filter`] to [`Filter::new_blur`] using variable mapping.
///
/// [`filter`]: fn@filter
/// [`Filter::new_blur`]: zng_color::filter::Filter::new_blur
#[property(CONTEXT, default(0))]
pub fn blur(child: impl IntoUiNode, radius: impl IntoVar<Length>) -> UiNode {
    filter_layout(child, radius.into_var().map(|r| Filter::new_blur(r.clone())), false)
}

/// Blur the everything behind the widget.
///
/// This property is a shorthand way of setting [`backdrop_filter`] to [`Filter::new_blur`] using variable mapping.
///
/// [`backdrop_filter`]: fn@backdrop_filter
/// [`Filter::new_blur`]: zng_color::filter::Filter::new_blur
#[property(CONTEXT, default(0))]
pub fn backdrop_blur(child: impl IntoUiNode, radius: impl IntoVar<Length>) -> UiNode {
    backdrop_filter_layout(child, radius.into_var().map(|r| Filter::new_blur(r.clone())))
}

/// Sepia tone the widget.
///
/// zero is the original colors, one is the full desaturated brown look.
///
/// This property is a shorthand way of setting [`filter`] to [`Filter::new_sepia`] using variable mapping.
///
/// [`filter`]: fn@filter
/// [`Filter::new_sepia`]: zng_color::filter::Filter::new_sepia
#[property(CONTEXT, default(false))]
pub fn sepia(child: impl IntoUiNode, amount: impl IntoVar<Factor>) -> UiNode {
    filter_render(child, amount.into_var().map(|&a| Filter::new_sepia(a)), false)
}

/// Sepia tone everything behind the widget.
///
/// zero is the original colors, one is the full desaturated brown look.
///
/// This property is a shorthand way of setting [`backdrop_filter`] to [`Filter::new_sepia`] using variable mapping.
///
/// [`backdrop_filter`]: fn@backdrop_filter
/// [`Filter::new_sepia`]: zng_color::filter::Filter::new_sepia
#[property(CONTEXT, default(false))]
pub fn backdrop_sepia(child: impl IntoUiNode, amount: impl IntoVar<Factor>) -> UiNode {
    backdrop_filter_render(child, amount.into_var().map(|&a| Filter::new_sepia(a)))
}

/// Grayscale tone the widget.
///
/// Zero is the original colors, one if the full grayscale.
///
/// This property is a shorthand way of setting [`filter`] to [`Filter::new_grayscale`] using variable mapping.
///
/// [`filter`]: fn@filter
/// [`Filter::new_grayscale`]: zng_color::filter::Filter::new_grayscale
#[property(CONTEXT, default(false))]
pub fn grayscale(child: impl IntoUiNode, amount: impl IntoVar<Factor>) -> UiNode {
    filter_render(child, amount.into_var().map(|&a| Filter::new_grayscale(a)), false)
}

/// Grayscale tone everything behind the widget.
///
/// Zero is the original colors, one if the full grayscale.
///
/// This property is a shorthand way of setting [`backdrop_filter`] to [`Filter::new_grayscale`] using variable mapping.
///
/// [`backdrop_filter`]: fn@backdrop_filter
/// [`Filter::new_grayscale`]: zng_color::filter::Filter::new_grayscale
#[property(CONTEXT, default(false))]
pub fn backdrop_grayscale(child: impl IntoUiNode, amount: impl IntoVar<Factor>) -> UiNode {
    backdrop_filter_render(child, amount.into_var().map(|&a| Filter::new_grayscale(a)))
}

/// Drop-shadow effect for the widget.
///
/// The shadow is *pixel accurate*.
///
/// This property is a shorthand way of setting [`filter`] to [`Filter::new_drop_shadow`] using variable merging.
///
/// [`filter`]: fn@filter
/// [`Filter::new_drop_shadow`]: zng_color::filter::Filter::new_drop_shadow
#[property(CONTEXT, default((0, 0), 0, colors::BLACK.transparent()))]
pub fn drop_shadow(
    child: impl IntoUiNode,
    offset: impl IntoVar<Point>,
    blur_radius: impl IntoVar<Length>,
    color: impl IntoVar<Rgba>,
) -> UiNode {
    filter_layout(
        child,
        merge_var!(offset.into_var(), blur_radius.into_var(), color.into_var(), |o, r, &c| {
            Filter::new_drop_shadow(o.clone(), r.clone(), c)
        }),
        false,
    )
}

/// Adjust the widget colors brightness.
///
/// Zero removes all brightness, one is the original brightness.
///
/// This property is a shorthand way of setting [`filter`] to [`Filter::new_brightness`] using variable mapping.
///
/// [`filter`]: fn@filter
/// [`Filter::new_brightness`]: zng_color::filter::Filter::new_brightness
#[property(CONTEXT, default(1.0))]
pub fn brightness(child: impl IntoUiNode, amount: impl IntoVar<Factor>) -> UiNode {
    filter_render(child, amount.into_var().map(|&a| Filter::new_brightness(a)), false)
}

/// Adjust color brightness of everything behind the widget.
///
/// Zero removes all brightness, one is the original brightness.
///
/// This property is a shorthand way of setting [`backdrop_filter`] to [`Filter::new_brightness`] using variable mapping.
///
/// [`backdrop_filter`]: fn@backdrop_filter
/// [`Filter::new_brightness`]: zng_color::filter::Filter::new_brightness
#[property(CONTEXT, default(1.0))]
pub fn backdrop_brightness(child: impl IntoUiNode, amount: impl IntoVar<Factor>) -> UiNode {
    backdrop_filter_render(child, amount.into_var().map(|&a| Filter::new_brightness(a)))
}

/// Adjust the widget colors contrast.
///
/// Zero removes all contrast, one is the original contrast.
///
/// This property is a shorthand way of setting [`filter`] to [`Filter::new_contrast`] using variable mapping.
///
/// [`filter`]: fn@filter
/// [`Filter::new_contrast`]: zng_color::filter::Filter::new_contrast
#[property(CONTEXT, default(1.0))]
pub fn contrast(child: impl IntoUiNode, amount: impl IntoVar<Factor>) -> UiNode {
    filter_render(child, amount.into_var().map(|&a| Filter::new_contrast(a)), false)
}

/// Adjust the color contrast of everything behind the widget.
///
/// Zero removes all contrast, one is the original contrast.
///
/// This property is a shorthand way of setting [`backdrop_filter`] to [`Filter::new_contrast`] using variable mapping.
///
/// [`backdrop_filter`]: fn@backdrop_filter
/// [`Filter::new_contrast`]: zng_color::filter::Filter::new_contrast
#[property(CONTEXT, default(1.0))]
pub fn backdrop_contrast(child: impl IntoUiNode, amount: impl IntoVar<Factor>) -> UiNode {
    backdrop_filter_render(child, amount.into_var().map(|&a| Filter::new_contrast(a)))
}

/// Adjust the widget colors saturation.
///
/// Zero fully desaturates, one is the original saturation.
///
/// This property is a shorthand way of setting [`filter`] to [`Filter::new_saturate`] using variable mapping.
///
/// [`filter`]: fn@filter
/// [`Filter::new_saturate`]: zng_color::filter::Filter::new_saturate
#[property(CONTEXT, default(1.0))]
pub fn saturate(child: impl IntoUiNode, amount: impl IntoVar<Factor>) -> UiNode {
    filter_render(child, amount.into_var().map(|&a| Filter::new_saturate(a)), false)
}

/// Adjust color saturation of everything behind the widget.
///
/// Zero fully desaturates, one is the original saturation.
///
/// This property is a shorthand way of setting [`backdrop_filter`] to [`Filter::new_saturate`] using variable mapping.
///
/// [`backdrop_filter`]: fn@backdrop_filter
/// [`Filter::new_saturate`]: zng_color::filter::Filter::new_saturate
#[property(CONTEXT, default(1.0))]
pub fn backdrop_saturate(child: impl IntoUiNode, amount: impl IntoVar<Factor>) -> UiNode {
    backdrop_filter_render(child, amount.into_var().map(|&a| Filter::new_saturate(a)))
}

/// Hue shift the widget colors.
///
/// Adds `angle` to the [`hue`] of the widget colors.
///
/// This property is a shorthand way of setting [`filter`] to [`Filter::new_hue_rotate`] using variable mapping.
///
/// [`filter`]: fn@filter
/// [`hue`]: Hsla::hue
/// [`Filter::new_hue_rotate`]: zng_color::filter::Filter::new_hue_rotate
#[property(CONTEXT, default(0.deg()))]
pub fn hue_rotate(child: impl IntoUiNode, angle: impl IntoVar<AngleDegree>) -> UiNode {
    filter_render(child, angle.into_var().map(|&a| Filter::new_hue_rotate(a)), false)
}

/// Hue shift the colors behind the widget.
///
/// Adds `angle` to the [`hue`] of the widget colors.
///
/// This property is a shorthand way of setting [`backdrop_filter`] to [`Filter::new_hue_rotate`] using variable mapping.
///
/// [`backdrop_filter`]: fn@backdrop_filter
/// [`hue`]: Hsla::hue
/// [`Filter::new_hue_rotate`]: zng_color::filter::Filter::new_hue_rotate
#[property(CONTEXT, default(0.deg()))]
pub fn backdrop_hue_rotate(child: impl IntoUiNode, angle: impl IntoVar<AngleDegree>) -> UiNode {
    backdrop_filter_render(child, angle.into_var().map(|&a| Filter::new_hue_rotate(a)))
}

/// Custom color filter.
///
/// The color matrix is in the format of SVG color matrix, [0..5] is the first matrix row.
#[property(CONTEXT, default(ColorMatrix::identity()))]
pub fn color_matrix(child: impl IntoUiNode, matrix: impl IntoVar<ColorMatrix>) -> UiNode {
    filter_render(child, matrix.into_var().map(|&m| Filter::new_color_matrix(m)), false)
}

/// Custom backdrop filter.
///
/// The color matrix is in the format of SVG color matrix, [0..5] is the first matrix row.
#[property(CONTEXT, default(ColorMatrix::identity()))]
pub fn backdrop_color_matrix(child: impl IntoUiNode, matrix: impl IntoVar<ColorMatrix>) -> UiNode {
    backdrop_filter_render(child, matrix.into_var().map(|&m| Filter::new_color_matrix(m)))
}

/// Opacity/transparency of the widget.
///
/// This property provides the same visual result as setting [`filter`] to [`Filter::new_opacity`],
/// **but** updating the opacity is faster in this property.
///
/// [`filter`]: fn@filter
/// [`Filter::new_opacity`]: zng_color::filter::Filter::new_opacity
#[property(CONTEXT, default(1.0))]
pub fn opacity(child: impl IntoUiNode, alpha: impl IntoVar<Factor>) -> UiNode {
    opacity_impl(child, alpha, false)
}

/// Opacity/transparency of the widget's child.
///
/// This property provides the same visual result as setting [`child_filter`] to [`Filter::new_opacity`],
/// **but** updating the opacity is faster in this property.
///
/// [`child_filter`]: fn@child_filter
/// [`Filter::new_opacity`]: zng_color::filter::Filter::new_opacity
#[property(CHILD_CONTEXT, default(1.0))]
pub fn child_opacity(child: impl IntoUiNode, alpha: impl IntoVar<Factor>) -> UiNode {
    opacity_impl(child, alpha, true)
}

/// impl any filter, may need layout or not.
fn filter_any(child: impl IntoUiNode, filter: impl IntoVar<Filter>, target_child: bool) -> UiNode {
    let filter = filter.into_var();
    let mut render_filter = None;
    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var(&filter);
            render_filter = filter.with(Filter::try_render);
        }
        UiNodeOp::Update { .. } => {
            filter.with_new(|f| {
                if let Some(f) = f.try_render() {
                    render_filter = Some(f);
                    WIDGET.render();
                } else {
                    render_filter = None;
                    WIDGET.layout();
                }
            });
        }
        UiNodeOp::Layout { .. } => {
            filter.with(|f| {
                if f.needs_layout() {
                    let f = Some(f.layout());
                    if render_filter != f {
                        render_filter = f;
                        WIDGET.render();
                    }
                }
            });
        }
        UiNodeOp::Render { frame } => {
            if target_child {
                frame.push_filter(MixBlendMode::Normal, render_filter.as_ref().unwrap(), |frame| child.render(frame));
            } else {
                frame.push_inner_filter(render_filter.clone().unwrap(), |frame| child.render(frame));
            }
        }
        _ => {}
    })
}

/// impl any backdrop filter, may need layout or not.
fn backdrop_filter_any(child: impl IntoUiNode, filter: impl IntoVar<Filter>) -> UiNode {
    let filter = filter.into_var();
    let mut render_filter = None;
    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var(&filter);
            render_filter = filter.with(Filter::try_render);
        }
        UiNodeOp::Update { .. } => {
            filter.with_new(|f| {
                if let Some(f) = f.try_render() {
                    render_filter = Some(f);
                    WIDGET.render();
                } else {
                    render_filter = None;
                    WIDGET.layout();
                }
            });
        }
        UiNodeOp::Layout { .. } => {
            filter.with(|f| {
                if f.needs_layout() {
                    let f = Some(f.layout());
                    if render_filter != f {
                        render_filter = f;
                        WIDGET.render();
                    }
                }
            });
        }
        UiNodeOp::Render { frame } => {
            frame.push_inner_backdrop_filter(render_filter.clone().unwrap(), |frame| child.render(frame));
        }
        _ => {}
    })
}

/// impl filters that need layout.
fn filter_layout(child: impl IntoUiNode, filter: impl IntoVar<Filter>, target_child: bool) -> UiNode {
    let filter = filter.into_var();

    let mut render_filter = None;
    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_layout(&filter);
        }
        UiNodeOp::Layout { .. } => {
            filter.with(|f| {
                debug_assert!(f.needs_layout());
                let f = Some(f.layout());
                if render_filter != f {
                    render_filter = f;
                    WIDGET.render();
                }
            });
        }
        UiNodeOp::Render { frame } => {
            if let Some(filter) = &render_filter {
                if target_child {
                    frame.push_filter(MixBlendMode::Normal, filter, |frame| child.render(frame));
                } else {
                    frame.push_inner_filter(filter.clone(), |frame| child.render(frame));
                }
            } else {
                tracing::error!("filter_layout render called before any layout");
            }
        }
        _ => {}
    })
}

/// impl backdrop filters that need layout.
fn backdrop_filter_layout(child: impl IntoUiNode, filter: impl IntoVar<Filter>) -> UiNode {
    let filter = filter.into_var();

    let mut render_filter = None;
    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_layout(&filter);
        }
        UiNodeOp::Layout { .. } => {
            filter.with(|f| {
                debug_assert!(f.needs_layout());

                let f = Some(f.layout());
                if render_filter != f {
                    render_filter = f;
                    WIDGET.render();
                }
            });
        }
        UiNodeOp::Render { frame } => {
            if let Some(filter) = render_filter.clone() {
                frame.push_inner_backdrop_filter(filter, |frame| child.render(frame));
            } else {
                tracing::error!("filter_layout render called before any layout");
            }
        }
        _ => {}
    })
}

/// impl filters that only need render.
fn filter_render(child: impl IntoUiNode, filter: impl IntoVar<Filter>, target_child: bool) -> UiNode {
    let filter = filter.into_var().map(|f| f.try_render().unwrap());
    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_render(&filter);
        }
        UiNodeOp::Render { frame } => {
            if target_child {
                filter.with(|f| {
                    frame.push_filter(MixBlendMode::Normal, f, |frame| child.render(frame));
                });
            } else {
                frame.push_inner_filter(filter.get(), |frame| child.render(frame));
            }
        }
        _ => {}
    })
}

/// impl backdrop filter that only need render.
fn backdrop_filter_render(child: impl IntoUiNode, filter: impl IntoVar<Filter>) -> UiNode {
    let filter = filter.into_var().map(|f| f.try_render().unwrap());
    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_render(&filter);
        }
        UiNodeOp::Render { frame } => {
            frame.push_inner_backdrop_filter(filter.get(), |frame| child.render(frame));
        }
        _ => {}
    })
}

fn opacity_impl(child: impl IntoUiNode, alpha: impl IntoVar<Factor>, target_child: bool) -> UiNode {
    let frame_key = FrameValueKey::new_unique();
    let alpha = alpha.into_var();

    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_render_update(&alpha);
        }
        UiNodeOp::Render { frame } => {
            let opacity = frame_key.bind_var(&alpha, |f| f.0);
            if target_child {
                frame.push_opacity(opacity, |frame| child.render(frame));
            } else {
                frame.push_inner_opacity(opacity, |frame| child.render(frame));
            }
        }
        UiNodeOp::RenderUpdate { update } => {
            update.update_f32_opt(frame_key.update_var(&alpha, |f| f.0));
            child.render_update(update);
        }
        _ => {}
    })
}

/// Sets how the widget blends with the parent widget.
#[property(CONTEXT, default(MixBlendMode::default()))]
pub fn mix_blend(child: impl IntoUiNode, mode: impl IntoVar<MixBlendMode>) -> UiNode {
    let mode = mode.into_var();
    match_node(child, move |c, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_render(&mode);
        }
        UiNodeOp::Render { frame } => {
            frame.push_inner_blend(mode.get(), |frame| c.render(frame));
        }
        _ => {}
    })
}

/// Sets how the widget's child content blends with the widget.
#[property(CHILD_CONTEXT, default(MixBlendMode::default()))]
pub fn child_mix_blend(child: impl IntoUiNode, mode: impl IntoVar<MixBlendMode>) -> UiNode {
    let mode = mode.into_var();
    match_node(child, move |c, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_render(&mode);
        }
        UiNodeOp::Render { frame } => {
            frame.push_filter(mode.get(), &vec![], |frame| c.render(frame));
        }
        _ => {}
    })
}
