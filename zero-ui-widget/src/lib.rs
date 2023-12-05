//! Basic widget properties and helpers for declaring widgets and properties.

use std::fmt;

use zero_ui_app::{
    widget::{
        base::{Parallel, PARALLEL_VAR},
        border::{BORDER_ALIGN_VAR, BORDER_OVER_VAR, CORNER_RADIUS_FIT_VAR, CORNER_RADIUS_VAR},
        info::{Interactivity, Visibility},
        instance::*,
        *,
    },
    window::WINDOW,
};
use zero_ui_clone_move::clmv;
use zero_ui_var::*;

pub mod nodes;

/// Minimal widget.
///
/// You can use this to create a quick new custom widget that is only used in one code place and can be created entirely
/// by properties and `when` conditions.
#[widget($crate::Wgt)]
pub struct Wgt(VisibilityMix<HitTestMix<base::WidgetBase>>);

/// Defines the render order of a widget in a layout panel.
///
/// When set the widget will still update and layout according to their *logical* position in the list but
/// they will render according to the order defined by the [`ZIndex`] value.
///
/// Layout panels that support this property should mention it in their documentation, implementers
/// see [`PanelList`] for more details.
///
/// An error is logged on init if the widget is not a direct child of a Z-sorting panel.
#[property(CONTEXT, default(ZIndex::DEFAULT))]
pub fn z_index(child: impl UiNode, index: impl IntoVar<ZIndex>) -> impl UiNode {
    let index = index.into_var();
    let mut valid = false;

    match_node(child, move |_, op| match op {
        UiNodeOp::Init => {
            valid = Z_INDEX.set(index.get());

            if valid {
                WIDGET.sub_var(&index);
            } else {
                tracing::error!(
                    "property `z_index` set for `{}` but it is not the direct child of a Z-sorting panel",
                    WIDGET.trace_id()
                );
            }
        }
        UiNodeOp::Update { .. } => {
            if valid {
                if let Some(i) = index.get_new() {
                    assert!(Z_INDEX.set(i));
                }
            }
        }
        _ => {}
    })
}

/// Interactivity properties.
///
/// Mixin defines enabled and enabled state probing properties for interactive widgets.
#[widget_mixin]
pub struct InteractivityMix<P>(P);

context_var! {
    static IS_ENABLED_VAR: bool = true;
}

/// If default interaction is allowed in the widget and its descendants.
///
/// This property sets the interactivity of the widget to [`ENABLED`] or [`DISABLED`], to probe the enabled state in `when` clauses
/// use [`is_enabled`] or [`is_disabled`]. To probe the a widget's state use [`interactivity`] value.
///
/// # Interactivity
///
/// Every widget has an [`interactivity`] value, it defines two *tiers* of disabled, the normal disabled blocks the default actions
/// of the widget, but still allows some interactions, such as a different cursor on hover or event an error tool-tip on click, the
/// second tier blocks all interaction with the widget. This property controls the *normal* disabled, to fully block interaction use
/// the [`interactive`] property.
///
/// # Disabled Visual
///
/// Widgets that are interactive should visually indicate when the normal interactions are disabled, you can use the [`is_disabled`]
/// state property in a when block to implement the *visually disabled* appearance of a widget.
///
/// The visual cue for the disabled state is usually a reduced contrast from content and background by *graying-out* the text and applying a
/// grayscale filter for image content. You should also consider adding *disabled interactions* that inform the user when the widget will be
/// enabled.
///
/// # Implicit
///
/// This property is included in all widgets by default, you don't need to import it to use it.
///
/// [`ENABLED`]: zero_ui_app::widget::info::Interactivity::ENABLED
/// [`DISABLED`]: zero_ui_app::widget::info::Interactivity::DISABLED
/// [`interactivity`]: zero_ui_app::widget::info::WidgetInfo::interactivity
/// [`interactive`]: fn@interactive
/// [`is_enabled`]: fn@is_enabled
/// [`is_disabled`]: fn@is_disabled
#[property(CONTEXT, default(true), widget_impl(InteractivityMix<P>))]
pub fn enabled(child: impl UiNode, enabled: impl IntoVar<bool>) -> impl UiNode {
    let enabled = enabled.into_var();

    let child = match_node(
        child,
        clmv!(enabled, |_, op| match op {
            UiNodeOp::Init => {
                WIDGET.sub_var_info(&enabled);
            }
            UiNodeOp::Info { info } => {
                if !enabled.get() {
                    info.push_interactivity(Interactivity::DISABLED);
                }
            }
            _ => {}
        }),
    );

    nodes::with_context_var(child, IS_ENABLED_VAR, merge_var!(IS_ENABLED_VAR, enabled, |&a, &b| a && b))
}

/// Defines if any interaction is allowed in the widget and its descendants.
///
/// This property sets the interactivity of the widget to [`BLOCKED`] when `false`, widgets with blocked interactivity do not
/// receive any interaction event and behave like a background visual. To probe the widget state use [`interactivity`] value.
///
/// This property *enables* and *disables* interaction with the widget and its descendants without causing
/// a visual change like [`enabled`], it also blocks "disabled" interactions such as a different cursor or tool-tip for disabled buttons,
/// its use cases are more advanced then [`enabled`], it is mostly used when large parts of the screen are "not ready".
///
/// Note that this affects the widget where it is set and descendants, to disable interaction only in the widgets
/// inside `child` use the [`base::nodes::interactive_node`].
///
/// [`enabled`]: fn@enabled
/// [`BLOCKED`]: Interactivity::BLOCKED
/// [`interactivity`]: zero_ui_app::widget::info::WidgetInfo::interactivity
#[property(CONTEXT, default(true), widget_impl(InteractivityMix<P>))]
pub fn interactive(child: impl UiNode, interactive: impl IntoVar<bool>) -> impl UiNode {
    let interactive = interactive.into_var();

    match_node(child, move |_, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_info(&interactive);
        }
        UiNodeOp::Info { info } => {
            if !interactive.get() {
                info.push_interactivity(Interactivity::BLOCKED);
            }
        }
        _ => {}
    })
}

/// If the widget is enabled for interaction.
///
/// This property is used only for probing the state. You can set the state using
/// the [`enabled`] property.
///
/// [`enabled`]: fn@enabled
/// [`WidgetInfo::allow_interaction`]: crate::widget_info::WidgetInfo::allow_interaction
#[property(EVENT, widget_impl(InteractivityMix<P>))]
pub fn is_enabled(child: impl UiNode, state: impl IntoVar<bool>) -> impl UiNode {
    vis_enabled_eq_state(child, state, true)
}
/// If the widget is disabled for interaction.
///
/// This property is used only for probing the state. You can set the state using
/// the [`enabled`] property.
///
/// This is the same as `!self.is_enabled`.
///
/// [`enabled`]: fn@enabled
#[property(EVENT, widget_impl(InteractivityMix<P>))]
pub fn is_disabled(child: impl UiNode, state: impl IntoVar<bool>) -> impl UiNode {
    vis_enabled_eq_state(child, state, false)
}

/// Visibility properties.
///
/// Mixin defines visibility and visibility state probing properties for all widgets.
#[widget_mixin]
pub struct VisibilityMix<P>(P);

fn vis_enabled_eq_state(child: impl UiNode, state: impl IntoVar<bool>, expected: bool) -> impl UiNode {
    nodes::event_is_state(child, state, true, info::INTERACTIVITY_CHANGED_EVENT, move |args| {
        if let Some((_, new)) = args.vis_enabled_change(WIDGET.id()) {
            Some(new.is_vis_enabled() == expected)
        } else {
            None
        }
    })
}

/// Sets the widget visibility.
///
/// This property causes the widget to have the `visibility`, the widget actual visibility is computed, for example,
/// widgets that don't render anything are considered `Hidden` even if the visibility property is not set, this property
/// only forces the widget to layout and render according to the specified visibility.
///
/// To probe the visibility state of a widget in `when` clauses use [`is_visible`], [`is_hidden`] or [`is_collapsed`] in `when` clauses,
/// to probe a widget state use [`UiNode::with_context`] or [`WidgetInfo::visibility`].
///
/// # Implicit
///
/// This property is included in all widgets by default, you don't need to import it to use it.
///
/// [`is_visible`]: fn@is_visible
/// [`is_hidden`]: fn@is_hidden
/// [`is_collapsed`]: fn@is_collapsed
/// [`WidgetInfo::visibility`]: zero_ui_app::widget::info::WidgetInfo::visibility
#[property(CONTEXT, default(true), widget_impl(VisibilityMix<P>))]
pub fn visibility(child: impl UiNode, visibility: impl IntoVar<Visibility>) -> impl UiNode {
    let visibility = visibility.into_var();
    let mut prev_vis = Visibility::Visible;

    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var(&visibility);
            prev_vis = visibility.get();
        }
        UiNodeOp::Update { .. } => {
            if let Some(vis) = visibility.get_new() {
                use Visibility::*;
                match (prev_vis, vis) {
                    (Collapsed, Visible) | (Visible, Collapsed) => {
                        WIDGET.layout().render();
                    }
                    (Hidden, Visible) | (Visible, Hidden) => {
                        WIDGET.render();
                    }
                    (Collapsed, Hidden) | (Hidden, Collapsed) => {
                        WIDGET.layout();
                    }
                    _ => {}
                }
                prev_vis = vis;
            }
        }

        UiNodeOp::Measure { wm, desired_size } => {
            if Visibility::Collapsed != visibility.get() {
                *desired_size = child.measure(wm);
            } else {
                child.delegated();
            }
        }
        UiNodeOp::Layout { wl, final_size } => {
            if Visibility::Collapsed != visibility.get() {
                *final_size = child.layout(wl);
            } else {
                wl.collapse();
                child.delegated();
            }
        }

        UiNodeOp::Render { frame } => match visibility.get() {
            Visibility::Visible => child.render(frame),
            Visibility::Hidden => frame.hide(|frame| child.render(frame)),
            Visibility::Collapsed => {
                child.delegated();
                #[cfg(debug_assertions)]
                {
                    tracing::error!(
                        "collapsed {} rendered, to fix, layout the widget, or `WidgetLayout::collapse_child` the widget",
                        WIDGET.trace_id()
                    )
                }
            }
        },
        UiNodeOp::RenderUpdate { update } => match visibility.get() {
            Visibility::Visible => child.render_update(update),
            Visibility::Hidden => update.hidden(|update| child.render_update(update)),
            Visibility::Collapsed => {
                child.delegated();
                #[cfg(debug_assertions)]
                {
                    tracing::error!(
                        "collapsed {} render-updated, to fix, layout the widget, or `WidgetLayout::collapse_child` the widget",
                        WIDGET.trace_id()
                    )
                }
            }
        },
        _ => {}
    })
}

fn visibility_eq_state(child: impl UiNode, state: impl IntoVar<bool>, expected: Visibility) -> impl UiNode {
    nodes::event_is_state(
        child,
        state,
        expected == Visibility::Visible,
        info::VISIBILITY_CHANGED_EVENT,
        move |_| {
            let tree = WINDOW.info();
            let vis = tree.get(WIDGET.id()).map(|w| w.visibility()).unwrap_or(Visibility::Visible);

            Some(vis == expected)
        },
    )
}
/// If the widget is [`Visible`](Visibility::Visible).
#[property(CONTEXT, widget_impl(VisibilityMix<P>))]
pub fn is_visible(child: impl UiNode, state: impl IntoVar<bool>) -> impl UiNode {
    visibility_eq_state(child, state, Visibility::Visible)
}
/// If the widget is [`Hidden`](Visibility::Hidden).
#[property(CONTEXT, widget_impl(VisibilityMix<P>))]
pub fn is_hidden(child: impl UiNode, state: impl IntoVar<bool>) -> impl UiNode {
    visibility_eq_state(child, state, Visibility::Hidden)
}
/// If the widget is [`Collapsed`](Visibility::Collapsed).
#[property(CONTEXT, widget_impl(VisibilityMix<P>))]
pub fn is_collapsed(child: impl UiNode, state: impl IntoVar<bool>) -> impl UiNode {
    visibility_eq_state(child, state, Visibility::Collapsed)
}

/// Defines if the widget only renders if it-s bounds intersects with the viewport auto-hide rectangle.
///
/// The auto-hide rect is usually *one viewport* of extra space around the  viewport, so only widgets that transform
/// themselves very far need to set this, disabling auto-hide for an widget does not disable it for descendants.
///
/// # Examples
///
/// The example demonstrates a `container` that is *fixed* in the scroll viewport, it sets the `x` and `y` properties
/// to always stay in frame, but transforms set by a widget on itself always affects  the [`inner_bounds`], the
/// [`outer_bounds`] will still be the transform set by the parent so the container may end-up auto-hidden.
///
/// Note that auto-hide is not disabled for the `content` widget, but it's [`outer_bounds`] is affected by the `container`
/// so it is auto-hidden correctly.
///
/// ```
/// # macro_rules! Container { ($($tt:tt)*) => { NilUiNode }}
/// # use zero_ui_app::widget::instance::*;
/// fn center_viewport(content: impl UiNode) -> impl UiNode {
///     Container! {
///         zero_ui::core::widget_base::can_auto_hide = false;
///
///         x = zero_ui::widgets::scroll::SCROLL_HORIZONTAL_OFFSET_VAR.map(|&fct| Length::Relative(fct) - 1.vw() * fct);
///         y = zero_ui::widgets::scroll::SCROLL_VERTICAL_OFFSET_VAR.map(|&fct| Length::Relative(fct) - 1.vh() * fct);
///         max_size = (1.vw(), 1.vh());
///         content_align = Align::CENTER;
///      
///         content;
///     }
/// }
/// ```
///  
/// [`outer_bounds`]: info::WidgetBoundsInfo::outer_bounds
/// [`inner_bounds`]: info::WidgetBoundsInfo::inner_bounds
#[property(CONTEXT, default(true), widget_impl(VisibilityMix<P>))]
pub fn can_auto_hide(child: impl UiNode, enabled: impl IntoVar<bool>) -> impl UiNode {
    let enabled = enabled.into_var();

    match_node(child, move |_, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var(&enabled);
        }
        UiNodeOp::Update { .. } => {
            if let Some(new) = enabled.get_new() {
                if WIDGET.bounds().can_auto_hide() != new {
                    WIDGET.layout().render();
                }
            }
        }
        UiNodeOp::Layout { wl, .. } => {
            wl.allow_auto_hide(enabled.get());
        }
        _ => {}
    })
}

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
/// [`corner_radius`]: fn@corner_radius
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

    nodes::with_context_var(
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
    nodes::bind_is_state(child, HIT_TEST_MODE_VAR.map(|m| m.is_hit_testable()), state)
}

/// Defines what node list methods can run in parallel in the widget and descendants.
///
/// This property sets the [`PARALLEL_VAR`] that is used by [`UiNodeList`] implementers to toggle parallel processing.
///
/// See also `WINDOWS.parallel` to define parallelization in multi-window apps.
///
/// [`UiNode`]: zero_ui_app::widget::instance::UiNodeList
#[property(CONTEXT, default(PARALLEL_VAR))]
pub fn parallel(child: impl UiNode, enabled: impl IntoVar<Parallel>) -> impl UiNode {
    nodes::with_context_var(child, PARALLEL_VAR, enabled)
}

/// Border control properties.
#[widget_mixin]
pub struct BorderMix<P>(P);

/// Corner radius of widget and inner widgets.
///
/// The [`Default`] value is calculated to fit inside the parent widget corner curve, see [`corner_radius_fit`].
///
/// [`Default`]: zero_ui_layout::units::Length::Default
/// [`corner_radius_fit`]: fn@corner_radius_fit
#[property(CONTEXT, default(CORNER_RADIUS_VAR), widget_impl(BorderMix<P>))]
pub fn corner_radius(child: impl UiNode, radius: impl IntoVar<border::CornerRadius>) -> impl UiNode {
    let child = match_node(child, move |child, op| {
        if let UiNodeOp::Layout { wl, final_size } = op {
            *final_size = border::BORDER.with_corner_radius(|| child.layout(wl));
        }
    });
    nodes::with_context_var(child, CORNER_RADIUS_VAR, radius)
}

/// Defines how the [`corner_radius`] is computed for each usage.
///
/// Nesting borders with round corners need slightly different radius values to perfectly fit, the [`BORDER`]
/// coordinator can adjusts the radius inside each border to match the inside curve of the border.
///
/// Sets the [`CORNER_RADIUS_FIT_VAR`].
///
/// [`corner_radius`]: fn@corner_radius
/// [`BORDER`]: zero_ui_app::widget::border::BORDER
#[property(CONTEXT, default(CORNER_RADIUS_FIT_VAR), widget_impl(BorderMix<P>))]
pub fn corner_radius_fit(child: impl UiNode, fit: impl IntoVar<border::CornerRadiusFit>) -> impl UiNode {
    nodes::with_context_var(child, CORNER_RADIUS_FIT_VAR, fit)
}

/// Position of a widget borders in relation to the widget fill.
///
/// This property defines how much the widget's border offsets affect the layout of the fill content, by default
/// (0%) the fill content stretchers *under* the borders and is clipped by the [`corner_radius`], in the other end
/// of the scale (100%), the fill content is positioned *inside* the borders and clipped by the adjusted [`corner_radius`]
/// that fits the insider of the inner most border.
///
/// Note that widget's content is always *inside* the borders, this property only affects the *fill* properties content, such as a
/// the image in a background image.
///
/// Fill property implementers, see [`nodes::fill_node`], a helper function for quickly implementing support for `border_align`.
///
/// Sets the [`BORDER_ALIGN_VAR`].
///
/// [`corner_radius`]: fn@corner_radius
#[property(CONTEXT, default(BORDER_ALIGN_VAR), widget_impl(BorderMix<P>))]
pub fn border_align(child: impl UiNode, align: impl IntoVar<zero_ui_layout::units::FactorSideOffsets>) -> impl UiNode {
    nodes::with_context_var(child, BORDER_ALIGN_VAR, align)
}

/// If the border is rendered over the fill and child visuals.
///
/// Is `true` by default, if set to `false` the borders will render under the fill. Note that
/// this means the border will be occluded by the *background* if [`border_align`] is not set to `1.fct()`.
///
/// Sets the [`BORDER_OVER_VAR`].
///
/// [`border_align`]: fn@border_align
#[property(CONTEXT, default(BORDER_OVER_VAR), widget_impl(BorderMix<P>))]
pub fn border_over(child: impl UiNode, over: impl IntoVar<bool>) -> impl UiNode {
    nodes::with_context_var(child, BORDER_OVER_VAR, over)
}
