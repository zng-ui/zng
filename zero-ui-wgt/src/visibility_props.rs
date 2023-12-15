use crate::prelude::*;

use zero_ui_app::widget::info;

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
#[property(CONTEXT, default(true))]
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
    event_is_state(
        child,
        state,
        expected == Visibility::Visible,
        info::VISIBILITY_CHANGED_EVENT,
        move |a| {
            let vis = a.tree.get(WIDGET.id()).map(|w| w.visibility()).unwrap_or(Visibility::Visible);

            Some(vis == expected)
        },
    )
}
/// If the widget is [`Visible`](Visibility::Visible).
#[property(CONTEXT)]
pub fn is_visible(child: impl UiNode, state: impl IntoVar<bool>) -> impl UiNode {
    visibility_eq_state(child, state, Visibility::Visible)
}
/// If the widget is [`Hidden`](Visibility::Hidden).
#[property(CONTEXT)]
pub fn is_hidden(child: impl UiNode, state: impl IntoVar<bool>) -> impl UiNode {
    visibility_eq_state(child, state, Visibility::Hidden)
}
/// If the widget is [`Collapsed`](Visibility::Collapsed).
#[property(CONTEXT)]
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
#[property(CONTEXT, default(true))]
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
}
