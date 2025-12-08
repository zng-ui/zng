use std::{collections::HashSet, time::Duration};

use zng_app::timer::TIMERS;
use zng_ext_input::{
    gesture::{CLICK_EVENT, GESTURES},
    mouse::{ClickMode, MOUSE_HOVERED_EVENT, MOUSE_INPUT_EVENT, MOUSE_MOVE_EVENT, MOUSE_WHEEL_EVENT, WidgetInfoMouseExt as _},
    pointer_capture::POINTER_CAPTURE_EVENT,
    touch::{TOUCH_TAP_EVENT, TOUCHED_EVENT},
};
use zng_view_api::{mouse::ButtonState, touch::TouchPhase};
use zng_wgt::{node::validate_getter_var, prelude::*};

/// If the mouse pointer is over the widget or a descendant and the widget is disabled.
#[property(EVENT)]
pub fn is_hovered_disabled(child: impl IntoUiNode, state: impl IntoVar<bool>) -> UiNode {
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
pub fn is_hovered(child: impl IntoUiNode, state: impl IntoVar<bool>) -> UiNode {
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

/// If the mouse pointer is over the widget, or a descendant, or is captured by it.
///
/// The value is always `false` when the widget is not [`ENABLED`].
///
/// [`ENABLED`]: Interactivity::ENABLED
#[property(EVENT)]
pub fn is_cap_hovered(child: impl IntoUiNode, state: impl IntoVar<bool>) -> UiNode {
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
pub fn is_mouse_pressed(child: impl IntoUiNode, state: impl IntoVar<bool>) -> UiNode {
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
#[property(EVENT)]
pub fn is_cap_mouse_pressed(child: impl IntoUiNode, state: impl IntoVar<bool>) -> UiNode {
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
pub fn is_shortcut_pressed(child: impl IntoUiNode, state: impl IntoVar<bool>) -> UiNode {
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
            if let Some(args) = CLICK_EVENT.on(update)
                && (args.is_from_keyboard() || args.is_from_access())
                && args.target.contains_enabled(WIDGET.id())
            {
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
        UiNodeOp::Update { updates } => {
            child.update(updates);

            if let Some(timer) = &shortcut_press
                && timer.is_new()
            {
                shortcut_press = None;
                state.set(false);
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
pub fn is_touched(child: impl IntoUiNode, state: impl IntoVar<bool>) -> UiNode {
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
pub fn is_touched_from_start(child: impl IntoUiNode, state: impl IntoVar<bool>) -> UiNode {
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
pub fn is_cap_touched(child: impl IntoUiNode, state: impl IntoVar<bool>) -> UiNode {
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
pub fn is_cap_touched_from_start(child: impl IntoUiNode, state: impl IntoVar<bool>) -> UiNode {
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
pub fn is_pointer_pressed(child: impl IntoUiNode, state: impl IntoVar<bool>) -> UiNode {
    let pressed = var_state();
    let child = is_mouse_pressed(child, pressed.clone());

    let touched = var_state();
    let child = is_touched_from_start(child, touched.clone());

    bind_state(child, merge_var!(pressed, touched, |&p, &t| p || t), state)
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
pub fn is_pressed(child: impl IntoUiNode, state: impl IntoVar<bool>) -> UiNode {
    let pressed = var_state();
    let child = is_mouse_pressed(child, pressed.clone());

    let touched = var_state();
    let child = is_touched_from_start(child, touched.clone());

    let shortcut_pressed = var_state();
    let child = is_shortcut_pressed(child, shortcut_pressed.clone());

    bind_state(
        child,
        merge_var!(pressed, touched, shortcut_pressed, |&p, &t, &s| p || t || s),
        state,
    )
}

/// If [`is_cap_mouse_pressed`] or [`is_cap_touched_from_start`].
///
/// [`is_cap_mouse_pressed`]: fn@is_cap_mouse_pressed
/// [`is_cap_touched_from_start`]: fn@is_cap_touched_from_start
#[property(EVENT)]
pub fn is_cap_pointer_pressed(child: impl IntoUiNode, state: impl IntoVar<bool>) -> UiNode {
    let pressed = var_state();
    let child = is_cap_mouse_pressed(child, pressed.clone());

    let touched = var_state();
    let child = is_cap_touched_from_start(child, touched.clone());

    bind_state(child, merge_var!(pressed, touched, |&p, &t| p || t), state)
}

/// If [`is_cap_mouse_pressed`], [`is_cap_touched_from_start`] or [`is_shortcut_pressed`].
///
/// [`is_cap_mouse_pressed`]: fn@is_cap_mouse_pressed
/// [`is_cap_touched_from_start`]: fn@is_cap_touched_from_start
/// [`is_shortcut_pressed`]: fn@is_shortcut_pressed
#[property(EVENT)]
pub fn is_cap_pressed(child: impl IntoUiNode, state: impl IntoVar<bool>) -> UiNode {
    let pressed = var_state();
    let child = is_cap_mouse_pressed(child, pressed.clone());

    let touched = var_state();
    let child = is_cap_touched_from_start(child, touched.clone());

    let shortcut_pressed = var_state();
    let child = is_shortcut_pressed(child, pressed.clone());

    bind_state(
        child,
        merge_var!(pressed, touched, shortcut_pressed, |&p, &t, &s| p || t || s),
        state,
    )
}

/// If the mouse pointer moved over or interacted with the widget within a time duration defined by contextual [`mouse_active_config`].
///
/// This property is useful for implementing things like a media player widget, where the mouse cursor and controls vanish
/// after the mouse stops moving for a time.
///
/// See also [`is_pointer_active`] for an aggregate gesture that covers mouse and touch.
///
/// [`mouse_active_config`]: fn@mouse_active_config
/// [`is_pointer_active`]: fn@is_pointer_active
#[property(EVENT)]
pub fn is_mouse_active(child: impl IntoUiNode, state: impl IntoVar<bool>) -> UiNode {
    let state = state.into_var();
    enum State {
        False,
        Maybe(DipPoint),
        True(DipPoint, TimerVar),
    }
    let mut raw_state = State::False;
    match_node(child, move |_, op| match op {
        UiNodeOp::Init => {
            validate_getter_var(&state);
            WIDGET.sub_event(&MOUSE_MOVE_EVENT).sub_var(&MOUSE_ACTIVE_CONFIG_VAR);
            state.set(true);
        }
        UiNodeOp::Deinit => {
            state.set(false);
            raw_state = State::False;
        }
        UiNodeOp::Event { update } => {
            let mut start = None;
            if let Some(args) = MOUSE_MOVE_EVENT.on(update) {
                match &mut raw_state {
                    State::False => {
                        let cfg = MOUSE_ACTIVE_CONFIG_VAR.get();
                        if cfg.area.width <= Dip::new(1) || cfg.area.height <= Dip::new(1) {
                            start = Some((cfg.duration, args.position));
                        } else {
                            raw_state = State::Maybe(args.position);
                        }
                    }
                    State::Maybe(s) => {
                        let cfg = MOUSE_ACTIVE_CONFIG_VAR.get();
                        if (args.position.x - s.x).abs() >= cfg.area.width || (args.position.y - s.y).abs() >= cfg.area.height {
                            start = Some((cfg.duration, args.position));
                        }
                    }
                    State::True(p, timer) => {
                        if (args.position.x - p.x).abs() >= Dip::new(1) || (args.position.y - p.y).abs() >= Dip::new(1) {
                            // reset
                            timer.get().play(true);
                            *p = args.position;
                        }
                    }
                }
            } else {
                let pos = if let Some(args) = MOUSE_INPUT_EVENT.on(update) {
                    Some(args.position)
                } else {
                    MOUSE_WHEEL_EVENT.on(update).map(|args| args.position)
                };
                if let Some(pos) = pos {
                    match &raw_state {
                        State::True(_, timer) => {
                            // reset
                            timer.get().play(true);
                        }
                        _ => {
                            start = Some((MOUSE_ACTIVE_CONFIG_VAR.get().duration, pos));
                        }
                    }
                }
            }
            if let Some((t, pos)) = start {
                let timer = TIMERS.interval(t, false);
                timer.subscribe(UpdateOp::Update, WIDGET.id()).perm();
                state.set(true);
                raw_state = State::True(pos, timer);
            }
        }
        UiNodeOp::Update { .. } => {
            if let State::True(_, timer) = &raw_state {
                if let Some(timer) = timer.get_new() {
                    timer.stop();
                    state.set(false);
                    raw_state = State::False;
                } else if let Some(cfg) = MOUSE_ACTIVE_CONFIG_VAR.get_new() {
                    timer.get().set_interval(cfg.duration);
                }
            }
        }
        _ => {}
    })
}

/// Contextual configuration for [`is_mouse_active`].
///
/// Note that the [`MouseActiveConfig`] converts from duration, so you can set this to a time *literal*, like `5.secs()`, directly.
///
/// This property sets the [`MOUSE_ACTIVE_CONFIG_VAR`].
///
/// [`is_mouse_active`]: fn@is_mouse_active
#[property(CONTEXT, default(MOUSE_ACTIVE_CONFIG_VAR))]
pub fn mouse_active_config(child: impl IntoUiNode, config: impl IntoVar<MouseActiveConfig>) -> UiNode {
    with_context_var(child, MOUSE_ACTIVE_CONFIG_VAR, config)
}

/// If an unhandled touch tap has happened on the widget within a time duration defined by contextual [`touch_active_config`].
///
/// This property is the touch equivalent to [`is_mouse_active`].
///
/// [`touch_active_config`]: fn@touch_active_config
/// [`is_mouse_active`]: fn@is_mouse_active
#[property(EVENT)]
pub fn is_touch_active(child: impl IntoUiNode, state: impl IntoVar<bool>) -> UiNode {
    let state = state.into_var();
    enum State {
        False,
        True(TimerVar),
    }
    let mut raw_state = State::False;
    match_node(child, move |c, op| match op {
        UiNodeOp::Init => {
            validate_getter_var(&state);
            WIDGET.sub_event(&TOUCH_TAP_EVENT).sub_var(&TOUCH_ACTIVE_CONFIG_VAR);
            state.set(false);
        }
        UiNodeOp::Deinit => {
            state.set(false);
            raw_state = State::False;
        }
        UiNodeOp::Event { update } => {
            c.event(update);
            if TOUCH_TAP_EVENT.on_unhandled(update).is_some() {
                match &raw_state {
                    State::False => {
                        let t = TOUCH_ACTIVE_CONFIG_VAR.get().duration;
                        let timer = TIMERS.interval(t, false);
                        timer.subscribe(UpdateOp::Update, WIDGET.id()).perm();
                        state.set(true);
                        raw_state = State::True(timer);
                    }
                    State::True(timer) => {
                        let cfg = TOUCH_ACTIVE_CONFIG_VAR.get();
                        if cfg.toggle {
                            state.set(false);
                            timer.get().stop();
                        } else {
                            timer.get().play(true);
                        }
                    }
                }
            }
        }
        UiNodeOp::Update { .. } => {
            if let State::True(timer) = &raw_state {
                if let Some(timer) = timer.get_new() {
                    timer.stop();
                    state.set(false);
                    raw_state = State::False;
                } else if let Some(cfg) = TOUCH_ACTIVE_CONFIG_VAR.get_new() {
                    timer.get().set_interval(cfg.duration);
                }
            }
        }
        _ => {}
    })
}

/// Contextual configuration for [`is_touch_active`].
///
/// Note that the [`TouchActiveConfig`] converts from duration, so you can set this to a time *literal*, like `5.secs()`, directly.
///
/// This property sets the [`MOUSE_ACTIVE_CONFIG_VAR`].
///
/// [`is_touch_active`]: fn@is_touch_active
#[property(CONTEXT, default(TOUCH_ACTIVE_CONFIG_VAR))]
pub fn touch_active_config(child: impl IntoUiNode, config: impl IntoVar<TouchActiveConfig>) -> UiNode {
    with_context_var(child, TOUCH_ACTIVE_CONFIG_VAR, config)
}

/// If [`is_mouse_active`] or [`is_touch_active`].
///
/// [`is_mouse_active`]: fn@is_mouse_active
/// [`is_touch_active`]: fn@is_touch_active
#[property(EVENT)]
pub fn is_pointer_active(child: impl IntoUiNode, state: impl IntoVar<bool>) -> UiNode {
    let mouse_active = var_state();
    let child = is_mouse_active(child, mouse_active.clone());

    let touch_active = var_state();
    let child = is_touch_active(child, touch_active.clone());

    bind_state(
        child,
        expr_var! {
            *#{mouse_active} || *#{touch_active}
        },
        state,
    )
}

context_var! {
    /// Configuration for [`is_mouse_active`].
    ///
    /// [`is_mouse_active`]: fn@is_mouse_active
    pub static MOUSE_ACTIVE_CONFIG_VAR: MouseActiveConfig = MouseActiveConfig::default();
    /// Configuration for [`is_touch_active`].
    ///
    /// [`is_touch_active`]: fn@is_touch_active
    pub static TOUCH_ACTIVE_CONFIG_VAR: TouchActiveConfig = TouchActiveConfig::default();
}

/// Configuration for mouse active property.
///
/// See [`mouse_active_config`] for more details.
///
/// [`mouse_active_config`]: fn@mouse_active_config
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub struct MouseActiveConfig {
    /// Maximum time the state remains active after mouse leave or stops moving.
    pub duration: Duration,
    /// Minimum distance the pointer must move before state changes to active.
    pub area: DipSize,
}
impl Default for MouseActiveConfig {
    /// `(3s, 1)`
    fn default() -> Self {
        Self {
            duration: 3.secs(),
            area: DipSize::splat(Dip::new(1)),
        }
    }
}
impl_from_and_into_var! {
    fn from(duration: Duration) -> MouseActiveConfig {
        MouseActiveConfig {
            duration,
            ..Default::default()
        }
    }

    fn from(area: DipSize) -> MouseActiveConfig {
        MouseActiveConfig {
            area,
            ..Default::default()
        }
    }

    fn from((duration, area): (Duration, DipSize)) -> MouseActiveConfig {
        MouseActiveConfig { duration, area }
    }
}

/// Configuration for touch active property.
///
/// See [`touch_active_config`] for more details.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub struct TouchActiveConfig {
    /// Maximum time the state remains active after no touch interaction.
    pub duration: Duration,
    /// If a second unhandled interaction deactivates.
    pub toggle: bool,
}
impl Default for TouchActiveConfig {
    /// `(3s, false)`
    fn default() -> Self {
        Self {
            duration: 3.secs(),
            toggle: false,
        }
    }
}
impl_from_and_into_var! {
    fn from(duration: Duration) -> TouchActiveConfig {
        TouchActiveConfig {
            duration,
            ..Default::default()
        }
    }

    fn from((duration, toggle): (Duration, bool)) -> TouchActiveConfig {
        TouchActiveConfig { duration, toggle }
    }
}
