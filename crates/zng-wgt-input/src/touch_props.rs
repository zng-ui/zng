use zng_ext_input::touch::{TOUCH_TRANSFORM_EVENT, TouchTransformMode};
use zng_view_api::touch::TouchPhase;
use zng_wgt::prelude::*;

/// Applies transforms from touch gestures on the widget.
#[property(LAYOUT, default(false))]
pub fn touch_transform(child: impl UiNode, mode: impl IntoVar<TouchTransformMode>) -> impl UiNode {
    let mode = mode.into_var();
    let mut handle = EventHandle::dummy();
    let mut transform_committed = PxTransform::identity();
    let mut transform = PxTransform::identity();
    match_node(child, move |c, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var(&mode);
            if !mode.get().is_empty() {
                handle = TOUCH_TRANSFORM_EVENT.subscribe(WIDGET.id());
            }
        }
        UiNodeOp::Deinit => {
            handle = EventHandle::dummy();
        }
        UiNodeOp::Event { update } => {
            if let Some(args) = TOUCH_TRANSFORM_EVENT.on(update) {
                if args.propagation().is_stopped() {
                    return;
                }

                let t = transform_committed.then(&args.local_transform(mode.get()));
                if transform != t {
                    transform = t;
                    WIDGET.render_update();
                }

                match args.phase {
                    TouchPhase::Start | TouchPhase::Move => {}
                    TouchPhase::End => {
                        transform_committed = transform;
                    }
                    TouchPhase::Cancel => {
                        transform = transform_committed;
                        WIDGET.render_update();
                    }
                }
            }
        }
        UiNodeOp::Update { .. } => {
            if let Some(mode) = mode.get_new() {
                if handle.is_dummy() {
                    if !mode.is_empty() {
                        handle = TOUCH_TRANSFORM_EVENT.subscribe(WIDGET.id());
                    }
                } else if mode.is_empty() {
                    handle = EventHandle::dummy();
                }
            }
        }
        UiNodeOp::Render { frame } => {
            frame.push_inner_transform(&transform, |f| c.render(f));
        }
        UiNodeOp::RenderUpdate { update } => {
            update.with_inner_transform(&transform, |u| c.render_update(u));
        }
        _ => {}
    })
}
