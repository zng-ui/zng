use std::fmt;

use crate::prelude::*;

/// Hit-test visibility properties.
///
/// Mixin defines hit-test control state probing properties for all widgets.
#[widget_mixin]
pub struct HitTestMix<P>(P);

/// Defines if and how a widget is hit-tested.
///
/// See [`hit_test_mode`](fn@hit_test_mode) for more details.
#[derive(Copy, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum HitTestMode {
    /// Widget is never hit.
    ///
    /// This mode affects the entire UI branch, if set it disables hit-testing for the widget and all its descendants.
    Disabled,
    /// Widget is hit by any point that intersects the transformed inner bounds rectangle. If the widget is inlined
    /// excludes the first row advance and the last row trailing space.
    Bounds,
    /// Default mode.
    ///
    /// Same as `Bounds`, but also excludes the outside of rounded corners.
    #[default]
    RoundedBounds,
    /// Every render primitive used for rendering the widget is hit-testable, the widget is hit only by
    /// points that intersect visible parts of the render primitives.
    ///
    /// Note that not all primitives implement pixel accurate hit-testing.
    Visual,
}
impl HitTestMode {
    /// Returns `true` if is any mode other then [`Disabled`].
    ///
    /// [`Disabled`]: Self::Disabled
    pub fn is_hit_testable(&self) -> bool {
        !matches!(self, Self::Disabled)
    }

    /// Read-only context var with the contextual mode.
    pub fn var() -> impl Var<HitTestMode> {
        HIT_TEST_MODE_VAR.read_only()
    }
}
impl fmt::Debug for HitTestMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "HitTestMode::")?;
        }
        match self {
            Self::Disabled => write!(f, "Disabled"),
            Self::Bounds => write!(f, "Bounds"),
            Self::RoundedBounds => write!(f, "RoundedBounds"),
            Self::Visual => write!(f, "Visual"),
        }
    }
}
impl_from_and_into_var! {
    fn from(default_or_disabled: bool) -> HitTestMode {
        if default_or_disabled {
            HitTestMode::default()
        } else {
            HitTestMode::Disabled
        }
    }
}

context_var! {
    static HIT_TEST_MODE_VAR: HitTestMode = HitTestMode::default();
}

/// Defines how the widget is hit-tested.
///
/// Hit-testing determines if a point intersects with the widget, the most common hit-test point is the mouse pointer.
/// By default widgets are hit by any point inside the widget area, excluding the outer corners if [`corner_radius`] is set,
/// this is very efficient, but assumes that the widget is *filled*, if the widget has visual *holes* the user may be able
/// to see another widget underneath but be unable to click on it.
///
/// If you have a widget with a complex shape or with *holes*, set this property to [`HitTestMode::Visual`] to enable the full
/// hit-testing power where all render primitives and clips used to render the widget are considered during hit-testing.
///
/// [`hit_testable`]: fn@hit_testable
/// [`corner_radius`]: fn@crate::corner_radius
#[property(CONTEXT, default(HIT_TEST_MODE_VAR), widget_impl(HitTestMix<P>))]
pub fn hit_test_mode(child: impl UiNode, mode: impl IntoVar<HitTestMode>) -> impl UiNode {
    let child = match_node(child, |child, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_render(&HitTestMode::var());
        }
        UiNodeOp::Render { frame } => match HitTestMode::var().get() {
            HitTestMode::Disabled => {
                frame.with_hit_tests_disabled(|frame| child.render(frame));
            }
            HitTestMode::Visual => frame.with_auto_hit_test(true, |frame| child.render(frame)),
            _ => frame.with_auto_hit_test(false, |frame| child.render(frame)),
        },
        UiNodeOp::RenderUpdate { update } => {
            update.with_auto_hit_test(matches!(HitTestMode::var().get(), HitTestMode::Visual), |update| {
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
/// [`hit_testable`]: fn@hit_testable
/// [`hit_test_mode`]: fn@hit_test_mode
#[property(EVENT, widget_impl(HitTestMix<P>))]
pub fn is_hit_testable(child: impl UiNode, state: impl IntoVar<bool>) -> impl UiNode {
    bind_is_state(child, HIT_TEST_MODE_VAR.map(|m| m.is_hit_testable()), state)
}
