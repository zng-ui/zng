use std::{collections::HashSet, num::Wrapping, time::Duration};

use zng_app::timer::TIMERS;
use zng_ext_input::{
    gesture::{CLICK_EVENT, GESTURES},
    mouse::{ClickMode, MOUSE_HOVERED_EVENT, MOUSE_INPUT_EVENT, MOUSE_MOVE_EVENT, MOUSE_WHEEL_EVENT, WidgetInfoMouseExt as _},
    pointer_capture::POINTER_CAPTURE_EVENT,
    touch::{TOUCH_TAP_EVENT, TOUCHED_EVENT},
};
use zng_ext_window::WINDOWS;
use zng_view_api::{mouse::ButtonState, touch::TouchPhase};
use zng_wgt::{
    node::{bind_state_init, validate_getter_var},
    prelude::*,
};

/// If the mouse pointer is over the widget or a descendant and the widget is disabled.
#[property(EVENT)]
pub fn is_hovered_disabled(child: impl IntoUiNode, state: impl IntoVar<bool>) -> UiNode {
    bind_state_init(child, state, |s| {
        let wgt = (WINDOW.id(), WIDGET.id());
        MOUSE_HOVERED_EVENT.var_bind(s, move |args| {
            if args.is_mouse_enter_disabled(wgt) {
                Some(true)
            } else if args.is_mouse_leave_disabled(wgt) {
                Some(false)
            } else {
                None
            }
        })
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
    bind_state_init(child, state, |s| {
        let wgt = (WINDOW.id(), WIDGET.id());
        MOUSE_HOVERED_EVENT.var_bind(s, move |args| {
            if args.is_mouse_enter_enabled(wgt) {
                Some(true)
            } else if args.is_mouse_leave_enabled(wgt) {
                Some(false)
            } else {
                None
            }
        })
    })
}

fn hovered_var(wgt: (WindowId, WidgetId)) -> Var<bool> {
    MOUSE_HOVERED_EVENT.var_map(
        clmv!(wgt, |args| {
            if args.is_mouse_enter_enabled(wgt) {
                Some(true)
            } else if args.is_mouse_leave_enabled(wgt) {
                Some(false)
            } else {
                None
            }
        }),
        || false,
    )
}

fn captured_var(id: WidgetId) -> Var<bool> {
    POINTER_CAPTURE_EVENT.var_map(
        move |args| {
            if args.is_got(id) {
                Some(true)
            } else if args.is_lost(id) {
                Some(false)
            } else {
                None
            }
        },
        || false,
    )
}

/// If the mouse pointer is over the widget, or a descendant, or is captured by it.
///
/// The value is always `false` when the widget is not [`ENABLED`].
///
/// [`ENABLED`]: Interactivity::ENABLED
#[property(EVENT)]
pub fn is_cap_hovered(child: impl IntoUiNode, state: impl IntoVar<bool>) -> UiNode {
    bind_state_init(child, state, |s| {
        let wgt = (WINDOW.id(), WIDGET.id());

        let actual_state = expr_var!(*#{hovered_var(wgt)} || *#{captured_var(wgt.1)});
        actual_state.set_bind(s).perm();
        s.hold(actual_state)
    })
}

fn pressed_var(wgt: (WindowId, WidgetId)) -> Var<bool> {
    MOUSE_INPUT_EVENT.var_map(
        move |args| {
            if args.is_primary() {
                match args.state {
                    ButtonState::Pressed => {
                        if args.capture_allows(wgt) {
                            return Some(args.target.contains_enabled(wgt.1));
                        }
                    }
                    ButtonState::Released => return Some(false),
                }
            }
            None
        },
        || false,
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
    bind_state_init(child, state, |s| {
        let wgt = (WINDOW.id(), WIDGET.id());
        let mut info_gen = 0;
        let mut mode = ClickMode::default();
        let actual_state = expr_var! {
            let hovered = *#{hovered_var(wgt)};
            let captured = *#{captured_var(wgt.1)};
            let pressed = *#{pressed_var(wgt)};

            if let Some(tree) = WINDOWS.widget_tree(wgt.0) {
                let t_gen = tree.stats().generation;
                if info_gen != t_gen {
                    // cache mode to avoid some queries
                    info_gen = t_gen;
                    if let Some(w) = tree.get(wgt.1) {
                        mode = w.click_mode();
                    }
                }

                if mode.repeat { pressed || captured } else { hovered && pressed }
            } else {
                false
            }
        };
        actual_state.set_bind(s).perm();
        s.hold(actual_state)
    })
}

/// If the mouse pointer is pressed or captured by the widget and it is enabled.
#[property(EVENT)]
pub fn is_cap_mouse_pressed(child: impl IntoUiNode, state: impl IntoVar<bool>) -> UiNode {
    bind_state_init(child, state, |s| {
        let wgt = (WINDOW.id(), WIDGET.id());
        let actual_state = expr_var! {
            *#{pressed_var(wgt)} || *#{captured_var(wgt.1)}
        };
        actual_state.set_bind(s).perm();
        s.hold(actual_state)
    })
}

/// If the widget was clicked by shortcut or accessibility event and the [`shortcut_pressed_duration`] has not elapsed.
///
/// [`shortcut_pressed_duration`]: GESTURES::shortcut_pressed_duration
#[property(EVENT)]
pub fn is_shortcut_pressed(child: impl IntoUiNode, state: impl IntoVar<bool>) -> UiNode {
    bind_state_init(child, state, |s| {
        let id = WIDGET.id();
        let mut shortcut_press = None::<DeadlineVar>;
        CLICK_EVENT.hook(clmv!(s, |args| {
            if (args.is_from_keyboard() || args.is_from_access()) && args.target.contains_enabled(id) {
                // if a shortcut click happened, we show pressed for the duration of `shortcut_pressed_duration`
                // unless we where already doing that, then we just stop showing pressed, this causes
                // a flickering effect when rapid clicks are happening.
                let d = shortcut_press.take();
                if d.is_none() || matches!(d, Some(p) if p.get().has_elapsed()) {
                    let duration = GESTURES.shortcut_pressed_duration().get();
                    if duration > Duration::ZERO {
                        let dl = TIMERS.deadline(duration);
                        dl.hook(clmv!(s, |t| {
                            let elapsed = t.value().has_elapsed();
                            if elapsed {
                                s.set(false);
                            }
                            !elapsed
                        }))
                        .perm();
                        shortcut_press = Some(dl);
                        s.set(true);
                    }
                } else {
                    s.set(false);
                }
            }
            true
        }))
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
    bind_state_init(child, state, |s| {
        let wgt = (WINDOW.id(), WIDGET.id());
        TOUCHED_EVENT.var_bind(s, move |args| {
            if args.is_touch_enter_enabled(wgt) {
                Some(true)
            } else if args.is_touch_leave_enabled(wgt) {
                Some(false)
            } else {
                None
            }
        })
    })
}
fn touched_var(wgt: (WindowId, WidgetId)) -> Var<bool> {
    TOUCHED_EVENT.var_map(
        move |args| {
            if args.is_touch_enter_enabled(wgt) {
                Some(true)
            } else if args.is_touch_leave_enabled(wgt) {
                Some(false)
            } else {
                None
            }
        },
        || false,
    )
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
    bind_state_init(child, state, |s| {
        #[expect(clippy::mutable_key_type)] // EventPropagationHandle compares pointers, not value
        let mut touches_started = HashSet::new();
        let wgt = (WINDOW.id(), WIDGET.id());
        TOUCHED_EVENT.var_bind(s, move |args| {
            if args.is_touch_enter_enabled(wgt) {
                match args.phase {
                    TouchPhase::Start => {
                        touches_started.retain(|t: &EventPropagationHandle| !t.is_stopped()); // for touches released outside the widget.
                        touches_started.insert(args.touch_propagation.clone());
                        Some(true)
                    }
                    TouchPhase::Move => Some(touches_started.contains(&args.touch_propagation)),
                    TouchPhase::End | TouchPhase::Cancel => Some(false), // weird
                }
            } else if args.is_touch_leave_enabled(wgt) {
                if let TouchPhase::End | TouchPhase::Cancel = args.phase {
                    touches_started.remove(&args.touch_propagation);
                }
                Some(false)
            } else {
                None
            }
        })
    })
}
fn touched_from_start_var(wgt: (WindowId, WidgetId)) -> Var<bool> {
    #[expect(clippy::mutable_key_type)] // EventPropagationHandle compares pointers, not value
    let mut touches_started = HashSet::new();
    TOUCHED_EVENT.var_map(
        move |args| {
            if args.is_touch_enter_enabled(wgt) {
                match args.phase {
                    TouchPhase::Start => {
                        touches_started.retain(|t: &EventPropagationHandle| !t.is_stopped()); // for touches released outside the widget.
                        touches_started.insert(args.touch_propagation.clone());
                        Some(true)
                    }
                    TouchPhase::Move => Some(touches_started.contains(&args.touch_propagation)),
                    TouchPhase::End | TouchPhase::Cancel => Some(false), // weird
                }
            } else if args.is_touch_leave_enabled(wgt) {
                if let TouchPhase::End | TouchPhase::Cancel = args.phase {
                    touches_started.remove(&args.touch_propagation);
                }
                Some(false)
            } else {
                None
            }
        },
        || false,
    )
}

/// If a touch contact point is over the widget, or is over a descendant, or is captured by it.
///
/// The value is always `false` when the widget is not [`ENABLED`].
///
/// [`ENABLED`]: Interactivity::ENABLED
#[property(EVENT)]
pub fn is_cap_touched(child: impl IntoUiNode, state: impl IntoVar<bool>) -> UiNode {
    bind_state_init(child, state, |s| {
        let wgt = (WINDOW.id(), WIDGET.id());
        let actual_state = expr_var! {
            *#{touched_var(wgt)} || *#{captured_var(wgt.1)}
        };
        actual_state.set_bind(s).perm();
        s.hold(actual_state)
    })
}

/// If a touch contact point is over the widget, or is over a descendant, or is captured by it.
///
/// The value is always `false` when the widget is not [`ENABLED`].
///
/// [`ENABLED`]: Interactivity::ENABLED
#[property(EVENT)]
pub fn is_cap_touched_from_start(child: impl IntoUiNode, state: impl IntoVar<bool>) -> UiNode {
    bind_state_init(child, state, |s| {
        let wgt = (WINDOW.id(), WIDGET.id());
        let actual_state = expr_var! {
            *#{touched_from_start_var(wgt)} || *#{captured_var(wgt.1)}
        };
        actual_state.set_bind(s).perm();
        s.hold(actual_state)
    })
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
    match_node(child, move |_, op| match op {
        UiNodeOp::Init => {
            let mut timer = None::<TimerVar>;
            let cfg = MOUSE_ACTIVE_CONFIG_VAR.current_context();

            // variable set by mouse events when mouse activity is observed
            let activate = var(Wrapping(0u8));
            let mut last = u8::MAX;
            activate
                .hook(clmv!(cfg, state, |args| {
                    let cfg = cfg.get();
                    if cfg.duration == Duration::ZERO {
                        if timer.take().is_some() {
                            // just disabled
                            state.set(false);
                        }
                        return true;
                    }

                    let n = args.value().0;
                    let is_activate = last != n;
                    last = n;

                    if is_activate {
                        if let Some(t) = &timer {
                            t.with(|t| {
                                if t.count() > 0 {
                                    // activate again
                                    state.set(true);
                                    t.set_count(0);
                                }
                                // update cfg if needed
                                t.set_interval(cfg.duration);
                                // restart or reset running timer
                                t.play(true);
                            });
                        } else {
                            state.set(true);
                            // start timer that will disable the state on elapsed and pause,
                            // the timer is reused on subsequent activations
                            let t = TIMERS.interval(cfg.duration, true);
                            t.hook(clmv!(state, |t| {
                                let t = t.value();
                                if t.count() > 0 {
                                    t.pause();
                                    state.set(false);
                                }
                                true
                            }))
                            .perm();
                            t.with(|t| t.play(false));
                            timer = Some(t);
                        }
                    }

                    true
                }))
                .perm();

            let id = WIDGET.id();

            let mut first_pos = None::<DipPoint>;

            // update timer interval
            let handle = cfg.hook(clmv!(activate, |_| {
                activate.update();
                true
            }));
            // activate on mouse move >= cfg.area
            let handle = MOUSE_MOVE_EVENT.hook(clmv!(activate, cfg, |args| {
                let _hold = &handle;
                if args.target.contains(id) {
                    let dist = if let Some(prev_pos) = first_pos {
                        (prev_pos - args.position).abs()
                    } else {
                        first_pos = Some(args.position);
                        DipVector::zero()
                    };
                    let cfg = cfg.get();
                    if dist.x >= cfg.area.width || dist.y >= cfg.area.height {
                        activate.modify(|c| **c += 1);
                    }
                } else {
                    first_pos = None;
                }
                true
            }));
            // activate on mouse wheel
            let handle = MOUSE_WHEEL_EVENT.hook(clmv!(activate, |args| {
                let _hold = &handle;
                if args.target.contains(id) {
                    activate.modify(|c| **c += 1);
                }
                true
            }));
            // activate on mouse input
            let handle = MOUSE_INPUT_EVENT.hook(clmv!(activate, |args| {
                let _hold = &handle;
                if args.target.contains(id) {
                    activate.modify(|c| **c += 1);
                }
                true
            }));
            // event always hooks, so its ok to chain handles like this
            WIDGET.push_var_handle(handle);
        }
        UiNodeOp::Deinit if state.get() => {
            state.set(false);
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
#[property(EVENT - 1)]
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
        UiNodeOp::Update { updates } => {
            c.update(updates);

            if TOUCH_TAP_EVENT.has_update(false) {
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
