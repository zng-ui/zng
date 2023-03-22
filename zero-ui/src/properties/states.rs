//! Widget state properties, [`is_hovered`](fn@is_hovered), [`is_pressed`](fn@is_pressed) and more.

use std::time::Duration;

use crate::core::{
    mouse::*,
    timer::{DeadlineVar, TIMERS},
};
use crate::prelude::new_property::*;

/// If the mouse pointer is over the widget or a descendant and the widget is [`DISABLED`].
///
/// [`DISABLED`]: Interactivity::DISABLED
#[property(CONTEXT)]
pub fn is_hovered_disabled(child: impl UiNode, state: impl IntoVar<bool>) -> impl UiNode {
    event_is_state(child, state, false, MOUSE_HOVERED_EVENT, |args| {
        if args.is_mouse_enter_disabled() {
            Some(true)
        } else if args.is_mouse_leave_disabled() {
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
/// [`is_hovered_disabled`]: fn@is_hovered_disabled
#[property(CONTEXT)]
pub fn is_hovered(child: impl UiNode, state: impl IntoVar<bool>) -> impl UiNode {
    event_is_state(child, state, false, MOUSE_HOVERED_EVENT, |args| {
        if args.is_mouse_enter_enabled() {
            Some(true)
        } else if args.is_mouse_leave_enabled() {
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
#[property(CONTEXT)]
pub fn is_cap_hovered(child: impl UiNode, state: impl IntoVar<bool>) -> impl UiNode {
    event_is_state2(
        child,
        state,
        false,
        MOUSE_HOVERED_EVENT,
        false,
        |hovered_args| {
            if hovered_args.is_mouse_enter_enabled() {
                Some(true)
            } else if hovered_args.is_mouse_leave_enabled() {
                Some(false)
            } else {
                None
            }
        },
        MOUSE_CAPTURE_EVENT,
        false,
        |cap_args| {
            if cap_args.is_got(WIDGET.id()) {
                Some(true)
            } else if cap_args.is_lost(WIDGET.id()) {
                Some(false)
            } else {
                None
            }
        },
        |hovered, captured| Some(hovered || captured),
    )
}

/// If the pointer is pressed in the widget and the widget is [`ENABLED`].
///
/// This is `true` when the mouse primary button started pressing in the widget
/// and the pointer is over the widget and the primary button is still pressed and
/// the widget is fully [`ENABLED`].
///
/// This state property only considers mouse capture for repeat [click modes](ClickMode), if the pointer is captured by a widget
/// with [`ClickMode::Default`] and the pointer is not actually over the widget the state is `false`, use [`is_cap_pointer_pressed`] to
/// always include the captured state.
///
/// [`ENABLED`]: Interactivity::ENABLED
/// [`is_cap_pointer_pressed`]: fn@is_cap_pointer_pressed
#[property(CONTEXT)]
pub fn is_pointer_pressed(child: impl UiNode, state: impl IntoVar<bool>) -> impl UiNode {
    event_is_state3(
        child,
        state,
        false,
        MOUSE_HOVERED_EVENT,
        false,
        |hovered_args| {
            if hovered_args.is_mouse_enter_enabled() {
                Some(true)
            } else if hovered_args.is_mouse_leave_enabled() {
                Some(false)
            } else {
                None
            }
        },
        MOUSE_INPUT_EVENT,
        false,
        |input_args| {
            if input_args.is_primary() {
                match input_args.state {
                    ButtonState::Pressed => {
                        if input_args.capture_allows() {
                            return Some(input_args.is_enabled(WIDGET.id()));
                        }
                    }
                    ButtonState::Released => return Some(false),
                }
            } else {
            }
            None
        },
        MOUSE_CAPTURE_EVENT,
        false,
        |cap_args| {
            if cap_args.is_got(WIDGET.id()) {
                Some(true)
            } else if cap_args.is_lost(WIDGET.id()) {
                Some(false)
            } else {
                None
            }
        },
        |hovered, is_down, is_captured| match WINDOW.widget_tree().get(WIDGET.id()).unwrap().click_mode() {
            ClickMode::Default => Some(hovered && is_down),
            ClickMode::Repeat | ClickMode::Mixed => Some(is_down || is_captured),
        },
    )
}

/// If the pointer is pressed in the widget or was captured during press and the widget is [`ENABLED`].
///
/// [`ENABLED`]: Interactivity::ENABLED
#[property(CONTEXT)]
pub fn is_cap_pointer_pressed(child: impl UiNode, state: impl IntoVar<bool>) -> impl UiNode {
    event_is_state2(
        child,
        state,
        false,
        MOUSE_INPUT_EVENT,
        false,
        |input_args| {
            if input_args.is_primary() {
                match input_args.state {
                    ButtonState::Pressed => {
                        if input_args.capture_allows() {
                            return Some(input_args.is_enabled(WIDGET.id()));
                        }
                    }
                    ButtonState::Released => return Some(false),
                }
            }
            None
        },
        MOUSE_CAPTURE_EVENT,
        false,
        |cap_args| {
            if cap_args.is_got(WIDGET.id()) {
                Some(true)
            } else if cap_args.is_lost(WIDGET.id()) {
                Some(false)
            } else {
                None
            }
        },
        |is_down, is_captured| Some(is_down || is_captured),
    )
}

/// If the widget was clicked by shortcut and the [`shortcut_pressed_duration`] has not elapsed.
///
/// [`shortcut_pressed_duration`]: GESTURES::shortcut_pressed_duration
#[property(CONTEXT)]
pub fn is_shortcut_pressed(child: impl UiNode, state: impl IntoVar<bool>) -> impl UiNode {
    #[ui_node(struct IsShortcutPressedNode {
        child: impl UiNode,
        state: impl Var<bool>,
        shortcut_press: Option<DeadlineVar>,
    })]
    impl UiNode for IsShortcutPressedNode {
        fn init(&mut self) {
            let _ = self.state.set_ne(false);
            WIDGET.sub_event(&CLICK_EVENT);
            self.child.init();
        }
        fn deinit(&mut self) {
            let _ = self.state.set_ne(false);
            self.child.deinit();
        }
        fn event(&mut self, update: &mut EventUpdate) {
            if let Some(args) = CLICK_EVENT.on(update) {
                if args.shortcut().is_some() && args.is_enabled(WIDGET.id()) {
                    // if a shortcut click happened, we show pressed for the duration of `shortcut_pressed_duration`
                    // unless we where already doing that, then we just stop showing pressed, this causes
                    // a flickering effect when rapid clicks are happening.
                    if self.shortcut_press.take().is_none() {
                        let duration = GESTURES.shortcut_pressed_duration().get();
                        if duration != Duration::default() {
                            let dl = TIMERS.deadline(duration);
                            dl.subscribe(WIDGET.id()).perm();
                            self.shortcut_press = Some(dl);
                            let _ = self.state.set_ne(true);
                        }
                    } else {
                        let _ = self.state.set_ne(false);
                    }
                }
            }
            self.child.event(update);
        }
        fn update(&mut self, updates: &mut WidgetUpdates) {
            self.child.update(updates);

            if let Some(timer) = &self.shortcut_press {
                if timer.is_new() {
                    self.shortcut_press = None;
                    let _ = self.state.set_ne(false);
                }
            }
        }
    }
    IsShortcutPressedNode {
        child: child.cfg_boxed(),
        state: state.into_var(),
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
#[property(CONTEXT)]
pub fn is_pressed(child: impl UiNode, state: impl IntoVar<bool>) -> impl UiNode {
    let pointer_pressed = state_var();
    let child = is_pointer_pressed(child, pointer_pressed.clone());

    let shortcut_pressed = state_var();
    let child = is_shortcut_pressed(child, shortcut_pressed.clone());

    bind_is_state(child, merge_var!(pointer_pressed, shortcut_pressed, |&p, &s| p || s), state)
}

/// If [`is_cap_pointer_pressed`] or [`is_shortcut_pressed`].
///
/// [`is_cap_pointer_pressed`]: fn@is_cap_pointer_pressed
/// [`is_shortcut_pressed`]: fn@is_shortcut_pressed
#[property(CONTEXT)]
pub fn is_cap_pressed(child: impl UiNode, state: impl IntoVar<bool>) -> impl UiNode {
    let pointer_pressed = state_var();
    let child = is_cap_pointer_pressed(child, pointer_pressed.clone());

    let shortcut_pressed = state_var();
    let child = is_shortcut_pressed(child, shortcut_pressed.clone());

    bind_is_state(child, merge_var!(pointer_pressed, shortcut_pressed, |&p, &s| p || s), state)
}

#[doc(no_inline)]
pub use crate::core::widget_base::{is_collapsed, is_disabled, is_enabled, is_hidden, is_hit_testable, is_visible};
