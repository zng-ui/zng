//! Drag& drop properties, event properties.

use zng_ext_input::drag_drop::{
    DragEndArgs, DragHoverArgs, DragStartArgs, DropArgs, DRAG_END_EVENT, DRAG_HOVER_EVENT, DRAG_START_EVENT, DROP_EVENT,
};
use zng_wgt::prelude::*;

/// If this widget can be dragged in a drag&drop operation.
///
/// When this is `true` the widget can be dragged and dropped within the same app or it can handle [`on_drag_start`] and
/// use the [`DRAG_DROP.drag`] service to set a system wide drag data.
///
/// [`on_drag_start`]: fn@on_drag_start
/// [`DRAG_DROP.drag`]: zng_ext_input::drag_drop::DRAG_DROP::drag
#[property(CONTEXT)]
pub fn draggable(child: impl UiNode, input: impl IntoVar<bool>) -> impl UiNode {
    let input = input.into_var();
    match_node(child, move |_c, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_info(&input);
        }
        UiNodeOp::Info { info } => {
            // !!: TODO
        }
        _ => {}
    })
}

// !!: TODO filter like mouse.rs
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

    /// Drag operation started from the draggable widget has ended.
    pub fn drag_end {
        event: DRAG_END_EVENT,
        args: DragEndArgs,
    }

    /// Drag&drop operation entered or exited the widget area.
    pub fn drag_hover {
        event: DRAG_HOVER_EVENT,
        args: DragHoverArgs,
    }

    /// Drag&drop operation finished over the widget.
    pub fn drop {
        event: DROP_EVENT,
        args: DropArgs,
    }
}
