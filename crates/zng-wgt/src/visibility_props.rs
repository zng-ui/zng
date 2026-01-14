use crate::prelude::*;

use zng_app::{view_process::raw_events::RAW_FRAME_RENDERED_EVENT, window::WINDOWS_APP};

/// Sets the widget visibility.
///
/// This property causes the widget to have the `visibility`, the widget actual visibility is computed, for example,
/// widgets that don't render anything are considered `Hidden` even if the visibility property is not set, this property
/// only forces the widget to layout and render according to the specified visibility.
///
/// To probe the visibility state of a widget in `when` clauses use [`is_visible`], [`is_hidden`] or [`is_collapsed`],
/// to probe a widget state use [`WidgetInfo::visibility`].
///
/// [`is_visible`]: fn@is_visible
/// [`is_hidden`]: fn@is_hidden
/// [`is_collapsed`]: fn@is_collapsed
/// [`WidgetInfo::visibility`]: zng_app::widget::info::WidgetInfo::visibility
#[property(CONTEXT, default(true))]
pub fn visibility(child: impl IntoUiNode, visibility: impl IntoVar<Visibility>) -> UiNode {
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

fn visibility_eq_state(child: impl IntoUiNode, state: impl IntoVar<bool>, expected: Visibility) -> UiNode {
    let state = state.into_var();
    match_node(child, move |_, op| {
        if let UiNodeOp::Init = op {
            let w_id = WINDOW.id();
            let id = WIDGET.id();
            // !!: TODO, this is not the best event, we don't need to await render,
            // can't just modify in UiNodeOp::Render either because a parent may collapse us
            WIDGET.push_var_handle(RAW_FRAME_RENDERED_EVENT.hook(clmv!(state, |a| {
                if a.window_id == w_id
                    && let Some(w) = WINDOWS_APP.widget_tree(w_id)
                    && let Some(w) = w.get(id)
                {
                    let ns = w.visibility() == expected;
                    if ns != state.get() {
                        state.set(ns);
                    }
                }
                true
            })));
        }
    })
}
/// If the widget is [`Visible`](Visibility::Visible).
#[property(CONTEXT)]
pub fn is_visible(child: impl IntoUiNode, state: impl IntoVar<bool>) -> UiNode {
    visibility_eq_state(child, state, Visibility::Visible)
}
/// If the widget is [`Hidden`](Visibility::Hidden).
#[property(CONTEXT)]
pub fn is_hidden(child: impl IntoUiNode, state: impl IntoVar<bool>) -> UiNode {
    visibility_eq_state(child, state, Visibility::Hidden)
}
/// If the widget is [`Collapsed`](Visibility::Collapsed).
#[property(CONTEXT)]
pub fn is_collapsed(child: impl IntoUiNode, state: impl IntoVar<bool>) -> UiNode {
    visibility_eq_state(child, state, Visibility::Collapsed)
}

/// Defines if the widget only renders if it's bounds intersects with the viewport auto-hide rectangle.
///
/// The auto-hide rect is usually `(1.vw(), 1.vh())` of extra space around the viewport, so only widgets that transform
/// themselves very far need to set this, disabling auto-hide for a widget does not disable it for descendants.
///
/// # Examples
///
/// The example demonstrates a container that is fixed in the scroll viewport, it sets the `x` and `y` properties
/// to always stay in frame. Because the container is layout out of view and just transformed back into view it
/// auto-hides while visible, the example uses `auto_hide = false;` to fix the issue.
///
/// ```
/// # macro_rules! Container { ($($tt:tt)*) => { UiNode::nil() }}
/// # use zng_app::widget::node::*;
/// fn center_viewport(msg: impl IntoUiNode) -> UiNode {
///     Container! {
///         layout::x = merge_var!(SCROLL.horizontal_offset(), SCROLL.zoom_scale(), |&h, &s| h.0.fct_l()
///             - 1.vw() / s * h);
///         layout::y = merge_var!(SCROLL.vertical_offset(), SCROLL.zoom_scale(), |&v, &s| v.0.fct_l() - 1.vh() / s * v);
///         layout::scale = SCROLL.zoom_scale().map(|&fct| 1.fct() / fct);
///         layout::transform_origin = 0;
///         widget::auto_hide = false;
///         layout::max_size = (1.vw(), 1.vh());
///
///         child_align = Align::CENTER;
///         child = msg;
///     }
/// }
/// ```
#[property(CONTEXT, default(true))]
pub fn auto_hide(child: impl IntoUiNode, enabled: impl IntoVar<bool>) -> UiNode {
    let enabled = enabled.into_var();

    match_node(child, move |_, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var(&enabled);
        }
        UiNodeOp::Update { .. } => {
            if let Some(new) = enabled.get_new()
                && WIDGET.bounds().can_auto_hide() != new
            {
                WIDGET.layout().render();
            }
        }
        UiNodeOp::Layout { wl, .. } => {
            wl.allow_auto_hide(enabled.get());
        }
        _ => {}
    })
}

event_property! {
    /// Widget global inner transform changed.
    pub fn transform_changed {
        event: info::TRANSFORM_CHANGED_EVENT,
        args: info::TransformChangedArgs,
    }

    /// Widget global position changed.
    pub fn move {
        event: info::TRANSFORM_CHANGED_EVENT,
        args: info::TransformChangedArgs,
        filter: |a| a.offset(WIDGET.id()).unwrap_or_default() != PxVector::zero(),
    }

    /// Widget visibility changed.
    pub fn visibility_changed {
        event: info::VISIBILITY_CHANGED_EVENT,
        args: info::VisibilityChangedArgs,
    }

    /// Widget visibility changed to collapsed.
    pub fn collapse {
        event: info::VISIBILITY_CHANGED_EVENT,
        args: info::VisibilityChangedArgs,
        filter: |a| a.is_collapse(WIDGET.id()),
    }

    /// Widget visibility changed to hidden.
    pub fn hide {
        event: info::VISIBILITY_CHANGED_EVENT,
        args: info::VisibilityChangedArgs,
        filter: |a| a.is_hide(WIDGET.id()),
    }

    /// Widget visibility changed to visible.
    ///
    /// Note that widgets are **already marked visible** before the first render so this event does not fire on init.
    pub fn show {
        event: info::VISIBILITY_CHANGED_EVENT,
        args: info::VisibilityChangedArgs,
        filter: |a| a.is_show(WIDGET.id()),
    }
}
