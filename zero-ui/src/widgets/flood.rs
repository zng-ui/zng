use crate::prelude::new_widget::*;

/// Node that fills the widget area with a color.
///
/// Note that this node is not a full widget, it can be used as part of an widget without adding to the info tree.
pub fn flood(color: impl IntoVar<Rgba>) -> impl UiNode {
    let color = color.into_var();
    let mut render_size = PxSize::zero();
    let frame_key = FrameValueKey::new_unique();

    match_node_leaf(move |op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var(&color);
        }
        UiNodeOp::Update { .. } => {
            if color.is_new() {
                WIDGET.render_update();
            }
        }
        UiNodeOp::Measure { desired_size, .. } => {
            *desired_size = LAYOUT.constraints().fill_size();
        }
        UiNodeOp::Layout { final_size, .. } => {
            *final_size = LAYOUT.constraints().fill_size();
            if *final_size != render_size {
                render_size = *final_size;
                WIDGET.render();
            }
        }
        UiNodeOp::Render { frame } => {
            frame.push_color(PxRect::from_size(render_size), frame_key.bind_var(&color, |&c| c.into()));
        }
        UiNodeOp::RenderUpdate { update } => {
            update.update_color_opt(frame_key.update_var(&color, |&c| c.into()));
        }
        _ => {}
    })
}
