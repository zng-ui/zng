use std::{collections::HashSet, time::Duration};

use zng_app::timer::TIMERS;
use zng_ext_input::{
    gesture::{CLICK_EVENT, GESTURES},
    mouse::{ClickMode, MOUSE_HOVERED_EVENT, MOUSE_INPUT_EVENT, WidgetInfoMouseExt as _},
    pointer_capture::POINTER_CAPTURE_EVENT,
    touch::TOUCHED_EVENT,
};
use zng_view_api::{mouse::ButtonState, touch::TouchPhase};
use zng_wgt::prelude::*;

/// If the mouse pointer is over the widget or a descendant and the widget is disabled.
#[property(EVENT)]
pub fn is_hovered_disabled(child: impl UiNode, state: impl IntoVar<bool>) -> impl UiNode {
    event_state(child, state, false, MOUSE_HOVERED_EVENT, |args| {
        if args.is_mouse_enter_disabled() {
            Some(true)
        } else if args.is_mouse_leave_disabled() {
            Some(false)
        } else {
            None
        }
    })
}

/// If the mouse pointer is over the widget or a descendant and the widget is enabled.
///
/// This state property does not consider pointer capture, if the pointer is captured by the widget
/// but is not actually over the widget this is `false`, use [`is_cap_hovered`] to include the captured state.
///
/// The value is always `false` when the widget is not [`ENABLED`], use [`is_hovered_disabled`] to implement *disabled hovered* visuals.
///
/// [`is_cap_hovered`]: fn@is_cap_hovered
/// [`ENABLED`]: Interactivity::ENABLED
/// [`is_hovered_disabled`]: fn@is_hovered_disabled
#[property(EVENT)]
pub fn is_hovered(child: impl UiNode, state: impl IntoVar<bool>) -> impl UiNode {
    event_state(child, state, false, MOUSE_HOVERED_EVENT, |args| {
        if args.is_mouse_enter_enabled() {
            Some(true)
        } else if args.is_mouse_leave_enabled() {
            Some(false)
        } else {
            None
        }
    })
}

/// If the mouse pointer is over the widget, or a descendant, or is captured by the it.
///
/// The value is always `false` when the widget is not [`ENABLED`].
///
/// [`ENABLED`]: Interactivity::ENABLED
#[property(EVENT)]
pub fn is_cap_hovered(child: impl UiNode, state: impl IntoVar<bool>) -> impl UiNode {
    event_state2(
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
        POINTER_CAPTURE_EVENT,
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

/// If the mouse pointer is pressed in the widget and it is enabled.
///
/// This is `true` when the mouse primary button started pressing in the widget
/// and the pointer is over the widget and the primary button is still pressed and
/// the widget is fully [`ENABLED`].
///
/// This state property only considers pointer capture for repeat [click modes](ClickMode), if the pointer is captured by a widget
/// with [`ClickMode::repeat`] `false` and the pointer is not actually over the widget the state is `false`,
/// use [`is_cap_mouse_pressed`] to always include the captured state.
///
/// [`ENABLED`]: Interactivity::ENABLED
/// [`is_cap_mouse_pressed`]: fn@is_cap_mouse_pressed
/// [`ClickMode::repeat`]: zng_ext_input::mouse::ClickMode::repeat
#[property(EVENT)]
pub fn is_mouse_pressed(child: impl UiNode, state: impl IntoVar<bool>) -> impl UiNode {
    event_state3(
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
                            return Some(input_args.target.contains_enabled(WIDGET.id()));
                        }
                    }
                    ButtonState::Released => return Some(false),
                }
            }
            None
        },
        POINTER_CAPTURE_EVENT,
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
        {
            let mut info_gen = 0;
            let mut mode = ClickMode::default();

            move |hovered, is_down, is_captured| {
                // cache mode
                let tree = WINDOW.info();
                if info_gen != tree.stats().generation {
                    mode = tree.get(WIDGET.id()).unwrap().click_mode();
                    info_gen = tree.stats().generation;
                }

                if mode.repeat {
                    Some(is_down || is_captured)
                } else {
                    Some(hovered && is_down)
                }
            }
        },
    )
}

/// If the mouse pointer is pressed or captured by the widget and it is enabled.
///
/// [`ENABLED`]: Interactivity::ENABLED
#[property(EVENT)]
pub fn is_cap_mouse_pressed(child: impl UiNode, state: impl IntoVar<bool>) -> impl UiNode {
    event_state2(
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
                            return Some(input_args.target.contains_enabled(WIDGET.id()));
                        }
                    }
                    ButtonState::Released => return Some(false),
                }
            }
            None
        },
        POINTER_CAPTURE_EVENT,
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

/// If the widget was clicked by shortcut or accessibility event and the [`shortcut_pressed_duration`] has not elapsed.
///
/// [`shortcut_pressed_duration`]: GESTURES::shortcut_pressed_duration
#[property(EVENT)]
pub fn is_shortcut_pressed(child: impl UiNode, state: impl IntoVar<bool>) -> impl UiNode {
    let state = state.into_var();
    let mut shortcut_press = None;

    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            state.set(false);
            WIDGET.sub_event(&CLICK_EVENT);
        }
        UiNodeOp::Deinit => {
            state.set(false);
        }
        UiNodeOp::Event { update } => {
            if let Some(args) = CLICK_EVENT.on(update) {
                if (args.is_from_keyboard() || args.is_from_access()) && args.target.contains_enabled(WIDGET.id()) {
                    // if a shortcut click happened, we show pressed for the duration of `shortcut_pressed_duration`
                    // unless we where already doing that, then we just stop showing pressed, this causes
                    // a flickering effect when rapid clicks are happening.
                    if shortcut_press.take().is_none() {
                        let duration = GESTURES.shortcut_pressed_duration().get();
                        if duration != Duration::default() {
                            let dl = TIMERS.deadline(duration);
                            dl.subscribe(UpdateOp::Update, WIDGET.id()).perm();
                            shortcut_press = Some(dl);
                            state.set(true);
                        }
                    } else {
                        state.set(false);
                    }
                }
            }
        }
        UiNodeOp::Update { updates } => {
            child.update(updates);

            if let Some(timer) = &shortcut_press {
                if timer.is_new() {
                    shortcut_press = None;
                    state.set(false);
                }
            }
        }
        _ => {}
    })
}

/// If a touch contact point is over the widget or a descendant and the it is enabled.
///
/// This state property does not consider pointer capture, if the pointer is captured by the widget
/// but is not actually over the widget this is `false`, use [`is_cap_touched`] to include the captured state.
///
/// This state property also does not consider where the touch started, if it started in a different widget
/// and is not over this widget the widget is touched, use [`is_touched_from_start`] to ignore touched that move in.
///
/// The value is always `false` when the widget is not [`ENABLED`].
///
/// [`is_cap_touched`]: fn@is_cap_touched
/// [`is_touched_from_start`]: fn@is_touched_from_start
/// [`ENABLED`]: Interactivity::ENABLED
#[property(EVENT)]
pub fn is_touched(child: impl UiNode, state: impl IntoVar<bool>) -> impl UiNode {
    event_state(child, state, false, TOUCHED_EVENT, |args| {
        if args.is_touch_enter_enabled() {
            Some(true)
        } else if args.is_touch_leave_enabled() {
            Some(false)
        } else {
            None
        }
    })
}

/// If a touch contact that started over the widget is over it and it is enabled.
///
/// This state property does not consider pointer capture, if the pointer is captured by the widget
/// but is not actually over the widget this is `false`, use [`is_cap_touched_from_start`] to include the captured state.
///
/// The value is always `false` when the widget is not [`ENABLED`].
///
/// [`ENABLED`]: Interactivity::ENABLED
/// [`is_cap_touched_from_start`]: fn@is_cap_touched_from_start
#[property(EVENT)]
pub fn is_touched_from_start(child: impl UiNode, state: impl IntoVar<bool>) -> impl UiNode {
    #[expect(clippy::mutable_key_type)] // EventPropagationHandle compares pointers, not value
    let mut touches_started = HashSet::new();
    event_state(child, state, false, TOUCHED_EVENT, move |args| {
        if args.is_touch_enter_enabled() {
            match args.phase {
                TouchPhase::Start => {
                    touches_started.retain(|t: &EventPropagationHandle| !t.is_stopped()); // for touches released outside the widget.
                    touches_started.insert(args.touch_propagation.clone());
                    Some(true)
                }
                TouchPhase::Move => Some(touches_started.contains(&args.touch_propagation)),
                TouchPhase::End | TouchPhase::Cancel => Some(false), // weird
            }
        } else if args.is_touch_leave_enabled() {
            if let TouchPhase::End | TouchPhase::Cancel = args.phase {
                touches_started.remove(&args.touch_propagation);
            }
            Some(false)
        } else {
            None
        }
    })
}

/// If a touch contact point is over the widget, or is over a descendant, or is captured by it.
///
/// The value is always `false` when the widget is not [`ENABLED`].
///
/// [`ENABLED`]: Interactivity::ENABLED
#[property(EVENT)]
pub fn is_cap_touched(child: impl UiNode, state: impl IntoVar<bool>) -> impl UiNode {
    event_state2(
        child,
        state,
        false,
        TOUCHED_EVENT,
        false,
        |hovered_args| {
            if hovered_args.is_touch_enter_enabled() {
                Some(true)
            } else if hovered_args.is_touch_leave_enabled() {
                Some(false)
            } else {
                None
            }
        },
        POINTER_CAPTURE_EVENT,
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

/// If a touch contact point is over the widget, or is over a descendant, or is captured by it.
///
/// The value is always `false` when the widget is not [`ENABLED`].
///
/// [`ENABLED`]: Interactivity::ENABLED
#[property(EVENT)]
pub fn is_cap_touched_from_start(child: impl UiNode, state: impl IntoVar<bool>) -> impl UiNode {
    #[expect(clippy::mutable_key_type)] // EventPropagationHandle compares pointers, not value
    let mut touches_started = HashSet::new();
    event_state2(
        child,
        state,
        false,
        TOUCHED_EVENT,
        false,
        move |hovered_args| {
            if hovered_args.is_touch_enter_enabled() {
                match hovered_args.phase {
                    TouchPhase::Start => {
                        touches_started.retain(|t: &EventPropagationHandle| !t.is_stopped()); // for touches released outside the widget.
                        touches_started.insert(hovered_args.touch_propagation.clone());
                        Some(true)
                    }
                    TouchPhase::Move => Some(touches_started.contains(&hovered_args.touch_propagation)),
                    TouchPhase::End | TouchPhase::Cancel => Some(false), // weird
                }
            } else if hovered_args.is_touch_leave_enabled() {
                if let TouchPhase::End | TouchPhase::Cancel = hovered_args.phase {
                    touches_started.remove(&hovered_args.touch_propagation);
                }
                Some(false)
            } else {
                None
            }
        },
        POINTER_CAPTURE_EVENT,
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

/// If [`is_mouse_pressed`] or [`is_touched_from_start`].
///
/// Note that [`is_mouse_pressed`] and [`is_touched_from_start`] do not consider pointer capture, use [`is_cap_pointer_pressed`] to
/// include the captured state.
///
/// [`is_mouse_pressed`]: fn@is_mouse_pressed
/// [`is_touched_from_start`]: fn@is_touched_from_start
/// [`is_cap_pointer_pressed`]: fn@is_cap_pointer_pressed
#[property(EVENT)]
pub fn is_pointer_pressed(child: impl UiNode, state: impl IntoVar<bool>) -> impl UiNode {
    let pressed = var_state();
    let child = is_mouse_pressed(child, pressed.clone());

    let touched = var_state();
    let child = is_touched_from_start(child, touched.clone());

    bind_state(child, var_merge!(pressed, touched, |&p, &t| p || t), state)
}

/// If [`is_mouse_pressed`], [`is_touched_from_start`] or [`is_shortcut_pressed`].
///
/// Note that [`is_mouse_pressed`] and [`is_touched_from_start`] do not consider pointer capture, use [`is_cap_pressed`] to
/// include the captured state.
///
/// [`is_mouse_pressed`]: fn@is_mouse_pressed
/// [`is_touched_from_start`]: fn@is_touched_from_start
/// [`is_shortcut_pressed`]: fn@is_shortcut_pressed
/// [`is_cap_pressed`]: fn@is_cap_pressed
#[property(EVENT)]
pub fn is_pressed(child: impl UiNode, state: impl IntoVar<bool>) -> impl UiNode {
    let pressed = var_state();
    let child = is_mouse_pressed(child, pressed.clone());

    let touched = var_state();
    let child = is_touched_from_start(child, touched.clone());

    let shortcut_pressed = var_state();
    let child = is_shortcut_pressed(child, shortcut_pressed.clone());

    bind_state(
        child,
        var_merge!(pressed, touched, shortcut_pressed, |&p, &t, &s| p || t || s),
        state,
    )
}

/// If [`is_cap_mouse_pressed`] or [`is_cap_touched_from_start`].
///
/// [`is_cap_mouse_pressed`]: fn@is_cap_mouse_pressed
/// [`is_cap_touched_from_start`]: fn@is_cap_touched_from_start
#[property(EVENT)]
pub fn is_cap_pointer_pressed(child: impl UiNode, state: impl IntoVar<bool>) -> impl UiNode {
    let pressed = var_state();
    let child = is_cap_mouse_pressed(child, pressed.clone());

    let touched = var_state();
    let child = is_cap_touched_from_start(child, touched.clone());

    bind_state(child, var_merge!(pressed, touched, |&p, &t| p || t), state)
}

/// If [`is_cap_mouse_pressed`], [`is_cap_touched_from_start`] or [`is_shortcut_pressed`].
///
/// [`is_cap_mouse_pressed`]: fn@is_cap_mouse_pressed
/// [`is_cap_touched_from_start`]: fn@is_cap_touched_from_start
/// [`is_shortcut_pressed`]: fn@is_shortcut_pressed
#[property(EVENT)]
pub fn is_cap_pressed(child: impl UiNode, state: impl IntoVar<bool>) -> impl UiNode {
    let pressed = var_state();
    let child = is_cap_mouse_pressed(child, pressed.clone());

    let touched = var_state();
    let child = is_cap_touched_from_start(child, touched.clone());

    let shortcut_pressed = var_state();
    let child = is_shortcut_pressed(child, pressed.clone());

    bind_state(
        child,
        var_merge!(pressed, touched, shortcut_pressed, |&p, &t, &s| p || t || s),
        state,
    )
}
