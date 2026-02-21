use crate::{event_property, node::VarEventNodeBuilder, prelude::*};

use zng_app::widget::info::WIDGET_TREE_CHANGED_EVENT;

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
            WIDGET.push_var_handle(WIDGET_TREE_CHANGED_EVENT.var_bind(&state, move |a| {
                if a.tree.window_id() == w_id
                    && let Some(w) = a.tree.get(id)
                {
                    Some(w.visibility() == expected)
                } else {
                    None
                }
            }));
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

macro_rules! visibility_var_event_source {
    (|$visibility:ident| $map:expr, $default:expr) => {
        $crate::node::VarEventNodeBuilder::new(|| {
            let win_id = WINDOW.id();
            let wgt_id = WIDGET.id();
            WIDGET_TREE_CHANGED_EVENT.var_map(
                move |a| {
                    if a.tree.window_id() == win_id
                        && let Some(w) = a.tree.get(wgt_id)
                    {
                        Some({
                            let $visibility = w.visibility();
                            $map
                        })
                    } else {
                        None
                    }
                },
                || $default,
            )
        })
    };
}

event_property! {
    /// Widget global inner transform changed.
    #[property(EVENT)]
    pub fn on_transform_changed(child: impl IntoUiNode, handler: Handler<PxTransform>) -> UiNode {
        VarEventNodeBuilder::new(|| {
            let win_id = WINDOW.id();
            let wgt_id = WIDGET.id();
            WIDGET_TREE_CHANGED_EVENT.var_map(
                move |a| {
                    if a.tree.window_id() == win_id
                        && let Some(w) = a.tree.get(wgt_id)
                    {
                        Some(w.inner_transform())
                    } else {
                        None
                    }
                },
                PxTransform::identity,
            )
        })
        .build::<false>(child, handler)
    }

    /// Widget visibility changed.
    ///
    /// Note that there are multiple specific events for visibility changes, [`on_visible`], [`on_hidden`] and [`on_collapsed`].
    ///
    /// [`on_visible`]: fn@on_visible
    /// [`on_hidden`]: fn@on_hidden
    /// [`on_collapsed`]: fn@on_collapsed
    #[property(EVENT)]
    pub fn on_visibility_changed(child: impl IntoUiNode, handler: Handler<Visibility>) -> UiNode {
        visibility_var_event_source!(|v| v, Visibility::Visible).build::<false>(child, handler)
    }

    /// Widget visibility changed to visible.
    ///
    /// See [`on_visibility_changed`] for a more general visibility event.
    ///
    /// Note that widgets are visible by default, so this will not notify on init.
    ///
    /// [`on_visibility_changed`]: fn@on_visibility_changed
    #[property(EVENT)]
    pub fn on_visible(child: impl IntoUiNode, handler: Handler<()>) -> UiNode {
        visibility_var_event_source!(|v| v.is_visible(), true)
            .filter(|| |v| *v)
            .map_args(|_| ())
            .build::<false>(child, handler)
    }

    /// Widget visibility changed to hidden.
    ///
    /// See [`on_visibility_changed`] for a more general visibility event.
    ///
    /// [`on_visibility_changed`]: fn@on_visibility_changed
    #[property(EVENT)]
    pub fn on_hidden(child: impl IntoUiNode, handler: Handler<()>) -> UiNode {
        visibility_var_event_source!(|v| v.is_hidden(), true)
            .filter(|| |v| *v)
            .map_args(|_| ())
            .build::<false>(child, handler)
    }

    /// Widget visibility changed to collapsed.
    ///
    /// See [`on_visibility_changed`] for a more general visibility event.
    ///
    /// [`on_visibility_changed`]: fn@on_visibility_changed
    #[property(EVENT)]
    pub fn on_collapsed(child: impl IntoUiNode, handler: Handler<()>) -> UiNode {
        visibility_var_event_source!(|v| v.is_collapsed(), true)
            .filter(|| |v| *v)
            .map_args(|_| ())
            .build::<false>(child, handler)
    }
}
