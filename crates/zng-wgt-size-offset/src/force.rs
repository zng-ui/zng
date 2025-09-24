use zng_wgt::prelude::*;

use crate::{WIDGET_SIZE, with_fill_metrics};

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
            child.delegated();
            let c = LAYOUT.constraints().with_new_min_x(Px(0)).with_fill_x(false);

            let width = with_fill_metrics(c, |d| width.layout_dft_x(d.width));
            let mut size = LAYOUT.with_constraints(c.with_unbounded_x().with_exact_x(width), || wm.measure_block(child.node()));
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
            child.delegated();
            let c = LAYOUT.constraints().with_new_min_y(Px(0)).with_fill_y(false);

            let height = with_fill_metrics(c, |d| height.layout_dft_y(d.height));
            let mut size = LAYOUT.with_constraints(c.with_unbounded_y().with_exact_y(height), || wm.measure_block(child.node()));
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
