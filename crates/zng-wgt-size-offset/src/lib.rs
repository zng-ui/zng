#![doc(html_favicon_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo-icon.png")]
#![doc(html_logo_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo.png")]
//!
//! Exact size constraints and exact positioning properties.
//!
//! # Crate
//!
#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]
#![warn(unused_extern_crates)]
#![warn(missing_docs)]

use zng_wgt::prelude::*;

/// Widget layout offset.
///
/// Relative values are computed from the constraints maximum bounded size.
///
/// # `x` and `y`
///
/// You can use the [`x`](fn@x) and [`y`](fn@y) properties to only set the position in one dimension.
#[property(LAYOUT, default((0, 0)))]
pub fn offset(child: impl IntoUiNode, offset: impl IntoVar<Vector>) -> UiNode {
    let offset = offset.into_var();
    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_layout(&offset);
        }
        UiNodeOp::Layout { wl, final_size } => {
            let size = child.layout(wl);
            let offset = LAYOUT.with_constraints(PxConstraints2d::new_exact_size(LAYOUT.constraints().fill_size().max(size)), || {
                offset.layout()
            });
            wl.translate(offset);
            *final_size = size;
        }
        _ => {}
    })
}

/// Offset on the ***x*** axis.
///
/// Relative values are computed from the constraints maximum bounded width.
///
/// # `offset`
///
/// You can set both `x` and `y` at the same time using the [`offset`](fn@offset) property.
#[property(LAYOUT, default(0))]
pub fn x(child: impl IntoUiNode, x: impl IntoVar<Length>) -> UiNode {
    let x = x.into_var();
    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_layout(&x);
        }
        UiNodeOp::Layout { wl, final_size } => {
            let size = child.layout(wl);

            let x = with_fill_metrics(LAYOUT.constraints(), |_| x.layout_x());
            wl.translate(PxVector::new(x, Px(0)));
            *final_size = size;
        }
        _ => {}
    })
}

/// Offset on the ***y*** axis.
///
/// Relative values are computed from the constraints maximum bounded height.
///
/// # `offset`
///
/// You can set both `x` and `y` at the same time using the [`offset`](fn@offset) property.
#[property(LAYOUT, default(0))]
pub fn y(child: impl IntoUiNode, y: impl IntoVar<Length>) -> UiNode {
    let y = y.into_var();
    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_layout(&y);
        }
        UiNodeOp::Layout { wl, final_size } => {
            let size = child.layout(wl);
            let y = LAYOUT.with_constraints(PxConstraints2d::new_exact_size(LAYOUT.constraints().fill_size().max(size)), || {
                y.layout_y()
            });
            wl.translate(PxVector::new(Px(0), y));
            *final_size = size;
        }
        _ => {}
    })
}

/// Minimum size of the widget.
///
/// The widget size can be larger then this but not smaller.
/// Relative values are computed from the constraints maximum bounded size.
///
/// This property does not force the minimum constrained size, the `min_size` is only used
/// in a dimension if it is greater then the constrained minimum.
///
/// This property disables inline layout for the widget.
///
/// # `min_width` and `min_height`
///
/// You can use the [`min_width`](fn@min_width) and [`min_height`](fn@min_height) properties to only
/// set the minimum size of one dimension.
#[property(SIZE-2, default((0, 0)))]
pub fn min_size(child: impl IntoUiNode, min_size: impl IntoVar<Size>) -> UiNode {
    let min_size = min_size.into_var();
    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_layout(&min_size);
        }
        UiNodeOp::Measure { wm, desired_size } => {
            let c = LAYOUT.constraints();
            let min = LAYOUT.with_constraints(c.with_fill_vector(c.is_bounded()), || min_size.layout());
            let size = LAYOUT.with_constraints(c.with_min_size(min), || wm.measure_block(child));
            *desired_size = size.max(min);
        }
        UiNodeOp::Layout { wl, final_size } => {
            let c = LAYOUT.constraints();
            let min = LAYOUT.with_constraints(c.with_fill_vector(c.is_bounded()), || min_size.layout());
            let size = LAYOUT.with_constraints(c.with_min_size(min), || child.layout(wl));
            *final_size = size.max(min);
        }
        _ => {}
    })
}

/// Minimum width of the widget.
///
/// The widget width can be larger then this but not smaller.
/// Relative values are computed from the constraints maximum bounded width.
///
/// This property does not force the minimum constrained width, the `min_width` is only used
/// if it is greater then the constrained minimum.
///
/// This property disables inline layout for the widget.
///
/// # `min_size`
///
/// You can set both `min_width` and `min_height` at the same time using the [`min_size`](fn@min_size) property.
#[property(SIZE-2, default(0))]
pub fn min_width(child: impl IntoUiNode, min_width: impl IntoVar<Length>) -> UiNode {
    let min_width = min_width.into_var();
    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_layout(&min_width);
        }
        UiNodeOp::Measure { wm, desired_size } => {
            let c = LAYOUT.constraints();
            let min = LAYOUT.with_constraints(c.with_fill_vector(c.is_bounded()), || min_width.layout_x());
            let mut size = LAYOUT.with_constraints(c.with_min_x(min), || wm.measure_block(child));
            size.width = size.width.max(min);
            *desired_size = size;
        }
        UiNodeOp::Layout { wl, final_size } => {
            let c = LAYOUT.constraints();
            let min = LAYOUT.with_constraints(c.with_fill_vector(c.is_bounded()), || min_width.layout_x());
            let mut size = LAYOUT.with_constraints(c.with_min_x(min), || child.layout(wl));
            size.width = size.width.max(min);
            *final_size = size;
        }
        _ => {}
    })
}

/// Minimum height of the widget.
///
/// The widget height can be larger then this but not smaller.
/// Relative values are computed from the constraints maximum bounded height.
///
/// This property does not force the minimum constrained height, the `min_height` is only used
/// if it is greater then the constrained minimum.
///
/// This property disables inline layout for the widget.
///
/// # `min_size`
///
/// You can set both `min_width` and `min_height` at the same time using the [`min_size`](fn@min_size) property.
#[property(SIZE-2, default(0))]
pub fn min_height(child: impl IntoUiNode, min_height: impl IntoVar<Length>) -> UiNode {
    let min_height = min_height.into_var();
    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_layout(&min_height);
        }
        UiNodeOp::Measure { wm, desired_size } => {
            let c = LAYOUT.constraints();
            let min = LAYOUT.with_constraints(c.with_fill_vector(c.is_bounded()), || min_height.layout_y());
            let mut size = LAYOUT.with_constraints(c.with_min_y(min), || wm.measure_block(child));
            size.height = size.height.max(min);
            *desired_size = size;
        }
        UiNodeOp::Layout { wl, final_size } => {
            let c = LAYOUT.constraints();
            let min = LAYOUT.with_constraints(c.with_fill_vector(c.is_bounded()), || min_height.layout_y());
            let mut size = LAYOUT.with_constraints(c.with_min_y(min), || child.layout(wl));
            size.height = size.height.max(min);
            *final_size = size;
        }
        _ => {}
    })
}

/// Maximum size of the widget.
///
/// The widget size can be smaller then this but not larger. Relative values are computed from the
/// constraints maximum bounded size.
///
/// This property does not force the maximum constrained size, the `max_size` is only used
/// in a dimension if it is less then the constrained maximum, or the maximum was not constrained.
///
/// This property disables inline layout for the widget.
///
/// # `max_width` and `max_height`
///
/// You can use the [`max_width`](fn@max_width) and [`max_height`](fn@max_height) properties to only
/// set the maximum size of one dimension.
#[property(SIZE-1,  default(PxSize::splat(Px::MAX)))]
pub fn max_size(child: impl IntoUiNode, max_size: impl IntoVar<Size>) -> UiNode {
    let max_size = max_size.into_var();
    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_layout(&max_size);
        }
        UiNodeOp::Measure { wm, desired_size } => {
            let parent_constraints = LAYOUT.constraints();
            let max = with_fill_metrics(parent_constraints, |d| max_size.layout_dft(d));
            let size = LAYOUT.with_constraints(parent_constraints.with_max_size(max), || wm.measure_block(child));
            *desired_size = size.min(max);
            *desired_size = Align::TOP_LEFT.measure(*desired_size, parent_constraints);
        }
        UiNodeOp::Layout { wl, final_size } => {
            let parent_constraints = LAYOUT.constraints();
            let max = with_fill_metrics(parent_constraints, |d| max_size.layout_dft(d));
            let size = LAYOUT.with_constraints(parent_constraints.with_max_size(max), || child.layout(wl));
            *final_size = Align::TOP_LEFT.measure(size.min(max), parent_constraints);
        }
        _ => {}
    })
}

/// Maximum width of the widget.
///
/// The widget width can be smaller then this but not larger.
/// Relative values are computed from the constraints maximum bounded width.
///
/// This property does not force the maximum constrained width, the `max_width` is only used
/// if it is less then the constrained maximum, or the maximum was not constrained.
///
/// This property disables inline layout for the widget.
///
/// # `max_size`
///
/// You can set both `max_width` and `max_height` at the same time using the [`max_size`](fn@max_size) property.
#[property(SIZE-1, default(Px::MAX))]
pub fn max_width(child: impl IntoUiNode, max_width: impl IntoVar<Length>) -> UiNode {
    let max_width = max_width.into_var();
    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_layout(&max_width);
        }
        UiNodeOp::Measure { wm, desired_size } => {
            let parent_constraints = LAYOUT.constraints();
            let max = with_fill_metrics(parent_constraints, |d| max_width.layout_dft_x(d.width));

            let mut size = LAYOUT.with_constraints(parent_constraints.with_max_x(max), || wm.measure_block(child));
            size.width = size.width.min(max);
            *desired_size = size;
            desired_size.width = Align::TOP_LEFT.measure_x(desired_size.width, parent_constraints.x);
        }
        UiNodeOp::Layout { wl, final_size } => {
            let parent_constraints = LAYOUT.constraints();
            let max = with_fill_metrics(parent_constraints, |d| max_width.layout_dft_x(d.width));

            let mut size = LAYOUT.with_constraints(LAYOUT.constraints().with_max_x(max), || child.layout(wl));
            size.width = size.width.min(max);
            *final_size = size;
            final_size.width = Align::TOP_LEFT.measure_x(final_size.width, parent_constraints.x);
        }
        _ => {}
    })
}

/// Maximum height of the widget.
///
/// The widget height can be smaller then this but not larger.
/// Relative values are computed from the constraints maximum bounded height.
///
/// This property does not force the maximum constrained height, the `max_height` is only used
/// if it is less then the constrained maximum, or the maximum was not constrained.
///
/// This property disables inline layout for the widget.
///
/// # `max_size`
///
/// You can set both `max_width` and `max_height` at the same time using the [`max_size`](fn@max_size) property.
#[property(SIZE-1, default(Px::MAX))]
pub fn max_height(child: impl IntoUiNode, max_height: impl IntoVar<Length>) -> UiNode {
    let max_height = max_height.into_var();
    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_layout(&max_height);
        }
        UiNodeOp::Measure { wm, desired_size } => {
            let parent_constraints = LAYOUT.constraints();
            let max = with_fill_metrics(parent_constraints, |d| max_height.layout_dft_y(d.height));

            let mut size = LAYOUT.with_constraints(LAYOUT.constraints().with_max_y(max), || wm.measure_block(child));
            size.height = size.height.min(max);
            *desired_size = size;
            desired_size.height = Align::TOP_LEFT.measure_y(desired_size.height, parent_constraints.y);
        }
        UiNodeOp::Layout { wl, final_size } => {
            let parent_constraints = LAYOUT.constraints();
            let max = with_fill_metrics(parent_constraints, |d| max_height.layout_dft_y(d.height));

            let mut size = LAYOUT.with_constraints(LAYOUT.constraints().with_max_y(max), || child.layout(wl));
            size.height = size.height.min(max);
            *final_size = size;
            final_size.height = Align::TOP_LEFT.measure_y(final_size.height, parent_constraints.y);
        }
        _ => {}
    })
}

/// Exact size of the widget.
///
/// When set the widget is layout with exact size constraints, clamped by the contextual max.
/// Relative size values are computed from the constraints maximum bounded size.
///
/// This property disables inline layout for the widget. This property sets the [`WIDGET_SIZE`].
///
/// See also [`force_size`] to deliberately break layout and cause out-of-bounds rendering when
/// the exact size cannot fit in the contextual min/max.
///
/// # `width` and `height`
///
/// You can use the [`width`] and [`height`] properties to only set the size of one dimension.
///
/// [`min_size`]: fn@min_size
/// [`max_size`]: fn@max_size
/// [`width`]: fn@width
/// [`height`]: fn@height
/// [`force_size`]: fn@force_size
/// [`align`]: fn@zng_wgt::align
#[property(SIZE, default(Size::default()))]
pub fn size(child: impl IntoUiNode, size: impl IntoVar<Size>) -> UiNode {
    let size = size.into_var();
    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var(&size);
            child.init();
            size.with(|l| WIDGET_SIZE.set(l));
        }
        UiNodeOp::Update { updates } => {
            child.update(updates);
            size.with_new(|s| {
                WIDGET_SIZE.set(s);
                WIDGET.layout();
            });
        }
        UiNodeOp::Measure { wm, desired_size } => {
            child.delegated();
            wm.measure_block(&mut UiNode::nil()); // no need to actually measure child

            let parent_constraints = LAYOUT.constraints();

            *desired_size = with_fill_metrics(parent_constraints.with_new_min(Px(0), Px(0)), |d| size.layout_dft(d));
            *desired_size = Align::TOP_LEFT.measure(*desired_size, parent_constraints);
        }
        UiNodeOp::Layout { wl, final_size } => {
            let parent_constraints = LAYOUT.constraints();
            let constraints = parent_constraints.with_new_min(Px(0), Px(0));

            let size = with_fill_metrics(constraints, |d| size.layout_dft(d));
            let size = constraints.clamp_size(size);
            LAYOUT.with_constraints(PxConstraints2d::new_exact_size(size), || child.layout(wl));

            *final_size = Align::TOP_LEFT.measure(size, parent_constraints);
        }
        _ => {}
    })
}

/// Exact width of the widget.
///
/// When set the widget is layout with exact size constraints, clamped by the contextual max.
/// Relative values are computed from the constraints maximum bounded width.
///
/// This property disables inline layout for the widget. This property sets the [`WIDGET_SIZE`] width.
///
/// See also [`force_width`] to deliberately break layout and cause out-of-bounds rendering when
/// the exact width cannot fit in the contextual min/max.
///
/// # `size`
///
/// You can set both `width` and `height` at the same time using the [`size`](fn@size) property.
///
/// [`min_width`]: fn@min_width
/// [`max_width`]: fn@max_width
/// [`force_width`]: fn@force_width
#[property(SIZE, default(Length::Default))]
pub fn width(child: impl IntoUiNode, width: impl IntoVar<Length>) -> UiNode {
    let width = width.into_var();
    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var(&width);
            child.init();
            width.with(|s| WIDGET_SIZE.set_width(s));
        }
        UiNodeOp::Update { updates } => {
            child.update(updates);
            width.with_new(|w| {
                WIDGET_SIZE.set_width(w);
                WIDGET.layout();
            });
        }
        UiNodeOp::Measure { wm, desired_size } => {
            let parent_constraints = LAYOUT.constraints();
            let constraints = parent_constraints.with_new_min_x(Px(0));

            let width = with_fill_metrics(constraints, |d| width.layout_dft_x(d.width));
            let width = constraints.x.clamp(width);
            *desired_size = LAYOUT.with_constraints(constraints.with_exact_x(width), || wm.measure_block(child));
            desired_size.width = Align::TOP_LEFT.measure_x(width, parent_constraints.x);
        }
        UiNodeOp::Layout { wl, final_size } => {
            let parent_constraints = LAYOUT.constraints();
            let constraints = parent_constraints.with_new_min_x(Px(0));

            let width = with_fill_metrics(constraints, |d| width.layout_dft_x(d.width));
            let width = constraints.x.clamp(width);
            *final_size = LAYOUT.with_constraints(constraints.with_exact_x(width), || child.layout(wl));
            final_size.width = Align::TOP_LEFT.measure_x(width, parent_constraints.x);
        }
        _ => {}
    })
}

/// Exact height of the widget.
///
/// When set the widget is layout with exact size constraints, clamped by the contextual min/max.
/// Relative values are computed from the constraints maximum bounded height.
///
/// This property disables inline layout for the widget. This property sets the [`WIDGET_SIZE`] height.
///
/// See also [`force_height`] to deliberately break layout and cause out-of-bounds rendering when
/// the exact height cannot fit in the contextual min/max.
///
/// # `size`
///
/// You can set both `width` and `height` at the same time using the [`size`](fn@size) property.
///
/// [`min_height`]: fn@min_height
/// [`max_height`]: fn@max_height
/// [`force_height`]: fn@force_height
#[property(SIZE, default(Length::Default))]
pub fn height(child: impl IntoUiNode, height: impl IntoVar<Length>) -> UiNode {
    let height = height.into_var();
    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var(&height);
            child.init();
            height.with(|s| WIDGET_SIZE.set_height(s));
        }
        UiNodeOp::Update { updates } => {
            child.update(updates);

            height.with_new(|h| {
                WIDGET_SIZE.set_height(h);
                WIDGET.layout();
            });
        }
        UiNodeOp::Measure { wm, desired_size } => {
            let parent_constraints = LAYOUT.constraints();
            let constraints = parent_constraints.with_new_min_y(Px(0));

            let height = with_fill_metrics(constraints, |d| height.layout_dft_x(d.height));
            let height = constraints.x.clamp(height);
            *desired_size = LAYOUT.with_constraints(constraints.with_exact_y(height), || wm.measure_block(child));
            desired_size.height = Align::TOP_LEFT.measure_y(height, parent_constraints.y);
        }
        UiNodeOp::Layout { wl, final_size } => {
            let parent_constraints = LAYOUT.constraints();
            let constraints = parent_constraints.with_new_min_y(Px(0));

            let height = with_fill_metrics(constraints, |d| height.layout_dft_y(d.height));
            let height = constraints.y.clamp(height);
            *final_size = LAYOUT.with_constraints(constraints.with_exact_y(height), || child.layout(wl));
            final_size.height = Align::TOP_LEFT.measure_y(height, parent_constraints.y);
        }
        _ => {}
    })
}

/// Exact size of the widget ignoring the contextual max.
///
/// When set the widget is layout with exact size constraints, ignoring the contextual max.
/// Relative values are computed from the constraints maximum bounded size.
///
/// Note that this property deliberately breaks layout and causes out-of-bounds rendering. You
/// can use [`size`](fn@size) instead to set an exact size that is coerced by the contextual max.
///
/// # `force_width` and `force_height`
///
/// You can use the [`force_width`] and [`force_height`] properties to only set the size of one dimension.
///
/// [`force_width`]: fn@force_width
/// [`force_height`]: fn@force_height
#[property(SIZE, default(Size::default()))]
pub fn force_size(child: impl IntoUiNode, size: impl IntoVar<Size>) -> UiNode {
    let size = size.into_var();
    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var(&size);
            child.init();
            size.with(|l| WIDGET_SIZE.set(l));
        }
        UiNodeOp::Update { updates } => {
            child.update(updates);
            size.with_new(|s| {
                WIDGET_SIZE.set(s);
                WIDGET.layout();
            });
        }
        UiNodeOp::Measure { wm, desired_size } => {
            child.delegated();
            let c = LAYOUT.constraints().with_new_min(Px(0), Px(0)).with_fill(false, false);
            let size = with_fill_metrics(c, |d| size.layout_dft(d));
            wm.measure_block(&mut UiNode::nil());
            *desired_size = Align::TOP_LEFT.measure(size, c);
        }
        UiNodeOp::Layout { wl, final_size } => {
            let c = LAYOUT.constraints().with_new_min(Px(0), Px(0)).with_fill(false, false);
            let size = with_fill_metrics(c, |d| size.layout_dft(d));
            LAYOUT.with_constraints(PxConstraints2d::new_exact_size(size), || child.layout(wl));
            *final_size = Align::TOP_LEFT.measure(size, c);
        }
        _ => {}
    })
}

/// Exact width of the widget ignoring the contextual max.
///
/// When set the widget is layout with exact size constraints, ignoring the contextual max.
/// Relative values are computed from the constraints maximum bounded width.
///
/// Note that this property deliberately breaks layout and causes out-of-bounds rendering. You
/// can use [`width`](fn@width) instead to set an exact width that is coerced by the contextual max.
///
/// # `force_size`
///
/// You can set both `force_width` and `force_height` at the same time using the [`force_size`](fn@force_size) property.
#[property(SIZE, default(Length::Default))]
pub fn force_width(child: impl IntoUiNode, width: impl IntoVar<Length>) -> UiNode {
    let width = width.into_var();
    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var(&width);
            child.init();
            width.with(|s| WIDGET_SIZE.set_width(s));
        }
        UiNodeOp::Update { updates } => {
            child.update(updates);
            width.with_new(|w| {
                WIDGET_SIZE.set_width(w);
                WIDGET.layout();
            });
        }
        UiNodeOp::Measure { wm, desired_size } => {
            let c = LAYOUT.constraints().with_new_min_x(Px(0)).with_fill_x(false);

            let width = with_fill_metrics(c, |d| width.layout_dft_x(d.width));
            let mut size = LAYOUT.with_constraints(c.with_unbounded_x().with_exact_x(width), || wm.measure_block(child));
            size.width = Align::TOP_LEFT.measure_x(width, c.x);
            *desired_size = size;
        }
        UiNodeOp::Layout { wl, final_size } => {
            let c = LAYOUT.constraints().with_new_min_x(Px(0)).with_fill_x(false);

            let width = with_fill_metrics(c, |d| width.layout_dft_x(d.width));
            let mut size = LAYOUT.with_constraints(c.with_unbounded_x().with_exact_x(width), || child.layout(wl));
            size.width = Align::TOP_LEFT.measure_x(width, c.x);
            *final_size = size;
        }
        _ => {}
    })
}

/// Exact height of the widget ignoring the contextual max.
///
/// When set the widget is layout with exact size constraints, ignoring the contextual max.
/// Relative values are computed from the constraints maximum bounded height.
///
/// Note that this property deliberately breaks layout and causes out-of-bounds rendering. You
/// can use [`height`](fn@height) instead to set an exact height that is coerced by the contextual max.
///
/// # `force_size`
///
/// You can set both `force_width` and `force_height` at the same time using the [`force_size`](fn@force_size) property.
#[property(SIZE, default(Length::Default))]
pub fn force_height(child: impl IntoUiNode, height: impl IntoVar<Length>) -> UiNode {
    let height = height.into_var();
    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var(&height);
            child.init();
            height.with(|s| WIDGET_SIZE.set_height(s));
        }
        UiNodeOp::Update { updates } => {
            child.update(updates);

            height.with_new(|h| {
                WIDGET_SIZE.set_height(h);
                WIDGET.layout();
            });
        }
        UiNodeOp::Measure { wm, desired_size } => {
            let c = LAYOUT.constraints().with_new_min_y(Px(0)).with_fill_y(false);

            let height = with_fill_metrics(c, |d| height.layout_dft_y(d.height));
            let mut size = LAYOUT.with_constraints(c.with_unbounded_y().with_exact_y(height), || wm.measure_block(child));
            size.height = Align::TOP_LEFT.measure_y(height, c.y);
            *desired_size = size;
        }
        UiNodeOp::Layout { wl, final_size } => {
            let c = LAYOUT.constraints().with_new_min_y(Px(0)).with_fill_y(false);

            let height = with_fill_metrics(c, |d| height.layout_dft_y(d.height));
            let mut size = LAYOUT.with_constraints(c.with_unbounded_y().with_exact_y(height), || child.layout(wl));
            size.height = Align::TOP_LEFT.measure_y(height, c.y);
            *final_size = size;
        }
        _ => {}
    })
}

fn with_fill_metrics<R>(c: PxConstraints2d, f: impl FnOnce(PxSize) -> R) -> R {
    let dft = c.fill_size();
    LAYOUT.with_constraints(c.with_fill_vector(c.is_bounded()), || f(dft))
}

/// Set or overwrite the baseline of the widget.
///
/// The `baseline` is a vertical offset from the bottom edge of the widget's inner bounds up, it defines the
/// line where the widget naturally *sits*, some widgets like [Text!` have a non-zero default baseline, most others leave it at zero.
///
/// Relative values are computed from the widget's height.
#[property(BORDER, default(Length::Default))]
pub fn baseline(child: impl IntoUiNode, baseline: impl IntoVar<Length>) -> UiNode {
    let baseline = baseline.into_var();
    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_layout(&baseline);
        }
        UiNodeOp::Layout { wl, final_size } => {
            let size = child.layout(wl);

            let bounds = WIDGET.bounds();
            let inner_size = bounds.inner_size();
            let default = bounds.baseline();

            let baseline = LAYOUT.with_constraints(LAYOUT.constraints().with_max_size(inner_size).with_fill(true, true), || {
                baseline.layout_dft_y(default)
            });
            wl.set_baseline(baseline);

            *final_size = size;
        }
        _ => {}
    })
}

/// Retain the widget's previous width if the new layout width is smaller.
/// The widget is layout using its previous width as the minimum width constrain.
///
/// This property disables inline layout for the widget.
#[property(SIZE, default(false))]
pub fn sticky_width(child: impl IntoUiNode, sticky: impl IntoVar<bool>) -> UiNode {
    let sticky = sticky.into_var();
    let mut sticky_after_layout = false;
    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var(&sticky);
        }
        UiNodeOp::Deinit => {
            sticky_after_layout = false;
        }
        UiNodeOp::Update { .. } => {
            if sticky.is_new() {
                sticky_after_layout = false;
            }
        }
        UiNodeOp::Measure { wm, desired_size } => {
            if sticky_after_layout && sticky.get() {
                let min = WIDGET.bounds().inner_size().width;
                let mut size = LAYOUT.with_constraints(LAYOUT.constraints().with_min_x(min), || wm.measure_block(child));
                size.width = size.width.max(min);
                *desired_size = size;
            }
        }
        UiNodeOp::Layout { wl, final_size } => {
            let sticky = sticky.get();
            if sticky_after_layout && sticky {
                let min = WIDGET.bounds().inner_size().width;
                let mut size = LAYOUT.with_constraints(LAYOUT.constraints().with_min_x(min), || child.layout(wl));
                size.width = size.width.max(min);
                *final_size = size;
            }

            // only enable after the `WIDGET.bounds().inner_size()` updates
            sticky_after_layout = sticky;
        }
        _ => {}
    })
}

/// Retain the widget's previous height if the new layout height is smaller.
/// The widget is layout using its previous height as the minimum height constrain.
///
/// This property disables inline layout for the widget.
#[property(SIZE, default(false))]
pub fn sticky_height(child: impl IntoUiNode, sticky: impl IntoVar<bool>) -> UiNode {
    let sticky = sticky.into_var();
    let mut sticky_after_layout = false;
    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var(&sticky);
        }
        UiNodeOp::Deinit => {
            sticky_after_layout = false;
        }
        UiNodeOp::Update { .. } => {
            if sticky.is_new() {
                sticky_after_layout = false;
            }
        }
        UiNodeOp::Measure { wm, desired_size } => {
            if sticky_after_layout && sticky.get() {
                let min = WIDGET.bounds().inner_size().height;
                let mut size = LAYOUT.with_constraints(LAYOUT.constraints().with_min_y(min), || wm.measure_block(child));
                size.height = size.height.max(min);
                *desired_size = size;
            }
        }
        UiNodeOp::Layout { wl, final_size } => {
            let sticky = sticky.get();
            if sticky_after_layout && sticky {
                let min = WIDGET.bounds().inner_size().height;
                let mut size = LAYOUT.with_constraints(LAYOUT.constraints().with_min_y(min), || child.layout(wl));
                size.height = size.height.max(min);
                *final_size = size;
            }

            // only enable after the `WIDGET.bounds().inner_size()` updates
            sticky_after_layout = sticky;
        }
        _ => {}
    })
}

/// Retain the widget's previous size if the new layout size is smaller.
/// The widget is layout using its previous size as the minimum size constrain.
///
/// This property disables inline layout for the widget.
#[property(SIZE, default(false))]
pub fn sticky_size(child: impl IntoUiNode, sticky: impl IntoVar<bool>) -> UiNode {
    let sticky = sticky.into_var();
    let mut sticky_after_layout = false;
    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var(&sticky);
        }
        UiNodeOp::Deinit => {
            sticky_after_layout = false;
        }
        UiNodeOp::Update { .. } => {
            if sticky.is_new() {
                sticky_after_layout = false;
            }
        }
        UiNodeOp::Measure { wm, desired_size } => {
            if sticky_after_layout && sticky.get() {
                let min = WIDGET.bounds().inner_size();
                let mut size = LAYOUT.with_constraints(LAYOUT.constraints().with_min_size(min), || wm.measure_block(child));
                size = size.max(min);
                *desired_size = size;
            }
        }
        UiNodeOp::Layout { wl, final_size } => {
            let sticky = sticky.get();
            if sticky_after_layout && sticky {
                let min = WIDGET.bounds().inner_size();
                let mut size = LAYOUT.with_constraints(LAYOUT.constraints().with_min_size(min), || child.layout(wl));
                size = size.max(min);
                *final_size = size;
            }

            // only enable after the `WIDGET.bounds().inner_size()` updates
            sticky_after_layout = sticky;
        }
        _ => {}
    })
}

/// Exact size property info.
///
/// Properties like [`size`], [`width`] and [`height`] set this metadata in the widget state.
/// Panels can use this info to implement [`Length::Leftover`] support.
///
/// [`size`]: fn@size
/// [`width`]: fn@width
/// [`height`]: fn@height
/// [`Length::Leftover`]: zng_wgt::prelude::Length::Leftover
#[expect(non_camel_case_types)]
pub struct WIDGET_SIZE;
impl WIDGET_SIZE {
    /// Set the width state.
    pub fn set_width(&self, width: &Length) {
        WIDGET.with_state_mut(|mut state| {
            let width = width.into();
            match state.entry(*WIDGET_SIZE_ID) {
                state_map::StateMapEntry::Occupied(mut e) => e.get_mut().width = width,
                state_map::StateMapEntry::Vacant(e) => {
                    e.insert(euclid::size2(width, WidgetLength::Default));
                }
            }
        });
    }

    /// Set the height state.
    pub fn set_height(&self, height: &Length) {
        WIDGET.with_state_mut(|mut state| {
            let height = height.into();
            match state.entry(*WIDGET_SIZE_ID) {
                state_map::StateMapEntry::Occupied(mut e) => e.get_mut().height = height,
                state_map::StateMapEntry::Vacant(e) => {
                    e.insert(euclid::size2(WidgetLength::Default, height));
                }
            }
        })
    }

    /// Set the size state.
    pub fn set(&self, size: &Size) {
        WIDGET.set_state(*WIDGET_SIZE_ID, euclid::size2((&size.width).into(), (&size.height).into()));
    }

    /// Get the size set in the state.
    pub fn get(&self) -> euclid::Size2D<WidgetLength, ()> {
        WIDGET.get_state(*WIDGET_SIZE_ID).unwrap_or_default()
    }

    /// Get the size set in the widget state.
    pub fn get_wgt(&self, wgt: &mut UiNode) -> euclid::Size2D<WidgetLength, ()> {
        match wgt.as_widget() {
            Some(mut wgt) => wgt.with_context(WidgetUpdateMode::Ignore, || self.get()),
            None => Default::default()
        }
    }
}

static_id! {
    static ref WIDGET_SIZE_ID: StateId<euclid::Size2D<WidgetLength, ()>>;
}

/// Getter property, gets the latest rendered widget inner size.
#[property(LAYOUT)]
pub fn actual_size(child: impl IntoUiNode, size: impl IntoVar<DipSize>) -> UiNode {
    let size = size.into_var();
    match_node(child, move |c, op| match op {
        UiNodeOp::Render { frame } => {
            c.render(frame);

            let f = frame.scale_factor();
            let s = WIDGET.info().bounds_info().inner_size().to_dip(f);
            if size.get() != s {
                size.set(s);
            }
        }
        UiNodeOp::RenderUpdate { update } => {
            c.render_update(update);

            let info = WIDGET.info();
            let f = info.tree().scale_factor();
            let s = info.bounds_info().inner_size().to_dip(f);
            if size.get() != s {
                size.set(s);
            }
        }
        _ => {}
    })
}

/// Getter property, gets the latest rendered widget inner width.
#[property(LAYOUT)]
pub fn actual_width(child: impl IntoUiNode, width: impl IntoVar<Dip>) -> UiNode {
    let width = width.into_var();
    match_node(child, move |c, op| match op {
        UiNodeOp::Render { frame } => {
            c.render(frame);

            let f = frame.scale_factor();
            let w = WIDGET.info().bounds_info().inner_size().width.to_dip(f);
            if width.get() != w {
                width.set(w);
            }
        }
        UiNodeOp::RenderUpdate { update } => {
            c.render_update(update);

            let info = WIDGET.info();
            let f = info.tree().scale_factor();
            let w = info.bounds_info().inner_size().width.to_dip(f);
            if width.get() != w {
                width.set(w);
            }
        }
        _ => {}
    })
}

/// Getter property, gets the latest rendered widget inner height.
#[property(LAYOUT)]
pub fn actual_height(child: impl IntoUiNode, height: impl IntoVar<Dip>) -> UiNode {
    let height = height.into_var();
    match_node(child, move |c, op| match op {
        UiNodeOp::Render { frame } => {
            c.render(frame);

            let f = frame.scale_factor();
            let h = WIDGET.info().bounds_info().inner_size().height.to_dip(f);
            if height.get() != h {
                height.set(h);
            }
        }
        UiNodeOp::RenderUpdate { update } => {
            c.render_update(update);

            let info = WIDGET.info();
            let f = info.tree().scale_factor();
            let h = info.bounds_info().inner_size().height.to_dip(f);
            if height.get() != h {
                height.set(h);
            }
        }
        _ => {}
    })
}

/// Getter property, gets the latest rendered widget inner size, in device pixels.
#[property(LAYOUT)]
pub fn actual_size_px(child: impl IntoUiNode, size: impl IntoVar<PxSize>) -> UiNode {
    let size = size.into_var();
    match_node(child, move |c, op| match &op {
        UiNodeOp::Render { .. } | UiNodeOp::RenderUpdate { .. } => {
            c.op(op);
            let s = WIDGET.info().bounds_info().inner_size();
            if size.get() != s {
                // avoid pushing var changes every frame.
                size.set(s);
            }
        }
        _ => {}
    })
}

/// Getter property, gets the latest rendered widget inner width, in device pixels.
#[property(LAYOUT)]
pub fn actual_width_px(child: impl IntoUiNode, width: impl IntoVar<Px>) -> UiNode {
    let width = width.into_var();
    match_node(child, move |c, op| match &op {
        UiNodeOp::Render { .. } | UiNodeOp::RenderUpdate { .. } => {
            c.op(op);
            let w = WIDGET.info().bounds_info().inner_size().width;
            if width.get() != w {
                width.set(w);
            }
        }
        _ => {}
    })
}

/// Getter property, gets the latest rendered widget inner height, in device pixels.
#[property(LAYOUT)]
pub fn actual_height_px(child: impl IntoUiNode, height: impl IntoVar<Px>) -> UiNode {
    let height = height.into_var();
    match_node(child, move |c, op| match &op {
        UiNodeOp::Render { .. } | UiNodeOp::RenderUpdate { .. } => {
            c.op(op);
            let h = WIDGET.info().bounds_info().inner_size().height;
            if height.get() != h {
                height.set(h);
            }
        }
        _ => {}
    })
}

/// Getter property, gets the latest rendered widget inner transform.
#[property(LAYOUT)]
pub fn actual_transform(child: impl IntoUiNode, transform: impl IntoVar<PxTransform>) -> UiNode {
    let transform = transform.into_var();
    match_node(child, move |c, op| match &op {
        UiNodeOp::Render { .. } | UiNodeOp::RenderUpdate { .. } => {
            c.op(op);
            let t = WIDGET.info().bounds_info().inner_transform();
            if transform.get() != t {
                transform.set(t);
            }
        }
        _ => {}
    })
}

/// Getter property, gets the latest rendered widget inner bounds in the window space.
#[property(LAYOUT)]
pub fn actual_bounds(child: impl IntoUiNode, bounds: impl IntoVar<PxRect>) -> UiNode {
    let bounds = bounds.into_var();
    match_node(child, move |c, op| match &op {
        UiNodeOp::Render { .. } | UiNodeOp::RenderUpdate { .. } => {
            c.op(op);
            let t = WIDGET.info().bounds_info().inner_bounds();
            if bounds.get() != t {
                bounds.set(t);
            }
        }
        _ => {}
    })
}

/// Represents the width or height property value set on a widget.
///
/// Properties like [`size`], [`width`] and [`height`] set the [`WIDGET_SIZE`]
/// metadata in the widget state. Panels can use this info to implement [`Length::Leftover`] support.
///  
/// [`size`]: fn@size
/// [`width`]: fn@width
/// [`height`]: fn@height
/// [`Length::Leftover`]: zng_wgt::prelude::Length::Leftover
#[derive(Debug, Clone, Copy, PartialEq, Default, serde::Serialize, serde::Deserialize)]
pub enum WidgetLength {
    /// Evaluates to [`PxConstraints2d::fill_size`] when measured, can serve as a request for *size-to-fit*.
    ///
    /// The `Grid!` widget uses this to fit the column and row widgets to *their* cells, as they don't
    /// logically own the cells, this fit needs to be computed by the parent panel.
    ///
    /// [`PxConstraints2d::fill_size`]: zng_wgt::prelude::PxConstraints2d::fill_size
    #[default]
    Default,
    /// The [`Length::Leftover`] value. Evaluates to the [`LayoutMetrics::leftover`] value when measured, if
    /// a leftover value is not provided evaluates like a [`Length::Factor`].
    ///
    /// The *leftover* length needs to be computed by the parent panel, as it depends on the length of the sibling widgets,
    /// not just the panel constraints. Panels that support this, compute the value for each widget and measure/layout each using
    /// [`LAYOUT.with_leftover`] to inject the computed value.
    ///
    /// [`LAYOUT.with_leftover`]: zng_wgt::prelude::LAYOUT::with_leftover
    /// [`Length::Leftover`]: zng_wgt::prelude::Length::Leftover
    /// [`Length::Factor`]: zng_wgt::prelude::Length::Factor
    /// [`LayoutMetrics::leftover`]: zng_wgt::prelude::LayoutMetrics::leftover
    Leftover(Factor),
    /// Any of the other [`Length`] kinds. All contextual metrics needed to compute these values is already available
    /// in the [`LayoutMetrics`], panels that support [`Length::Leftover`] can layout this widget first to compute the
    /// leftover length.
    ///
    /// [`Length::Leftover`]: zng_wgt::prelude::Length::Leftover
    /// [`LayoutMetrics`]: zng_wgt::prelude::LayoutMetrics
    /// [`Length`]: zng_wgt::prelude::Length
    Exact,
}

impl From<&Length> for WidgetLength {
    fn from(value: &Length) -> Self {
        match value {
            Length::Default => WidgetLength::Default,
            Length::Leftover(f) => WidgetLength::Leftover(*f),
            _ => WidgetLength::Exact,
        }
    }
}
