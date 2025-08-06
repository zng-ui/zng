use zng_wgt::prelude::*;

use crate::SCROLL_SCALE_VAR;

/// Set on an descendant of `Scroll!` to resize the widget instead of scaling it with the scroll zoom.
///
/// This property disables inline layout for the widget.
#[property(SIZE+1, default(false))]
pub fn zoom_size_only(child: impl IntoUiNode, enabled: impl IntoVar<bool>) -> UiNode {
    let enabled = enabled.into_var();
    let mut scale = 1.fct();
    let mut _zoom_sub = VarHandle::dummy();
    match_node(child, move |c, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var(&enabled);
            if enabled.get() {
                _zoom_sub = SCROLL_SCALE_VAR.subscribe(UpdateOp::Layout, WIDGET.id());
            }
        }
        UiNodeOp::Deinit => {
            _zoom_sub = VarHandle::dummy();
        }
        UiNodeOp::Measure { wm, .. } => {
            wm.disable_inline();
        }
        UiNodeOp::Update { .. } => {
            if let Some(e) = enabled.get_new() {
                if e {
                    _zoom_sub = SCROLL_SCALE_VAR.subscribe(UpdateOp::Layout, WIDGET.id());
                } else {
                    _zoom_sub = VarHandle::dummy();
                    scale = 1.fct();
                }
                WIDGET.layout();
            }
        }
        UiNodeOp::Layout { wl, final_size } => {
            if enabled.get() {
                let s = SCROLL_SCALE_VAR.get();
                if s != 1.fct() {
                    // return the unscaled size to not affect the parent layout,
                    // ideally the scaled parent will fit around the resized child.
                    *final_size = c.measure(&mut wl.to_measure(None));
                    let scaled_size = *final_size * s;
                    LAYOUT.with_constraints(PxConstraints2d::new_exact_size(scaled_size), || c.layout(wl));
                    if scale != s {
                        scale = s;
                        WIDGET.render_update();
                    }
                }
            }
        }
        UiNodeOp::Render { frame } => {
            if frame.is_outer() {
                if scale != 1.fct() {
                    if let Some(t) = PxTransform::scale(scale.0, scale.0).inverse() {
                        frame.push_inner_transform(&t, |frame| c.render(frame));
                    }
                }
            } else {
                tracing::error!("zoom_size_only must render outside NestGroup::WIDGET_INNER")
            }
        }
        UiNodeOp::RenderUpdate { update } => {
            if update.is_outer() {
                if scale != 1.fct() {
                    if let Some(t) = PxTransform::scale(scale.0, scale.0).inverse() {
                        update.with_inner_transform(&t, |update| c.render_update(update));
                    }
                }
            } else {
                tracing::error!("zoom_size_only must render outside NestGroup::WIDGET_INNER")
            }
        }
        _ => {}
    })
}
