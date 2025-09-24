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

            let x = LAYOUT.with_constraints(PxConstraints2d::new_exact_size(LAYOUT.constraints().fill_size().max(size)), || {
                x.layout_x()
            });
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
