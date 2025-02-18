#![cfg(feature = "drag_drop")]

//! Drag&drop properties, event properties.

use zng_ext_input::drag_drop::{
    DragEndArgs, DragHoveredArgs, DragStartArgs, DropArgs, WidgetInfoBuilderDragDropExt as _, DRAG_END_EVENT, DRAG_HOVERED_EVENT,
    DRAG_START_EVENT, DROP_EVENT,
};
use zng_wgt::prelude::*;

/// If this widget can be dragged in a drag&drop operation.
///
/// When this is `true` the widget can be dragged and dropped within the same app or it can handle [`on_drag_start`] and
/// use the [`DRAG_DROP.drag`] service to set a system wide drag data.
///
/// [`on_drag_start`]: fn@on_drag_start
/// [`DRAG_DROP.drag`]: zng_ext_input::drag_drop::DRAG_DROP::drag
#[property(CONTEXT, default(false))]
pub fn draggable(child: impl UiNode, input: impl IntoVar<bool>) -> impl UiNode {
    let input = input.into_var();
    match_node(child, move |_c, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_info(&input);
        }
        UiNodeOp::Info { info } => {
            if input.get() {
                info.draggable();
            }
        }
        _ => {}
    })
}

event_property! {
    /// Draggable widget started dragging.
    ///
    /// To receive this event in a widget set [`draggable`] to `true`.
    ///
    /// [`draggable`]: fn@draggable
    pub fn drag_start {
        event: DRAG_START_EVENT,
        args: DragStartArgs,
    }

    /// Draggable widget stopped dragging.
    ///
    /// This event is always paired with [`on_drag_start`] first.
    ///
    /// [`on_drag_start`]: fn@on_drag_start
    pub fn drag_end {
        event: DRAG_END_EVENT,
        args: DragEndArgs,
    }

    /// Dragging cursor entered or exited the widget area and the widget is enabled.
    pub fn drag_hovered {
        event: DRAG_HOVERED_EVENT,
        args: DragHoveredArgs,
        filter: |args| args.is_drag_enter_enabled()
    }
    /// Dragging cursor entered the widget area and the widget is enabled.
    pub fn drag_enter {
        event: DRAG_HOVERED_EVENT,
        args: DragHoveredArgs,
        filter: |args| args.is_drag_enter_enabled(),
    }
    /// Dragging cursor exited the widget area and the widget is enabled.
    pub fn drag_leave {
        event: DRAG_HOVERED_EVENT,
        args: DragHoveredArgs,
        filter: |args| args.is_drag_leave_enabled(),
    }

    /// Dragging cursor dropped data in the widget area and the widget is enabled.
    pub fn drop {
        event: DROP_EVENT,
        args: DropArgs,
        filter: |args| args.is_enabled(WIDGET.id()),
    }
}

/// If the dragging cursor is over the widget or a descendant and the widget is enabled.
///
/// The value is always `false` when the widget is not [`ENABLED`], use [`is_drag_hovered_disabled`] to implement *disabled hovered* visuals.
///
/// [`ENABLED`]: Interactivity::ENABLED
/// [`is_drag_hovered_disabled`]: fn@is_drag_hovered_disabled
#[property(EVENT)]
pub fn is_drag_hovered(child: impl UiNode, state: impl IntoVar<bool>) -> impl UiNode {
    event_state(child, state, false, DRAG_HOVERED_EVENT, |args| {
        if args.is_drag_enter_enabled() {
            Some(true)
        } else if args.is_drag_leave_enabled() {
            Some(false)
        } else {
            None
        }
    })
}

/// If the dragging cursor is over the widget or a descendant and the widget is disabled.
#[property(EVENT)]
pub fn is_drag_hovered_disabled(child: impl UiNode, state: impl IntoVar<bool>) -> impl UiNode {
    event_state(child, state, false, DRAG_HOVERED_EVENT, |args| {
        if args.is_drag_enter_disabled() {
            Some(true)
        } else if args.is_drag_leave_disabled() {
            Some(false)
        } else {
            None
        }
    })
}

/// If the draggable widget is dragging.
#[property(EVENT)]
pub fn is_dragging(child: impl UiNode, state: impl IntoVar<bool>) -> impl UiNode {
    let state = state.into_var();
    match_node(child, move |_, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_event(&DRAG_START_EVENT).sub_event(&DRAG_END_EVENT);
        }
        UiNodeOp::Deinit => {
            let _ = state.set(false);
        }
        UiNodeOp::Event { update } => {
            if let Some(args) = DRAG_START_EVENT.on(update) {
                if args.target.contains(WIDGET.id()) {
                    let _ = state.set(true);
                }
            } else if let Some(args) = DRAG_END_EVENT.on(update) {
                if args.target.contains(WIDGET.id()) {
                    let _ = state.set(false);
                }
            }
        }
        _ => {}
    })
}
