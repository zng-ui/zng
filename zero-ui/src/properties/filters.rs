//! Color filter properties, [`opacity`](fn@opacity), [`filter`](fn@filter) and more.

use crate::prelude::new_property::*;

use crate::core::color::filters::{self as cf, Filter};

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
pub fn filter(child: impl UiNode, filter: impl IntoVar<Filter>) -> impl UiNode {
    filter_any(child, filter, false)
}
/// impl any filter, may need layout or not.
fn filter_any(child: impl UiNode, filter: impl IntoVar<Filter>, target_child: bool) -> impl UiNode {
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
                frame.push_filter(MixBlendMode::Normal.into(), render_filter.as_ref().unwrap(), |frame| {
                    child.render(frame)
                });
            } else {
                frame.push_inner_filter(render_filter.clone().unwrap(), |frame| child.render(frame));
            }
        }
        _ => {}
    })
}

/// impl filters that need layout.
fn filter_layout(child: impl UiNode, filter: impl IntoVar<Filter>, target_child: bool) -> impl UiNode {
    let filter = filter.into_var();

    let mut render_filter = None;
    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_layout(&filter);
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
                frame.push_filter(MixBlendMode::Normal.into(), render_filter.as_ref().unwrap(), |frame| {
                    child.render(frame)
                });
            } else {
                frame.push_inner_filter(render_filter.clone().unwrap(), |frame| child.render(frame));
            }
        }
        _ => {}
    })
}

/// impl filters that only need render.
fn filter_render(child: impl UiNode, filter: impl IntoVar<Filter>, target_child: bool) -> impl UiNode {
    let filter = filter.into_var().map(|f| f.try_render().unwrap());
    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_render(&filter);
        }
        UiNodeOp::Render { frame } => {
            if target_child {
                filter.with(|f| {
                    frame.push_filter(MixBlendMode::Normal.into(), f, |frame| child.render(frame));
                });
            } else {
                frame.push_inner_filter(filter.get(), |frame| child.render(frame));
            }
        }
        _ => {}
    })
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
pub fn child_filter(child: impl UiNode, filter: impl IntoVar<Filter>) -> impl UiNode {
    filter_any(child, filter, true)
}

/// Inverts the colors of the widget.
///
/// Zero does not invert, one fully inverts.
///
/// This property is a shorthand way of setting [`filter`] to [`color::filters::invert`] using variable mapping.
///
/// [`filter`]: fn@filter
#[property(CONTEXT, default(false))]
pub fn invert_color(child: impl UiNode, amount: impl IntoVar<Factor>) -> impl UiNode {
    filter_render(child, amount.into_var().map(|&a| cf::invert(a)), false)
}

/// Blur the widget.
///
/// This property is a shorthand way of setting [`filter`] to [`color::filters::blur`] using variable mapping.
///
/// [`filter`]: fn@filter
#[property(CONTEXT, default(0))]
pub fn blur(child: impl UiNode, radius: impl IntoVar<Length>) -> impl UiNode {
    filter_layout(child, radius.into_var().map(|r| cf::blur(r.clone())), false)
}

/// Sepia tone the widget.
///
/// zero is the original colors, one is the full desaturated brown look.
///
/// This property is a shorthand way of setting [`filter`] to [`color::filters::sepia`] using variable mapping.
///
/// [`filter`]: fn@filter
#[property(CONTEXT, default(false))]
pub fn sepia(child: impl UiNode, amount: impl IntoVar<Factor>) -> impl UiNode {
    filter_render(child, amount.into_var().map(|&a| cf::sepia(a)), false)
}

/// Grayscale tone the widget.
///
/// Zero is the original colors, one if the full grayscale.
///
/// This property is a shorthand way of setting [`filter`] to [`color::filters::grayscale`] using variable mapping.
///
/// [`filter`]: fn@filter
#[property(CONTEXT, default(false))]
pub fn grayscale(child: impl UiNode, amount: impl IntoVar<Factor>) -> impl UiNode {
    filter_render(child, amount.into_var().map(|&a| cf::grayscale(a)), false)
}

/// Drop-shadow effect for the widget.
///
/// The shadow is *pixel accurate*.
///
/// This property is a shorthand way of setting [`filter`] to [`color::filters::drop_shadow`] using variable merging.
///
/// [`filter`]: fn@filter
#[property(CONTEXT, default((0, 0), 0, colors::BLACK.transparent()))]
pub fn drop_shadow(
    child: impl UiNode,
    offset: impl IntoVar<Point>,
    blur_radius: impl IntoVar<Length>,
    color: impl IntoVar<Rgba>,
) -> impl UiNode {
    filter_layout(
        child,
        merge_var!(offset.into_var(), blur_radius.into_var(), color.into_var(), |o, r, &c| {
            cf::drop_shadow(o.clone(), r.clone(), c)
        }),
        false,
    )
}

/// Adjust the widget colors brightness.
///
/// Zero removes all brightness, one is the original brightness.
///
/// This property is a shorthand way of setting [`filter`] to [`color::filters::brightness`] using variable mapping.
///
/// [`filter`]: fn@filter
#[property(CONTEXT, default(1.0))]
pub fn brightness(child: impl UiNode, amount: impl IntoVar<Factor>) -> impl UiNode {
    filter_render(child, amount.into_var().map(|&a| cf::brightness(a)), false)
}

/// Adjust the widget colors contrast.
///
/// Zero removes all contrast, one is the original contrast.
///
/// This property is a shorthand way of setting [`filter`] to [`color::filters::brightness`] using variable mapping.
///
/// [`filter`]: fn@filter
#[property(CONTEXT, default(1.0))]
pub fn contrast(child: impl UiNode, amount: impl IntoVar<Factor>) -> impl UiNode {
    filter_render(child, amount.into_var().map(|&a| cf::contrast(a)), false)
}

/// Adjust the widget colors saturation.
///
/// Zero fully desaturates, one is the original saturation.
///
/// This property is a shorthand way of setting [`filter`] to [`color::filters::saturate`] using variable mapping.
///
/// [`filter`]: fn@filter
#[property(CONTEXT, default(1.0))]
pub fn saturate(child: impl UiNode, amount: impl IntoVar<Factor>) -> impl UiNode {
    filter_render(child, amount.into_var().map(|&a| cf::saturate(a)), false)
}

/// Hue shift the widget colors.
///
/// Adds `angle` to the [`hue`] of the widget colors.
///
/// This property is a shorthand way of setting [`filter`] to [`color::filters::hue_rotate`] using variable mapping.
///
/// [`filter`]: fn@filter
/// [`hue`]: Hsla::hue
#[property(CONTEXT, default(0.deg()))]
pub fn hue_rotate(child: impl UiNode, angle: impl IntoVar<AngleDegree>) -> impl UiNode {
    filter_render(child, angle.into_var().map(|&a| cf::hue_rotate(a)), false)
}

/// Custom color filter.
///
/// The color matrix is in the format of SVG color matrix, [0..5] is the first matrix row.
#[property(CONTEXT, default(cf::ColorMatrix::identity()))]
pub fn color_matrix(child: impl UiNode, matrix: impl IntoVar<cf::ColorMatrix>) -> impl UiNode {
    filter_render(child, matrix.into_var().map(|&m| cf::color_matrix(m)), false)
}

/// Opacity/transparency of the widget.
///
/// This property provides the same visual result as setting [`filter`] to [`color::filters::opacity(opacity)`](color::filters::opacity),
/// **but** updating the opacity is faster in this property.
///
/// [`filter`]: fn@filter
#[property(CONTEXT, default(1.0))]
pub fn opacity(child: impl UiNode, alpha: impl IntoVar<Factor>) -> impl UiNode {
    opacity_impl(child, alpha, false)
}
fn opacity_impl(child: impl UiNode, alpha: impl IntoVar<Factor>, target_child: bool) -> impl UiNode {
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

/// Opacity/transparency of the widget's child.
///
/// This property provides the same visual result as setting [`child_filter`] to [`color::filters::opacity(opacity)`](color::filters::opacity),
/// **but** updating the opacity is faster in this property.
///
/// [`child_filter`]: fn@child_filter
#[property(CHILD_CONTEXT, default(1.0))]
pub fn child_opacity(child: impl UiNode, alpha: impl IntoVar<Factor>) -> impl UiNode {
    opacity_impl(child, alpha, true)
}
