use zng_wgt::prelude::*;

use crate::{WIDGET_SIZE, with_fill_metrics};

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
            child.delegated();
            let parent_constraints = LAYOUT.constraints();
            let constraints = parent_constraints.with_new_min_x(Px(0));

            let width = with_fill_metrics(constraints, |d| width.layout_dft_x(d.width));
            let width = constraints.x.clamp(width);
            *desired_size = LAYOUT.with_constraints(constraints.with_exact_x(width), || wm.measure_block(child.node()));
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
            child.delegated();
            let parent_constraints = LAYOUT.constraints();
            let constraints = parent_constraints.with_new_min_y(Px(0));

            let height = with_fill_metrics(constraints, |d| height.layout_dft_x(d.height));
            let height = constraints.x.clamp(height);
            *desired_size = LAYOUT.with_constraints(constraints.with_exact_y(height), || wm.measure_block(child.node()));
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
