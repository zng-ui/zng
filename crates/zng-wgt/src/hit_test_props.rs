use zng_app::widget::base::HIT_TEST_MODE_VAR;
pub use zng_app::widget::base::HitTestMode;

use crate::prelude::*;

/// Defines if and how the widget is hit-tested.
///
/// See [`HitTestMode`] for more details.
#[property(CONTEXT, default(HIT_TEST_MODE_VAR))]
pub fn hit_test_mode(child: impl UiNode, mode: impl IntoVar<HitTestMode>) -> impl UiNode {
    let child = match_node(child, |child, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_render(&HIT_TEST_MODE_VAR);
        }
        UiNodeOp::Render { frame } => match HIT_TEST_MODE_VAR.get() {
            HitTestMode::Disabled => {
                frame.with_hit_tests_disabled(|frame| child.render(frame));
            }
            HitTestMode::Detailed => frame.with_auto_hit_test(true, |frame| child.render(frame)),
            _ => frame.with_auto_hit_test(false, |frame| child.render(frame)),
        },
        UiNodeOp::RenderUpdate { update } => {
            update.with_auto_hit_test(matches!(HIT_TEST_MODE_VAR.get(), HitTestMode::Detailed), |update| {
                child.render_update(update)
            });
        }
        _ => {}
    });

    with_context_var(
        child,
        HIT_TEST_MODE_VAR,
        merge_var!(HIT_TEST_MODE_VAR, mode.into_var(), |&a, &b| match (a, b) {
            (HitTestMode::Disabled, _) | (_, HitTestMode::Disabled) => HitTestMode::Disabled,
            (_, b) => b,
        }),
    )
}

/// If the widget is visible for hit-tests.
///
/// This property is used only for probing the state. You can set the state using
/// the [`hit_test_mode`] property.
///
/// [`hit_test_mode`]: fn@hit_test_mode
#[property(EVENT)]
pub fn is_hit_testable(child: impl UiNode, state: impl IntoVar<bool>) -> impl UiNode {
    bind_state(child, HIT_TEST_MODE_VAR.map(|m| m.is_hit_testable()), state)
}
