//! Touch events, [`on_touch_move`](fn@on_touch_move), [`on_touch_tap`](fn@on_touch_tap),
//! [`on_touch_start`](fn@on_touch_start) and more.
//!
//! There events are low level and directly tied to touch inputs.
//! Before using them review the [`gesture`](super::gesture) events, in particular the
//! [`on_click`](fn@super::gesture::on_click) event.

use zng_ext_input::touch::{
    TOUCH_INPUT_EVENT, TOUCH_LONG_PRESS_EVENT, TOUCH_MOVE_EVENT, TOUCH_TAP_EVENT, TOUCH_TRANSFORM_EVENT, TOUCHED_EVENT, TouchInputArgs,
    TouchLongPressArgs, TouchMoveArgs, TouchTapArgs, TouchTransformArgs, TouchedArgs,
};
use zng_wgt::prelude::*;

event_property! {
    /// Touch contact moved over the widget and cursor capture allows it.
    #[property(EVENT)]
    pub fn on_touch_move<on_pre_touch_move>(child: impl IntoUiNode, handler: Handler<TouchMoveArgs>) -> UiNode {
        const PRE: bool;
        EventNodeBuilder::new(TOUCH_MOVE_EVENT)
            .filter(|| {
                let wgt = (WINDOW.id(), WIDGET.id());
                move |args| args.capture_allows(wgt)
            })
            .build::<PRE>(child, handler)
    }

    /// Touch contact started or ended over the widget, it is enabled and cursor capture allows it.
    #[property(EVENT)]
    pub fn on_touch_input<on_pre_touch_input>(child: impl IntoUiNode, handler: Handler<TouchInputArgs>) -> UiNode {
        const PRE: bool;
        EventNodeBuilder::new(TOUCH_INPUT_EVENT)
            .filter(|| {
                let wgt = (WINDOW.id(), WIDGET.id());
                move |args| args.target.contains_enabled(wgt.1) && args.capture_allows(wgt)
            })
            .build::<PRE>(child, handler)
    }

    /// Touch contact started or ended over the widget, it is disabled and cursor capture allows it.
    #[property(EVENT)]
    pub fn on_disabled_touch_input<on_pre_disabled_touch_input>(child: impl IntoUiNode, handler: Handler<TouchInputArgs>) -> UiNode {
        const PRE: bool;
        EventNodeBuilder::new(TOUCH_INPUT_EVENT)
            .filter(|| {
                let wgt = (WINDOW.id(), WIDGET.id());
                move |args| args.target.contains_disabled(wgt.1) && args.capture_allows(wgt)
            })
            .build::<PRE>(child, handler)
    }

    /// Touch contact started over the widget, it is enabled and cursor capture allows it.
    #[property(EVENT)]
    pub fn on_touch_start<on_pre_touch_start>(child: impl IntoUiNode, handler: Handler<TouchInputArgs>) -> UiNode {
        const PRE: bool;
        EventNodeBuilder::new(TOUCH_INPUT_EVENT)
            .filter(|| {
                let wgt = (WINDOW.id(), WIDGET.id());
                move |args| args.is_touch_start() && args.target.contains_enabled(wgt.1) && args.capture_allows(wgt)
            })
            .build::<PRE>(child, handler)
    }

    /// Touch contact ended over the widget, it is enabled and cursor capture allows it.
    #[property(EVENT)]
    pub fn on_touch_end<on_pre_touch_end>(child: impl IntoUiNode, handler: Handler<TouchInputArgs>) -> UiNode {
        const PRE: bool;
        EventNodeBuilder::new(TOUCH_INPUT_EVENT)
            .filter(|| {
                let wgt = (WINDOW.id(), WIDGET.id());
                move |args| args.is_touch_end() && args.target.contains_enabled(wgt.1) && args.capture_allows(wgt)
            })
            .build::<PRE>(child, handler)
    }

    /// Touch contact canceled over the widget, it is enabled and cursor capture allows it.
    #[property(EVENT)]
    pub fn on_touch_cancel<on_pre_touch_cancel>(child: impl IntoUiNode, handler: Handler<TouchInputArgs>) -> UiNode {
        const PRE: bool;
        EventNodeBuilder::new(TOUCH_INPUT_EVENT)
            .filter(|| {
                let wgt = (WINDOW.id(), WIDGET.id());
                move |args| args.is_touch_cancel() && args.target.contains_enabled(wgt.1) && args.capture_allows(wgt)
            })
            .build::<PRE>(child, handler)
    }

    /// Touch tap on the widget and it is enabled.
    #[property(EVENT)]
    pub fn on_touch_tap<on_pre_touch_tap>(child: impl IntoUiNode, handler: Handler<TouchTapArgs>) -> UiNode {
        const PRE: bool;
        EventNodeBuilder::new(TOUCH_TAP_EVENT)
            .filter(|| {
                let id = WIDGET.id();
                move |args| args.target.contains_enabled(id)
            })
            .build::<PRE>(child, handler)
    }

    /// Touch tap on the widget and it is disabled.
    #[property(EVENT)]
    pub fn on_disabled_touch_tap<on_pre_disabled_touch_tap>(child: impl IntoUiNode, handler: Handler<TouchTapArgs>) -> UiNode {
        const PRE: bool;
        EventNodeBuilder::new(TOUCH_TAP_EVENT)
            .filter(|| {
                let id = WIDGET.id();
                move |args| args.target.contains_disabled(id)
            })
            .build::<PRE>(child, handler)
    }

    /// Touch contact is now over the widget or a descendant and it is enabled.
    #[property(EVENT)]
    pub fn on_touch_enter<on_pre_touch_enter>(child: impl IntoUiNode, handler: Handler<TouchedArgs>) -> UiNode {
        const PRE: bool;
        EventNodeBuilder::new(TOUCHED_EVENT)
            .filter(|| {
                let wgt = (WINDOW.id(), WIDGET.id());
                move |args| args.is_touch_enter_enabled(wgt)
            })
            .build::<PRE>(child, handler)
    }

    /// Touch contact is no longer over the widget or any descendant and it is enabled.
    #[property(EVENT)]
    pub fn on_touch_leave<on_pre_touch_leave>(child: impl IntoUiNode, handler: Handler<TouchedArgs>) -> UiNode {
        const PRE: bool;
        EventNodeBuilder::new(TOUCHED_EVENT)
            .filter(|| {
                let wgt = (WINDOW.id(), WIDGET.id());
                move |args| args.is_touch_leave_enabled(wgt)
            })
            .build::<PRE>(child, handler)
    }

    /// Touch contact entered or left the widget and descendants area and it is enabled.
    ///
    /// You can use the [`is_touch_enter`] and [`is_touch_leave`] methods to determinate the state change.
    ///
    /// [`is_touch_enter`]: TouchedArgs::is_touch_enter
    /// [`is_touch_leave`]: TouchedArgs::is_touch_leave
    #[property(EVENT)]
    pub fn on_touched<on_pre_touched>(child: impl IntoUiNode, handler: Handler<TouchedArgs>) -> UiNode {
        const PRE: bool;
        EventNodeBuilder::new(TOUCHED_EVENT)
            .filter(|| {
                let id = WIDGET.id();
                move |args| args.is_enabled(id)
            })
            .build::<PRE>(child, handler)
    }

    /// Touch gesture to translate, scale or rotate happened over this widget.
    #[property(EVENT)]
    pub fn on_touch_transform<on_pre_touch_transform>(child: impl IntoUiNode, handler: Handler<TouchTransformArgs>) -> UiNode {
        const PRE: bool;
        EventNodeBuilder::new(TOUCH_TRANSFORM_EVENT)
            .filter(|| {
                let id = WIDGET.id();
                move |args| args.target.contains_enabled(id)
            })
            .build::<PRE>(child, handler)
    }

    /// Single touch contact was made and held in place for a duration of time (default 500ms) on
    /// the widget and the widget is enabled.
    #[property(EVENT)]
    pub fn on_touch_long_press<on_pre_touch_long_press>(child: impl IntoUiNode, handler: Handler<TouchLongPressArgs>) -> UiNode {
        const PRE: bool;
        EventNodeBuilder::new(TOUCH_LONG_PRESS_EVENT)
            .filter(|| {
                let id = WIDGET.id();
                move |args| args.target.contains_enabled(id)
            })
            .build::<PRE>(child, handler)
    }

    /// Single touch contact was made and held in place for a duration of time (default 500ms) on
    /// the widget and the widget is disabled.
    #[property(EVENT)]
    pub fn on_disabled_touch_long_press<on_pre_disabled_touch_long_press>(
        child: impl IntoUiNode,
        handler: Handler<TouchLongPressArgs>,
    ) -> UiNode {
        const PRE: bool;
        EventNodeBuilder::new(TOUCH_LONG_PRESS_EVENT)
            .filter(|| {
                let id = WIDGET.id();
                move |args| args.target.contains_disabled(id)
            })
            .build::<PRE>(child, handler)
    }
}
