//! Mouse events, [`on_mouse_move`](fn@on_mouse_move), [`on_mouse_enter`](fn@on_mouse_enter),
//! [`on_mouse_down`](fn@on_mouse_down) and more.
//!
//! There events are low level and directly tied to a mouse device.
//! Before using them review the [`gesture`](super::gesture) events, in particular the
//! [`on_click`](fn@super::gesture::on_click) event.

use zng_ext_input::mouse::{
    CTRL_SCROLL_VAR, MOUSE_CLICK_EVENT, MOUSE_HOVERED_EVENT, MOUSE_INPUT_EVENT, MOUSE_MOVE_EVENT, MOUSE_WHEEL_EVENT, MouseClickArgs,
    MouseHoverArgs, MouseInputArgs, MouseMoveArgs, MouseWheelArgs,
};
use zng_wgt::prelude::*;

event_property! {
    /// Mouse cursor moved over the widget and cursor capture allows it.
    #[property(EVENT)]
    pub fn on_mouse_move<on_pre_mouse_move>(child: impl IntoUiNode, handler: Handler<MouseMoveArgs>) -> UiNode {
        const PRE: bool;
        EventNodeBuilder::new(MOUSE_MOVE_EVENT)
            .filter(|| {
                let wgt = (WINDOW.id(), WIDGET.id());
                move |args| args.capture_allows(wgt)
            })
            .build::<PRE>(child, handler)
    }

    /// Mouse button pressed or released while the cursor is over the widget, the widget is enabled and no cursor
    /// capture blocks it.
    #[property(EVENT)]
    pub fn on_mouse_input<on_pre_mouse_input>(child: impl IntoUiNode, handler: Handler<MouseInputArgs>) -> UiNode {
        const PRE: bool;
        EventNodeBuilder::new(MOUSE_INPUT_EVENT)
            .filter(|| {
                let wgt = (WINDOW.id(), WIDGET.id());
                move |args| args.target.contains_enabled(wgt.1) && args.capture_allows(wgt)
            })
            .build::<PRE>(child, handler)
    }

    /// Mouse button pressed or release while the cursor is over the widget, the widget is disabled and no cursor
    /// capture blocks it.
    #[property(EVENT)]
    pub fn on_disabled_mouse_input<on_pre_disabled_mouse_input>(child: impl IntoUiNode, handler: Handler<MouseInputArgs>) -> UiNode {
        const PRE: bool;
        EventNodeBuilder::new(MOUSE_INPUT_EVENT)
            .filter(|| {
                let wgt = (WINDOW.id(), WIDGET.id());
                move |args| args.target.contains_disabled(wgt.1) && args.capture_allows(wgt)
            })
            .build::<PRE>(child, handler)
    }

    /// Mouse button pressed while the cursor is over the widget, the widget is enabled and cursor capture allows it.
    #[property(EVENT)]
    pub fn on_mouse_down<on_pre_mouse_down>(child: impl IntoUiNode, handler: Handler<MouseInputArgs>) -> UiNode {
        const PRE: bool;
        EventNodeBuilder::new(MOUSE_INPUT_EVENT)
            .filter(|| {
                let wgt = (WINDOW.id(), WIDGET.id());
                move |args| args.is_mouse_down() && args.target.contains_enabled(wgt.1) && args.capture_allows(wgt)
            })
            .build::<PRE>(child, handler)
    }

    /// Mouse button released while the cursor if over the widget, the widget is enabled and cursor capture allows it.
    #[property(EVENT)]
    pub fn on_mouse_up<on_pre_mouse_up>(child: impl IntoUiNode, handler: Handler<MouseInputArgs>) -> UiNode {
        const PRE: bool;
        EventNodeBuilder::new(MOUSE_INPUT_EVENT)
            .filter(|| {
                let wgt = (WINDOW.id(), WIDGET.id());
                move |args| args.is_mouse_up() && args.target.contains_enabled(wgt.1) && args.capture_allows(wgt)
            })
            .build::<PRE>(child, handler)
    }

    /// Mouse clicked on the widget with any button and including repeat clicks and it is enabled.
    #[property(EVENT)]
    pub fn on_mouse_any_click<on_pre_mouse_any_click>(child: impl IntoUiNode, handler: Handler<MouseClickArgs>) -> UiNode {
        const PRE: bool;
        EventNodeBuilder::new(MOUSE_CLICK_EVENT)
            .filter(|| {
                let id = WIDGET.id();
                move |args| args.target.contains_enabled(id)
            })
            .build::<PRE>(child, handler)
    }

    /// Mouse clicked on the disabled widget with any button, including repeat clicks.
    #[property(EVENT)]
    pub fn on_disabled_mouse_any_click<on_pre_disabled_mouse_any_click>(
        child: impl IntoUiNode,
        handler: Handler<MouseClickArgs>,
    ) -> UiNode {
        const PRE: bool;
        EventNodeBuilder::new(MOUSE_CLICK_EVENT)
            .filter(|| {
                let id = WIDGET.id();
                move |args| args.target.contains_disabled(id)
            })
            .build::<PRE>(child, handler)
    }

    /// Mouse clicked on the widget with any button but excluding repeat clicks and it is enabled.
    #[property(EVENT)]
    pub fn on_mouse_any_single_click<on_pre_mouse_any_single_click>(
        child: impl IntoUiNode,
        handler: Handler<MouseClickArgs>,
    ) -> UiNode {
        const PRE: bool;
        EventNodeBuilder::new(MOUSE_CLICK_EVENT)
            .filter(|| {
                let id = WIDGET.id();
                move |args| args.is_single() && args.target.contains_enabled(id)
            })
            .build::<PRE>(child, handler)
    }

    /// Mouse double clicked on the widget with any button and it is enabled.
    #[property(EVENT)]
    pub fn on_mouse_any_double_click<on_pre_mouse_any_double_click>(
        child: impl IntoUiNode,
        handler: Handler<MouseClickArgs>,
    ) -> UiNode {
        const PRE: bool;
        EventNodeBuilder::new(MOUSE_CLICK_EVENT)
            .filter(|| {
                let id = WIDGET.id();
                move |args| args.is_double() && args.target.contains_enabled(id)
            })
            .build::<PRE>(child, handler)
    }

    /// Mouse triple clicked on the widget with any button and it is enabled.
    #[property(EVENT)]
    pub fn on_mouse_any_triple_click<on_pre_mouse_any_triple_click>(
        child: impl IntoUiNode,
        handler: Handler<MouseClickArgs>,
    ) -> UiNode {
        const PRE: bool;
        EventNodeBuilder::new(MOUSE_CLICK_EVENT)
            .filter(|| {
                let id = WIDGET.id();
                move |args| args.is_triple() && args.target.contains_enabled(id)
            })
            .build::<PRE>(child, handler)
    }

    /// Mouse clicked on the widget with the primary button including repeat clicks and it is enabled.
    #[property(EVENT)]
    pub fn on_mouse_click<on_pre_mouse_click>(child: impl IntoUiNode, handler: Handler<MouseClickArgs>) -> UiNode {
        const PRE: bool;
        EventNodeBuilder::new(MOUSE_CLICK_EVENT)
            .filter(|| {
                let id = WIDGET.id();
                move |args| args.is_primary() && args.target.contains_enabled(id)
            })
            .build::<PRE>(child, handler)
    }

    /// Mouse clicked on the disabled widget with the primary button, including repeat clicks.
    #[property(EVENT)]
    pub fn on_disabled_mouse_click<on_pre_disabled_mouse_click>(child: impl IntoUiNode, handler: Handler<MouseClickArgs>) -> UiNode {
        const PRE: bool;
        EventNodeBuilder::new(MOUSE_CLICK_EVENT)
            .filter(|| {
                let id = WIDGET.id();
                move |args| args.is_primary() && args.target.contains_disabled(id)
            })
            .build::<PRE>(child, handler)
    }

    /// Mouse clicked on the widget with the primary button excluding repeat clicks and it is enabled.
    #[property(EVENT)]
    pub fn on_mouse_single_click<on_pre_mouse_single_click>(child: impl IntoUiNode, handler: Handler<MouseClickArgs>) -> UiNode {
        const PRE: bool;
        EventNodeBuilder::new(MOUSE_CLICK_EVENT)
            .filter(|| {
                let id = WIDGET.id();
                move |args| args.is_primary() && args.is_single() && args.target.contains_enabled(id)
            })
            .build::<PRE>(child, handler)
    }

    /// Mouse double clicked on the widget with the primary button and it is enabled.
    #[property(EVENT)]
    pub fn on_mouse_double_click<on_pre_mouse_double_click>(child: impl IntoUiNode, handler: Handler<MouseClickArgs>) -> UiNode {
        const PRE: bool;
        EventNodeBuilder::new(MOUSE_CLICK_EVENT)
            .filter(|| {
                let id = WIDGET.id();
                move |args| args.is_primary() && args.is_double() && args.target.contains_enabled(id)
            })
            .build::<PRE>(child, handler)
    }

    /// Mouse triple clicked on the widget with the primary button and it is enabled.
    #[property(EVENT)]
    pub fn on_mouse_triple_click<on_pre_mouse_triple_click>(child: impl IntoUiNode, handler: Handler<MouseClickArgs>) -> UiNode {
        const PRE: bool;
        EventNodeBuilder::new(MOUSE_CLICK_EVENT)
            .filter(|| {
                let id = WIDGET.id();
                move |args| args.is_primary() && args.is_triple() && args.target.contains_enabled(id)
            })
            .build::<PRE>(child, handler)
    }

    /// Mouse is now over the widget or a descendant widget, the widget is enabled and cursor capture allows it.
    #[property(EVENT)]
    pub fn on_mouse_enter<on_pre_mouse_enter>(child: impl IntoUiNode, handler: Handler<MouseHoverArgs>) -> UiNode {
        const PRE: bool;
        EventNodeBuilder::new(MOUSE_HOVERED_EVENT)
            .filter(|| {
                let wgt = (WINDOW.id(), WIDGET.id());
                move |args| args.is_mouse_enter_enabled(wgt)
            })
            .build::<PRE>(child, handler)
    }

    /// Mouse is no longer over the widget or any descendant widget, the widget is enabled and cursor capture allows it.
    #[property(EVENT)]
    pub fn on_mouse_leave<on_pre_mouse_leave>(child: impl IntoUiNode, handler: Handler<MouseHoverArgs>) -> UiNode {
        const PRE: bool;
        EventNodeBuilder::new(MOUSE_HOVERED_EVENT)
            .filter(|| {
                let wgt = (WINDOW.id(), WIDGET.id());
                move |args| args.is_mouse_leave_enabled(wgt)
            })
            .build::<PRE>(child, handler)
    }

    /// Mouse entered or left the widget and descendant widgets area, the widget is enabled and cursor capture allows it.
    ///
    /// You can use the [`is_mouse_enter`] and [`is_mouse_leave`] methods to determinate the state change.
    ///
    /// [`is_mouse_enter`]: MouseHoverArgs::is_mouse_enter
    /// [`is_mouse_leave`]: MouseHoverArgs::is_mouse_leave
    #[property(EVENT)]
    pub fn on_mouse_hovered<on_pre_mouse_hovered>(child: impl IntoUiNode, handler: Handler<MouseHoverArgs>) -> UiNode {
        const PRE: bool;
        EventNodeBuilder::new(MOUSE_HOVERED_EVENT)
            .filter(|| {
                let wgt = (WINDOW.id(), WIDGET.id());
                move |args| args.is_enabled(wgt.1) && args.capture_allows(wgt)
            })
            .build::<PRE>(child, handler)
    }

    /// Mouse entered or left the widget and descendant widgets area, the widget is disabled and cursor capture allows it.
    #[property(EVENT)]
    pub fn on_disabled_mouse_hovered<on_pre_disabled_mouse_hovered>(
        child: impl IntoUiNode,
        handler: Handler<MouseHoverArgs>,
    ) -> UiNode {
        const PRE: bool;
        EventNodeBuilder::new(MOUSE_HOVERED_EVENT)
            .filter(|| {
                let wgt = (WINDOW.id(), WIDGET.id());
                move |args| args.is_disabled(wgt.1) && args.capture_allows(wgt)
            })
            .build::<PRE>(child, handler)
    }

    /// Mouse wheel scrolled while pointer is hovering widget and it is enabled.
    #[property(EVENT)]
    pub fn on_mouse_wheel<on_pre_mouse_wheel>(child: impl IntoUiNode, handler: Handler<MouseWheelArgs>) -> UiNode {
        const PRE: bool;
        EventNodeBuilder::new(MOUSE_WHEEL_EVENT)
            .filter(|| {
                let id = WIDGET.id();
                move |args| args.target.contains_enabled(id)
            })
            .build::<PRE>(child, handler)
    }

    /// Mouse wheel scrolled while pointer is hovering widget and it is disabled.
    #[property(EVENT)]
    pub fn on_disabled_mouse_wheel<on_pre_disabled_mouse_wheel>(child: impl IntoUiNode, handler: Handler<MouseWheelArgs>) -> UiNode {
        const PRE: bool;
        EventNodeBuilder::new(MOUSE_WHEEL_EVENT)
            .filter(|| {
                let id = WIDGET.id();
                move |args| args.target.contains_enabled(id)
            })
            .build::<PRE>(child, handler)
    }

    /// Mouse wheel scrolled while pointer is hovering the widget and the pressed keyboard modifiers allow a scroll operation and
    /// the widget is enabled.
    #[property(EVENT)]
    pub fn on_mouse_scroll<on_pre_mouse_scroll>(child: impl IntoUiNode, handler: Handler<MouseWheelArgs>) -> UiNode {
        const PRE: bool;
        EventNodeBuilder::new(MOUSE_WHEEL_EVENT)
            .filter(|| {
                let id = WIDGET.id();
                move |args| args.is_scroll() && args.target.contains_enabled(id)
            })
            .build::<PRE>(child, handler)
    }

    /// Mouse wheel scrolled while pointer is hovering the widget and the pressed keyboard modifiers allow a zoom operation and
    /// the widget is enabled.
    #[property(EVENT)]
    pub fn on_mouse_zoom<on_pre_mouse_zoom>(child: impl IntoUiNode, handler: Handler<MouseWheelArgs>) -> UiNode {
        const PRE: bool;
        EventNodeBuilder::new(MOUSE_WHEEL_EVENT)
            .filter(|| {
                let id = WIDGET.id();
                move |args| args.is_zoom() && args.target.contains_enabled(id)
            })
            .build::<PRE>(child, handler)
    }
}

/// Defines if [`MouseWheelArgs`] gesture [`is_scroll`] when `CTRL` is pressed and [`is_zoom`] when no modifier is pressed.
///
/// This property sets the [`CTRL_SCROLL_VAR`].
///
/// [`is_scroll`]: MouseWheelArgs::is_scroll
/// [`is_zoom`]: MouseWheelArgs::is_zoom
#[property(CONTEXT, default(CTRL_SCROLL_VAR))]
pub fn ctrl_scroll(child: impl IntoUiNode, ctrl_scroll: impl IntoVar<bool>) -> UiNode {
    with_context_var(child, CTRL_SCROLL_VAR, ctrl_scroll)
}
