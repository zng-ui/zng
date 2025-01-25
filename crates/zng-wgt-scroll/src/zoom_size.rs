use zng_wgt::prelude::*;

use crate::SCROLL_SCALE_VAR;

/// Set on an descendant of `Scroll!` to resize the widget instead of scaling it with the scroll zoom.
///
/// This property disables inline layout for the widget.
#[property(SIZE, default(false))]
pub fn zoom_size_only(child: impl UiNode, enabled: impl IntoVar<bool>) -> impl UiNode {
    let enabled = enabled.into_var();
    match_node(child, move |c, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_layout(&enabled).sub_var_layout(&SCROLL_SCALE_VAR);
        }
        UiNodeOp::Measure { wm, .. } => {
            wm.disable_inline();
        }
        UiNodeOp::Layout { wl, final_size } => {
            let scale = SCROLL_SCALE_VAR.get();
            if scale != 1.fct() {
                // return the unscaled size to not affect the parent layout,
                // ideally the scaled parent will fit around the resized child.
                *final_size = c.measure(&mut wl.to_measure(None));
                let scaled_size = *final_size * scale;
                LAYOUT.with_constraints(PxConstraints2d::new_exact_size(scaled_size), || c.layout(wl));
            }
        }
        UiNodeOp::Render { frame } => {
            if frame.is_outer() {
                let scale = SCROLL_SCALE_VAR.get();
                if scale != 1.fct() {
                    if let Some(t) = PxTransform::scale(scale.0, scale.0).inverse() {
                        frame.push_inner_transform(&t, |frame| c.render(frame));
                    }
                }
            } else {
                tracing::error!("zoom_size_only must render at NestGroup::SIZE")
            }
        }
        _ => {}
    })
}
