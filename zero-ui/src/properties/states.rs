//! Widget state properties, [`is_hovered`](fn@is_hovered), [`is_pressed`](fn@is_pressed) and more.

use std::time::Duration;

use crate::core::{mouse::*, timer::DeadlineVar};
use crate::prelude::new_property::*;

/// If the mouse pointer is over the widget or a descendant and the widget is [`DISABLED`].
///
/// [`DISABLED`]: Interactivity::DISABLED
#[property(context)]
pub fn is_hovered_disabled(child: impl UiNode, state: StateVar) -> impl UiNode {
    event_state(child, state, false, MOUSE_HOVERED_EVENT, |ctx, args| {
        if args.is_mouse_enter_disabled(ctx.path) {
            Some(true)
        } else if args.is_mouse_leave_disabled(ctx.path) {
            Some(false)
        } else {
            None
        }
    })
}

/// If the mouse pointer is over the widget or a descendant and the widget is [`ENABLED`].
///
/// This state property does not consider mouse capture, if the pointer is captured by the widget
/// but is not actually over the widget this is `false`, use [`is_cap_hovered`] to include the captured state.
///
/// The value is always `false` when the widget is not fully [`ENABLED`], use [`is_hovered_disabled`] to implement *disabled hovered* visuals.
///
/// [`is_cap_hovered`]: fn@is_cap_hovered
/// [`ENABLED`]: Interactivity::ENABLED
#[property(context)]
pub fn is_hovered(child: impl UiNode, state: StateVar) -> impl UiNode {
    event_state(child, state, false, MOUSE_HOVERED_EVENT, |ctx, args| {
        if args.is_mouse_enter_enabled(ctx.path) {
            Some(true)
        } else if args.is_mouse_leave_enabled(ctx.path) {
            Some(false)
        } else {
            None
        }
    })
}

/// If the mouse pointer is over the widget, or is over a widget descendant, or is captured by the widget.
///
/// The value is always `false` when the widget is not fully [`ENABLED`].
///
/// [`ENABLED`]: Interactivity::ENABLED
#[property(context)]
pub fn is_cap_hovered(child: impl UiNode, state: StateVar) -> impl UiNode {
    event_state2(
        child,
        state,
        false,
        MOUSE_HOVERED_EVENT,
        false,
        |ctx, hovered_args| {
            if hovered_args.is_mouse_enter_enabled(ctx.path) {
                Some(true)
            } else if hovered_args.is_mouse_leave_enabled(ctx.path) {
                Some(false)
            } else {
                None
            }
        },
        MOUSE_CAPTURE_EVENT,
        false,
        |ctx, cap_args| {
            if cap_args.is_got(ctx.path.widget_id()) {
                Some(true)
            } else if cap_args.is_lost(ctx.path.widget_id()) {
                Some(false)
            } else {
                None
            }
        },
        |_, hovered, captured| Some(hovered || captured),
    )
}

/// If the pointer is pressed in the widget and the widget is [`ENABLED`].
///
/// This is `true` when the mouse primary button started pressing in the widget
/// and the pointer is over the widget and the primary button is still pressed and
/// the widget is fully [`ENABLED`].
///
/// This state property does not consider mouse capture, if the pointer is captured by the widget
/// but is not actually over the widget this is `false`, use [`is_cap_pointer_pressed`] to
/// include the captured state.
///
/// [`ENABLED`]: Interactivity::ENABLED
#[property(context)]
pub fn is_pointer_pressed(child: impl UiNode, state: StateVar) -> impl UiNode {
    event_state2(
        child,
        state,
        false,
        MOUSE_HOVERED_EVENT,
        false,
        |ctx, hovered_args| {
            if hovered_args.is_mouse_enter(ctx.path) {
                Some(true)
            } else if hovered_args.is_mouse_leave(ctx.path) {
                Some(false)
            } else {
                None
            }
        },
        MOUSE_INPUT_EVENT,
        false,
        |ctx, input_args| {
            if input_args.is_primary() {
                match input_args.state {
                    ButtonState::Pressed => {
                        if input_args.capture_allows(ctx.path) {
                            return Some(true);
                        }
                    }
                    ButtonState::Released => return Some(false),
                }
            }
            None
        },
        |_, hovered, is_down| Some(hovered && is_down),
    )
}

/// If the pointer is pressed in the widget or was captured during press and the widget is [`ENABLED`].
///
/// [`ENABLED`]: Interactivity::ENABLED
#[property(context)]
pub fn is_cap_pointer_pressed(child: impl UiNode, state: StateVar) -> impl UiNode {
    event_state2(
        child,
        state,
        false,
        MOUSE_INPUT_EVENT,
        false,
        |ctx, input_args| {
            if input_args.is_primary() {
                match input_args.state {
                    ButtonState::Pressed => {
                        if input_args.capture_allows(ctx.path) {
                            return Some(true);
                        }
                    }
                    ButtonState::Released => return Some(false),
                }
            }
            None
        },
        MOUSE_CAPTURE_EVENT,
        false,
        |ctx, cap_args| {
            if cap_args.is_got(ctx.path.widget_id()) {
                Some(true)
            } else if cap_args.is_lost(ctx.path.widget_id()) {
                Some(false)
            } else {
                None
            }
        },
        |_, is_down, is_captured| Some(is_down || is_captured),
    )
}

/// If the widget was clicked by shortcut and the [`shortcut_pressed_duration`] has not elapsed.
///
/// [`shortcut_pressed_duration`]: Gestures::shortcut_pressed_duration
#[property(context)]
pub fn is_shortcut_pressed(child: impl UiNode, state: StateVar) -> impl UiNode {
    #[impl_ui_node(struct IsShortcutPressedNode {
        child: impl UiNode,
        state: StateVar,
        shortcut_press: Option<DeadlineVar>,
    })]
    impl UiNode for IsShortcutPressedNode {
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.state.set_ne(ctx, false).unwrap();
            ctx.sub_event(&CLICK_EVENT);
            self.child.init(ctx);
        }
        fn deinit(&mut self, ctx: &mut WidgetContext) {
            self.state.set_ne(ctx, false).unwrap();
            self.child.deinit(ctx);
        }
        fn event(&mut self, ctx: &mut WidgetContext, update: &mut EventUpdate) {
            if let Some(args) = CLICK_EVENT.on(update) {
                if args.shortcut().is_some() {
                    // if a shortcut click happened, we show pressed for the duration of `shortcut_pressed_duration`
                    // unless we where already doing that, then we just stop showing pressed, this causes
                    // a flickering effect when rapid clicks are happening.
                    if self.shortcut_press.take().is_none() {
                        let duration = Gestures::req(ctx.services).shortcut_pressed_duration;
                        if duration != Duration::default() {
                            let dl = ctx.timers.deadline(duration);
                            dl.subscribe(ctx.path.widget_id()).perm();
                            self.shortcut_press = Some(dl);
                            self.state.set_ne(ctx, true).unwrap();
                        }
                    } else {
                        self.state.set_ne(ctx, false).unwrap();
                        ctx.updates.subscriptions();
                    }
                }
            }
            self.child.event(ctx, update);
        }
        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            self.child.update(ctx, updates);

            if let Some(timer) = &self.shortcut_press {
                if timer.is_new(ctx) {
                    self.shortcut_press = None;
                    self.state.set_ne(ctx.vars, false).unwrap();
                }
            }
        }
    }
    IsShortcutPressedNode {
        child: child.cfg_boxed(),
        state,
        shortcut_press: None,
    }
    .cfg_boxed()
}

/// If [`is_pointer_pressed`] or [`is_shortcut_pressed`].
///
/// Note that [`is_pointer_pressed`] does not consider mouse capture, use [`is_cap_pressed`] to
/// include the captured state.
///
/// [`shortcut_pressed_duration`]: Gestures::shortcut_pressed_duration
/// [`is_pointer_pressed`]: fn@is_pointer_pressed
/// [`is_shortcut_pressed`]: fn@is_shortcut_pressed
/// [`is_cap_pressed`]: fn@is_cap_pressed
#[property(context)]
pub fn is_pressed(child: impl UiNode, state: StateVar) -> impl UiNode {
    let pointer_pressed = state_var();
    let child = is_pointer_pressed(child, pointer_pressed.clone());

    let shortcut_pressed = state_var();
    let child = is_shortcut_pressed(child, shortcut_pressed.clone());

    bind_state(child, merge_var!(pointer_pressed, shortcut_pressed, |&p, &s| p || s), state)
}

/// If [`is_cap_pointer_pressed`] or [`is_shortcut_pressed`].
///
/// [`is_cap_pointer_pressed`]: fn@is_cap_pointer_pressed
/// [`is_shortcut_pressed`]: fn@is_shortcut_pressed
#[property(context)]
pub fn is_cap_pressed(child: impl UiNode, state: StateVar) -> impl UiNode {
    let pointer_pressed = state_var();
    let child = is_cap_pointer_pressed(child, pointer_pressed.clone());

    let shortcut_pressed = state_var();
    let child = is_shortcut_pressed(child, shortcut_pressed.clone());

    bind_state(child, merge_var!(pointer_pressed, shortcut_pressed, |&p, &s| p || s), state)
}

#[doc(no_inline)]
pub use crate::core::widget_base::{is_collapsed, is_disabled, is_enabled, is_hidden, is_hit_testable, is_visible};
