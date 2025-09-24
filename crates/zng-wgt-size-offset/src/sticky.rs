use zng_wgt::prelude::*;

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
                child.delegated();
                let min = WIDGET.bounds().inner_size().width;
                let mut size = LAYOUT.with_constraints(LAYOUT.constraints().with_min_x(min), || wm.measure_block(child.node()));
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
                child.delegated();
                let min = WIDGET.bounds().inner_size().height;
                let mut size = LAYOUT.with_constraints(LAYOUT.constraints().with_min_y(min), || wm.measure_block(child.node()));
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
                child.delegated();
                let min = WIDGET.bounds().inner_size();
                let mut size = LAYOUT.with_constraints(LAYOUT.constraints().with_min_size(min), || wm.measure_block(child.node()));
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
