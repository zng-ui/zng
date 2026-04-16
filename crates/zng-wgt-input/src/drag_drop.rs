#![cfg(feature = "drag_drop")]

//! Drag&drop properties, event properties.

use zng_ext_input::drag_drop::{
    DRAG_END_EVENT, DRAG_HOVERED_EVENT, DRAG_START_EVENT, DROP_EVENT, DragEndArgs, DragHoveredArgs, DragStartArgs, DropArgs,
    WidgetInfoBuilderDragDropExt as _,
};
use zng_wgt::{node::bind_state_init, prelude::*};

/// If this widget can be dragged in a drag&drop operation.
///
/// When this is `true` the widget can be dragged and dropped within the same app or it can handle [`on_drag_start`] and
/// use the [`DRAG_DROP.drag`] service to set a system wide drag data.
///
/// [`on_drag_start`]: fn@on_drag_start
/// [`DRAG_DROP.drag`]: zng_ext_input::drag_drop::DRAG_DROP::drag
#[property(CONTEXT, default(false))]
pub fn draggable(child: impl IntoUiNode, input: impl IntoVar<bool>) -> UiNode {
    let input = input.into_var();
    match_node(child, move |_c, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_info(&input);
        }
        UiNodeOp::Info { info } if input.get() => {
            info.draggable();
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
    #[property(EVENT)]
    pub fn on_drag_start<on_pre_drag_start>(child: impl IntoUiNode, handler: Handler<DragStartArgs>) -> UiNode {
        const PRE: bool;
        EventNodeBuilder::new(DRAG_START_EVENT).build::<PRE>(child, handler)
    }

    /// Draggable widget stopped dragging.
    ///
    /// This event is always paired with [`on_drag_start`] first.
    ///
    /// [`on_drag_start`]: fn@on_drag_start
    #[property(EVENT)]
    pub fn on_drag_end<on_pre_drag_end>(child: impl IntoUiNode, handler: Handler<DragEndArgs>) -> UiNode {
        const PRE: bool;
        EventNodeBuilder::new(DRAG_END_EVENT).build::<PRE>(child, handler)
    }

    /// Dragging cursor entered or exited the widget area and the widget is enabled.
    #[property(EVENT)]
    pub fn on_drag_hovered<on_pre_drag_hovered>(child: impl IntoUiNode, handler: Handler<DragHoveredArgs>) -> UiNode {
        const PRE: bool;
        EventNodeBuilder::new(DRAG_HOVERED_EVENT)
            .filter(|| {
                let id = WIDGET.id();
                move |args| args.is_drag_enter_enabled(id) || args.is_drag_leave_enabled(id)
            })
            .build::<PRE>(child, handler)
    }
    /// Dragging cursor entered the widget area and the widget is enabled.
    #[property(EVENT)]
    pub fn on_drag_enter<on_pre_drag_enter>(child: impl IntoUiNode, handler: Handler<DragHoveredArgs>) -> UiNode {
        const PRE: bool;
        EventNodeBuilder::new(DRAG_HOVERED_EVENT)
            .filter(|| {
                let id = WIDGET.id();
                move |args| args.is_drag_enter_enabled(id)
            })
            .build::<PRE>(child, handler)
    }
    /// Dragging cursor exited the widget area and the widget is enabled.
    #[property(EVENT)]
    pub fn on_drag_leave<on_pre_drag_leave>(child: impl IntoUiNode, handler: Handler<DragHoveredArgs>) -> UiNode {
        const PRE: bool;
        EventNodeBuilder::new(DRAG_HOVERED_EVENT)
            .filter(|| {
                let id = WIDGET.id();
                move |args| args.is_drag_leave_enabled(id)
            })
            .build::<PRE>(child, handler)
    }

    /// Dragging cursor dropped data in the widget area and the widget is enabled.
    #[property(EVENT)]
    pub fn on_drop<on_pre_drop>(child: impl IntoUiNode, handler: Handler<DropArgs>) -> UiNode {
        const PRE: bool;
        EventNodeBuilder::new(DROP_EVENT)
            .filter(|| {
                let id = WIDGET.id();
                move |args| args.target.contains_enabled(id)
            })
            .build::<PRE>(child, handler)
    }
}

/// If the dragging cursor is over the widget or a descendant and the widget is enabled.
///
/// The value is always `false` when the widget is not [`ENABLED`], use [`is_drag_hovered_disabled`] to implement *disabled hovered* visuals.
///
/// [`ENABLED`]: Interactivity::ENABLED
/// [`is_drag_hovered_disabled`]: fn@is_drag_hovered_disabled
#[property(EVENT)]
pub fn is_drag_hovered(child: impl IntoUiNode, state: impl IntoVar<bool>) -> UiNode {
    bind_state_init(child, state, |s| {
        let id = WIDGET.id();
        DRAG_HOVERED_EVENT.var_bind(s, move |args| {
            if args.is_drag_enter_enabled(id) {
                Some(true)
            } else if args.is_drag_leave_enabled(id) {
                Some(false)
            } else {
                None
            }
        })
    })
}

/// If the dragging cursor is over the widget or a descendant and the widget is disabled.
#[property(EVENT)]
pub fn is_drag_hovered_disabled(child: impl IntoUiNode, state: impl IntoVar<bool>) -> UiNode {
    bind_state_init(child, state, |s| {
        let id = WIDGET.id();
        DRAG_HOVERED_EVENT.var_bind(s, move |args| {
            if args.is_drag_enter_disabled(id) {
                Some(true)
            } else if args.is_drag_leave_disabled(id) {
                Some(false)
            } else {
                None
            }
        })
    })
}

/// If the draggable widget is dragging.
#[property(EVENT)]
pub fn is_dragging(child: impl IntoUiNode, state: impl IntoVar<bool>) -> UiNode {
    bind_state_init(child, state, |s| {
        let id = WIDGET.id();
        let handle = DRAG_START_EVENT.var_bind(s, move |args| if args.target.contains(id) { Some(true) } else { None });
        DRAG_END_EVENT.var_bind(s, move |args| {
            let _hold = &handle;
            if args.target.contains(id) { Some(false) } else { None }
        })
    })
}
