use zng_wgt::prelude::*;

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
            child.delegated();
            let c = LAYOUT.constraints();
            let min = LAYOUT.with_constraints(c.with_fill_vector(c.is_bounded()), || min_size.layout());
            let size = LAYOUT.with_constraints(c.with_min_size(min), || wm.measure_block(child.node()));
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
            child.delegated();
            let c = LAYOUT.constraints();
            let min = LAYOUT.with_constraints(c.with_fill_vector(c.is_bounded()), || min_width.layout_x());
            let mut size = LAYOUT.with_constraints(c.with_min_x(min), || wm.measure_block(child.node()));
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
            child.delegated();
            let c = LAYOUT.constraints();
            let min = LAYOUT.with_constraints(c.with_fill_vector(c.is_bounded()), || min_height.layout_y());
            let mut size = LAYOUT.with_constraints(c.with_min_y(min), || wm.measure_block(child.node()));
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
