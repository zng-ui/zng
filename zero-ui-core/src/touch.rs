//! Touch events and service.
//!
//! The app extension [`TouchManager`] provides the events and service. It is included in the default application.

use std::mem;

use hashbrown::HashMap;
pub use zero_ui_view_api::{TouchConfig, TouchForce, TouchId, TouchPhase, TouchUpdate};

use crate::{
    app::{raw_events::*, *},
    context::*,
    event::*,
    keyboard::{ModifiersState, MODIFIERS_CHANGED_EVENT},
    pointer_capture::{CaptureInfo, POINTER_CAPTURE, POINTER_CAPTURE_EVENT},
    units::*,
    var::*,
    widget_info::{HitTestInfo, InteractionPath, WidgetInfoTree},
    widget_instance::WidgetId,
    window::{WindowId, WIDGET_INFO_CHANGED_EVENT, WINDOWS},
};

/// Application extension that provides touch events and service.
///
/// # Events
///
/// Events this extension provides.
///
/// * [`TOUCH_MOVE_EVENT`]
/// * [`TOUCH_INPUT_EVENT`]
/// * [`TOUCHED_EVENT`]
/// * [`TOUCH_TAP_EVENT`]
///
/// # Services
///
/// Services this extension provides.
///
/// * [`TOUCH`]
///
/// # Default
///
/// This extension is included in the [default app], events provided by it
/// are required by multiple other extensions.
///
/// [default app]: crate::app::App::default
#[derive(Default)]
pub struct TouchManager {
    modifiers: ModifiersState,
    pressed: HashMap<TouchId, PressedInfo>,
    tap_start: Option<TapStart>,
}
struct PressedInfo {
    touch_propagation: EventPropagationHandle,
    target: InteractionPath,
    device_id: DeviceId,
    position: DipPoint,
    force: Option<TouchForce>,
    hits: HitTestInfo,
}

/// Touch service.
///
/// # Touch Capture
///
/// Touch capture is integrated with mouse capture in the [`POINTER_CAPTURE`] service.
///
/// # Provider
///
/// This service is provided by the [`TouchManager`] extension.
///
/// [`POINTER_CAPTURE`]: crate::pointer_capture::POINTER_CAPTURE
pub struct TOUCH;

impl TOUCH {
    /// Read-only variable that tracks the system touch config.
    ///
    /// Note that some of these configs are not always used, a tap event for example can happen even if the
    /// touch moves out of the `tap_area` when there is no ambiguity.
    ///
    /// # Value Source
    ///
    /// The value comes from the operating system settings, the variable
    /// updates with a new value if the system setting is changed.
    ///
    /// In headless apps the default is [`TouchConfig::default`] and does not change.
    ///
    /// Internally the [`RAW_TOUCH_CONFIG_CHANGED_EVENT`] is listened to update this variable, so you can notify
    /// this event to set this variable, if you really must.
    pub fn touch_config(&self) -> ReadOnlyArcVar<TouchConfig> {
        TOUCH_SV.read().touch_config.read_only()
    }
}

app_local! {
    static TOUCH_SV: TouchService = TouchService {
        touch_config: var(TouchConfig::default())
    };
}
struct TouchService {
    touch_config: ArcVar<TouchConfig>,
}

/// Identify the moves of one touch contact in [`TouchMoveArgs`].
#[derive(Debug, Clone)]
pub struct TouchMove {
    /// Identify the touch contact or *finger*.
    ///
    /// Multiple points of contact can happen in the same device at the same time,
    /// this ID identifies each uninterrupted contact. IDs are unique only among other concurrent touches
    /// on the same device, after a touch is ended an ID may be reused.
    pub touch: TouchId,

    /// Handle across the lifetime of `touch`.
    ///
    /// See [`TouchInputArgs::touch_propagation`] for more details.
    pub touch_propagation: EventPropagationHandle,

    /// Coalesced moves of the touch since last event.
    ///
    /// Last entry is the latest position.
    pub moves: Vec<(DipPoint, Option<TouchForce>)>,

    /// Hit-test result for the latest touch point in the window.
    pub hits: HitTestInfo,

    /// Full path to the top-most hit in [`hits`](TouchMove::hits).
    pub target: InteractionPath,
}
impl TouchMove {
    /// Latest position.
    pub fn position(&self) -> DipPoint {
        self.moves.last().map(|(p, _)| *p).unwrap_or_else(DipPoint::zero)
    }
}

event_args! {
    /// Arguments for [`TOUCH_MOVE_EVENT`].
    pub struct TouchMoveArgs {
        /// Id of window that received all touches in this event.
        pub window_id: WindowId,

        /// Id of device that generated all touches in this event.
        pub device_id: DeviceId,

        /// All touch contacts that moved since last event.
        ///
        /// Note that if a touch contact did not move it will not be in the list, the touch may still be active
        /// however, the [`TOUCH_INPUT_EVENT`] can be used to track touch start and end.
        pub touches: Vec<TouchMove>,

        /// Current pointer capture.
        pub capture: Option<CaptureInfo>,

        /// What modifier keys where pressed when this event happened.
        pub modifiers: ModifiersState,

        ..

        /// Each [`TouchMove::target`] and [`capture`].
        ///
        /// [`capture`]: Self::capture
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            for t in &self.touches {
                list.insert_path(&t.target);
            }
            if let Some(c) = &self.capture {
                list.insert_path(&c.target);
            }
        }
    }

    /// Arguments for [`TOUCH_INPUT_EVENT`].
    pub struct TouchInputArgs {
        /// Id of window that received the event.
        pub window_id: WindowId,

        /// Id of device that generated the event.
        pub device_id: DeviceId,

        /// Identify the touch contact or *finger*.
        ///
        /// Multiple points of contact can happen in the same device at the same time,
        /// this ID identifies each uninterrupted contact. IDs are unique only among other concurrent touches
        /// on the same device, after a touch is ended an ID may be reused.
        pub touch: TouchId,

        /// Propagation handle for the [`touch`] lifetime.
        ///
        /// The [`TOUCH_INPUT_EVENT`] and [`TOUCH_MOVE_EVENT`] have their own separate propagation handles, but
        /// touch gesture events aggregate all these events to produce a single *gesture event*, usually only a single
        /// gesture should be generated, multiple gestures can disambiguate using this `touch_propagation` handle.
        ///
        /// As an example, [`TOUCH_TAP_EVENT`] only tries to match the gesture if it has subscribers, and only notifies
        /// if by the time the gesture completes the `touch_propagation` was not stopped. Touch gesture events or event properties
        /// must stop touch propagation as soon as they commit to a gesture, a *pan* gesture for example, must stop as soon as
        /// it starts scrolling, otherwise the user may accidentally scroll and tap a button at the same time.
        ///
        /// The propagation handle always signals *stopped* after the touch ends. Handles are unique while at least one
        /// clone of it remains, this makes this a better unique identifier of a touch contact than [`TouchId`] that may be reused
        /// by the system as soon as a new touch contact is made.
        ///
        /// [`touch`]: Self::touch
        pub touch_propagation: EventPropagationHandle,

        /// Center of the touch in the window's content area.
        pub position: DipPoint,

        /// Touch pressure force and angle.
        pub force: Option<TouchForce>,

        /// Touch phase.
        ///
        /// Does not include `Moved`.
        pub phase: TouchPhase,

        /// Hit-test result for the touch point in the window.
        pub hits: HitTestInfo,

        /// Full path to the top-most hit in [`hits`](TouchInputArgs::hits).
        pub target: InteractionPath,

        /// Current pointer capture.
        pub capture: Option<CaptureInfo>,

        /// What modifier keys where pressed when this event happened.
        pub modifiers: ModifiersState,

        ..

        /// The [`target`] and [`capture`].
        ///
        /// [`target`]: Self::target
        /// [`capture`]: Self::capture
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.insert_path(&self.target);
            if let Some(c) = &self.capture {
                list.insert_path(&c.target);
            }
        }
    }

    /// Arguments for [`TOUCHED_EVENT`].
    pub struct TouchedArgs {
        /// Id of window that received the event.
        pub window_id: WindowId,

        /// Id of device that generated the event.
        pub device_id: Option<DeviceId>,

        /// Identify the touch contact or *finger*.
        ///
        /// Multiple points of contact can happen in the same device at the same time,
        /// this ID identifies each uninterrupted contact. IDs are unique only among other concurrent touches
        /// on the same device, after a touch is ended an ID may be reused.
        pub touch: TouchId,

        /// Handle across the lifetime of `touch`.
        ///
        /// See [`TouchInputArgs::touch_propagation`] for more details.
        pub touch_propagation: EventPropagationHandle,

        /// Center of the touch in the window's content area.
        pub position: DipPoint,

        /// Touch pressure force and angle.
        pub force: Option<TouchForce>,

        /// Touch phase that caused the contact gain or loss with the widget.
        pub phase: TouchPhase,

        /// Hit-test result for the touch point in the window.
        pub hits: HitTestInfo,

        /// Previous top-most hit before the touch moved.
        pub prev_target: Option<InteractionPath>,

        /// Full path to the top-most hit in [`hits`](TouchInputArgs::hits).
        pub target: Option<InteractionPath>,

        /// Previous pointer capture.
        pub prev_capture: Option<CaptureInfo>,

        /// Current pointer capture.
        pub capture: Option<CaptureInfo>,

        ..

        /// The [`prev_target`], [`target`] and [`capture`].
        ///
        /// [`prev_target`]: Self::prev_target
        /// [`target`]: Self::target
        /// [`capture`]: Self::capture
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            if let Some(p) = &self.prev_target {
                list.insert_path(p);
            }
            if let Some(p) = &self.target {
                list.insert_path(p);
            }
            if let Some(c) = &self.capture {
                list.insert_path(&c.target);
            }
        }
    }

    /// Arguments for [`TOUCH_TAP_EVENT`].
    pub struct TouchTapArgs {
        /// Id of window that received the event.
        pub window_id: WindowId,

        /// Id of device that generated the event.
        pub device_id: DeviceId,

        /// Identify the touch contact or *finger*.
        ///
        /// Multiple points of contact can happen in the same device at the same time,
        /// this ID identifies each uninterrupted contact. IDs are unique only among other concurrent touches
        /// on the same device, after a touch is ended an ID may be reused.
        pub touch: TouchId,

        /// Center of the touch in the window's content area.
        pub position: DipPoint,

        /// Hit-test result for the touch point in the window.
        pub hits: HitTestInfo,

        /// Full path to the top-most hit in [`hits`](TouchInputArgs::hits).
        pub target: InteractionPath,

        /// Current pointer capture.
        pub capture: Option<CaptureInfo>,

        /// What modifier keys where pressed when this event happened.
        pub modifiers: ModifiersState,

        ..

        /// The [`target`] and [`capture`].
        ///
        /// [`target`]: Self::target
        /// [`capture`]: Self::capture
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.insert_path(&self.target);
            if let Some(c) = &self.capture {
                list.insert_path(&c.target);
            }
        }
    }
}

impl TouchMoveArgs {
    /// If [`capture`] is `None` or [`allows`] the [`WIDGET`] to receive this event.
    ///
    /// [`capture`]: Self::capture
    /// [`allows`]: CaptureInfo::allows
    pub fn capture_allows(&self) -> bool {
        self.capture.as_ref().map(|c| c.allows()).unwrap_or(true)
    }
}

impl TouchInputArgs {
    /// If [`capture`] is `None` or [`allows`] the [`WIDGET`] to receive this event.
    ///
    /// [`capture`]: Self::capture
    /// [`allows`]: CaptureInfo::allows
    pub fn capture_allows(&self) -> bool {
        self.capture.as_ref().map(|c| c.allows()).unwrap_or(true)
    }

    /// If the `widget_id` is in the [`target`] is enabled.
    ///
    /// [`target`]: Self::target
    pub fn is_enabled(&self, widget_id: WidgetId) -> bool {
        self.target.interactivity_of(widget_id).map(|i| i.is_enabled()).unwrap_or(false)
    }

    /// If the `widget_id` is in the [`target`] is disabled.
    ///
    /// [`target`]: Self::target
    pub fn is_disabled(&self, widget_id: WidgetId) -> bool {
        self.target.interactivity_of(widget_id).map(|i| i.is_disabled()).unwrap_or(false)
    }

    /// If the [`phase`] is start.
    ///
    /// [`phase`]: Self::phase
    pub fn is_touch_start(&self) -> bool {
        matches!(self.phase, TouchPhase::Start)
    }

    /// If the [`phase`] is start.
    ///
    /// [`phase`]: Self::phase
    pub fn is_touch_end(&self) -> bool {
        matches!(self.phase, TouchPhase::End)
    }

    /// If the [`phase`] is start.
    ///
    /// [`phase`]: Self::phase
    pub fn is_touch_cancel(&self) -> bool {
        matches!(self.phase, TouchPhase::Cancel)
    }
}

impl TouchTapArgs {
    /// If [`capture`] is `None` or [`allows`] the [`WIDGET`] to receive this event.
    ///
    /// [`capture`]: Self::capture
    /// [`allows`]: CaptureInfo::allows
    pub fn capture_allows(&self) -> bool {
        self.capture.as_ref().map(|c| c.allows()).unwrap_or(true)
    }

    /// If the `widget_id` is in the [`target`] is enabled.
    ///
    /// [`target`]: Self::target
    pub fn is_enabled(&self, widget_id: WidgetId) -> bool {
        self.target.interactivity_of(widget_id).map(|i| i.is_enabled()).unwrap_or(false)
    }

    /// If the `widget_id` is in the [`target`] is disabled.
    ///
    /// [`target`]: Self::target
    pub fn is_disabled(&self, widget_id: WidgetId) -> bool {
        self.target.interactivity_of(widget_id).map(|i| i.is_disabled()).unwrap_or(false)
    }
}

impl TouchedArgs {
    /// If [`capture`] is `None` or [`allows`] the [`WIDGET`] to receive this event.
    ///
    /// [`capture`]: Self::capture
    /// [`allows`]: CaptureInfo::allows
    pub fn capture_allows(&self) -> bool {
        self.capture.as_ref().map(|c| c.allows()).unwrap_or(true)
    }

    /// Event caused by the touch position moving over/out of the widget bounds.
    pub fn is_touch_move(&self) -> bool {
        self.device_id.is_some()
    }

    /// Event caused by the widget moving under/out of the mouse position.
    pub fn is_widget_move(&self) -> bool {
        self.device_id.is_none()
    }

    /// Event caused by a pointer capture change.
    pub fn is_capture_change(&self) -> bool {
        self.prev_capture != self.capture
    }

    /// Returns `true` if the [`WIDGET`] was not touched, but now is.
    pub fn is_touch_enter(&self) -> bool {
        !self.was_touched() && self.is_touched()
    }

    /// Returns `true` if the [`WIDGET`] was touched, but now isn't.
    pub fn is_touch_leave(&self) -> bool {
        self.was_touched() && !self.is_touched()
    }

    /// Returns `true` if the [`WIDGET`] was not touched or was disabled, but now is touched and enabled.
    pub fn is_touch_enter_enabled(&self) -> bool {
        (!self.was_touched() || self.was_disabled(WIDGET.id())) && self.is_touched() && self.is_enabled(WIDGET.id())
    }

    /// Returns `true` if the [`WIDGET`] was touched and enabled, but now is not touched or is disabled.
    pub fn is_touch_leave_enabled(&self) -> bool {
        self.was_touched() && self.was_enabled(WIDGET.id()) && (!self.is_touched() || self.is_disabled(WIDGET.id()))
    }

    /// Returns `true` if the [`WIDGET`] was not touched or was enabled, but now is touched and disabled.
    pub fn is_touch_enter_disabled(&self) -> bool {
        (!self.was_touched() || self.was_enabled(WIDGET.id())) && self.is_touched() && self.is_disabled(WIDGET.id())
    }

    /// Returns `true` if the [`WIDGET`] was touched and disabled, but now is not touched or is enabled.
    pub fn is_touch_leave_disabled(&self) -> bool {
        self.was_touched() && self.was_disabled(WIDGET.id()) && (!self.is_touched() || self.is_enabled(WIDGET.id()))
    }

    /// Returns `true` if the [`WIDGET`] is in [`prev_target`] and is allowed by the [`prev_capture`].
    ///
    /// [`prev_target`]: Self::prev_target
    /// [`prev_capture`]: Self::prev_capture
    pub fn was_touched(&self) -> bool {
        if let Some(cap) = &self.prev_capture {
            if !cap.allows() {
                return false;
            }
        }

        if let Some(t) = &self.prev_target {
            return t.contains(WIDGET.id());
        }

        false
    }

    /// Returns `true` if the [`WIDGET`] is in [`target`] and is allowed by the current [`capture`].
    ///
    /// [`target`]: Self::target
    /// [`capture`]: Self::capture
    pub fn is_touched(&self) -> bool {
        if let Some(cap) = &self.capture {
            if !cap.allows() {
                return false;
            }
        }

        if let Some(t) = &self.target {
            return t.contains(WIDGET.id());
        }

        false
    }

    /// Returns `true` if the widget was enabled in [`prev_target`].
    ///
    /// [`prev_target`]: Self::prev_target
    pub fn was_enabled(&self, widget_id: WidgetId) -> bool {
        self.prev_target
            .as_ref()
            .and_then(|t| t.interactivity_of(widget_id))
            .map(|itr| itr.is_enabled())
            .unwrap_or(false)
    }

    /// Returns `true` if the widget was disabled in [`prev_target`].
    ///
    /// [`prev_target`]: Self::prev_target
    pub fn was_disabled(&self, widget_id: WidgetId) -> bool {
        self.prev_target
            .as_ref()
            .and_then(|t| t.interactivity_of(widget_id))
            .map(|itr| itr.is_disabled())
            .unwrap_or(false)
    }

    /// Returns `true` if the widget is enabled in [`target`].
    ///
    /// [`target`]: Self::target
    pub fn is_enabled(&self, widget_id: WidgetId) -> bool {
        self.target
            .as_ref()
            .and_then(|t| t.interactivity_of(widget_id))
            .map(|itr| itr.is_enabled())
            .unwrap_or(false)
    }

    /// Returns `true` if the widget is disabled in [`target`].
    ///
    /// [`target`]: Self::target
    pub fn is_disabled(&self, widget_id: WidgetId) -> bool {
        self.target
            .as_ref()
            .and_then(|t| t.interactivity_of(widget_id))
            .map(|itr| itr.is_disabled())
            .unwrap_or(false)
    }
}

event! {
    /// Touch contact moved.
    pub static TOUCH_MOVE_EVENT: TouchMoveArgs;

    /// Touch contact started or ended.
    pub static TOUCH_INPUT_EVENT: TouchInputArgs;

    /// Touch made first contact or lost contact with a widget.
    pub static TOUCHED_EVENT: TouchedArgs;

    /// Touch tap.
    ///
    /// This is a touch gesture event, it only notifies if it has listeners, either widget subscribers in the
    /// touched path or app level hooks.
    pub static TOUCH_TAP_EVENT: TouchTapArgs;
}

impl AppExtension for TouchManager {
    fn event_preview(&mut self, update: &mut EventUpdate) {
        if let Some(args) = RAW_FRAME_RENDERED_EVENT.on(update) {
            self.continue_pressed(args.window_id);
        } else if let Some(args) = RAW_TOUCH_EVENT.on(update) {
            let mut pending_move: Vec<TouchMove> = vec![];

            for u in &args.touches {
                if let TouchPhase::Move = u.phase {
                    if let Some(e) = pending_move.iter_mut().find(|e| e.touch == u.touch) {
                        e.moves.push((u.position, u.force));
                    } else {
                        pending_move.push(TouchMove {
                            touch: u.touch,
                            touch_propagation: if let Some(i) = self.pressed.get(&u.touch) {
                                i.touch_propagation.clone()
                            } else {
                                let weird = EventPropagationHandle::new();
                                weird.stop();
                                weird
                            },
                            moves: vec![(u.position, u.force)],
                            hits: HitTestInfo::no_hits(args.window_id), // hit-test deferred
                            target: InteractionPath::new(args.window_id, []),
                        })
                    }
                } else {
                    self.on_move(args, mem::take(&mut pending_move));
                    self.on_input(args, u);
                }
            }

            self.on_move(args, pending_move);
        } else if let Some(args) = WIDGET_INFO_CHANGED_EVENT.on(update) {
            self.continue_pressed(args.window_id);
        } else if let Some(args) = MODIFIERS_CHANGED_EVENT.on(update) {
            self.modifiers = args.modifiers;
        } else if let Some(args) = RAW_TOUCH_CONFIG_CHANGED_EVENT.on(update) {
            TOUCH_SV.read().touch_config.set(args.config);
        } else if let Some(args) = view_process::VIEW_PROCESS_INITED_EVENT.on(update) {
            TOUCH_SV.read().touch_config.set(args.touch_config);

            if args.is_respawn {
                self.tap_start = None;

                for (touch, info) in self.pressed.drain() {
                    let args = TouchInputArgs::now(
                        info.target.window_id(),
                        info.device_id,
                        touch,
                        info.touch_propagation.clone(),
                        DipPoint::splat(Dip::new(-1)),
                        None,
                        TouchPhase::Cancel,
                        HitTestInfo::no_hits(info.target.window_id()),
                        info.target.clone(),
                        None,
                        ModifiersState::empty(),
                    );
                    TOUCH_INPUT_EVENT.notify(args);

                    let args = TouchedArgs::now(
                        info.target.window_id(),
                        info.device_id,
                        touch,
                        info.touch_propagation,
                        DipPoint::splat(Dip::new(-1)),
                        None,
                        TouchPhase::Cancel,
                        HitTestInfo::no_hits(info.target.window_id()),
                        info.target,
                        None,
                        None,
                        None,
                    );
                    TOUCHED_EVENT.notify(args);
                }
            }
        }
    }

    fn event(&mut self, update: &mut EventUpdate) {
        if let Some(args) = TOUCH_INPUT_EVENT.on(update) {
            if let Some(s) = self.tap_start.take() {
                if let Ok(w) = WINDOWS.widget_tree(args.window_id) {
                    s.try_complete(args, &w);
                } else {
                    self.tap_start = None;
                }
            } else if TOUCH_TAP_EVENT.has_hooks() || args.target.widgets_path().iter().any(|w| TOUCH_TAP_EVENT.is_subscriber(*w)) {
                self.tap_start = TapStart::try_start(args);
            }
        } else if let Some(args) = TOUCH_MOVE_EVENT.on(update) {
            if let Some(s) = &self.tap_start {
                for t in &args.touches {
                    if !s.retain(args.window_id, args.device_id, t.touch) {
                        self.tap_start = None;
                        break;
                    }
                }
            }
        } else if let Some(args) = POINTER_CAPTURE_EVENT.on(update) {
            for (touch, info) in &self.pressed {
                let args = TouchedArgs::now(
                    info.target.window_id(),
                    info.device_id,
                    *touch,
                    info.touch_propagation.clone(),
                    info.position,
                    info.force,
                    TouchPhase::Move,
                    info.hits.clone(),
                    info.target.clone(),
                    info.target.clone(),
                    args.prev_capture.clone(),
                    args.new_capture.clone(),
                );
                TOUCHED_EVENT.notify(args);
            }
        }
    }
}
impl TouchManager {
    fn on_input(&mut self, args: &RawTouchArgs, update: &TouchUpdate) {
        if let Ok(w) = WINDOWS.widget_tree(args.window_id) {
            let hits = w.root().hit_test(update.position.to_px(w.scale_factor().0));
            let target = hits
                .target()
                .and_then(|t| w.get(t.widget_id))
                .map(|t| t.interaction_path())
                .unwrap_or_else(|| w.root().interaction_path());

            let target = match target.unblocked() {
                Some(t) => t,
                None => return, // entire window blocked
            };

            let capture_info = POINTER_CAPTURE.current_capture_value();

            let gesture_handle = match update.phase {
                TouchPhase::Start => {
                    let handle = EventPropagationHandle::new();
                    if let Some(weird) = self.pressed.insert(
                        update.touch,
                        PressedInfo {
                            touch_propagation: handle.clone(),
                            target: target.clone(),
                            device_id: args.device_id,
                            position: update.position,
                            force: update.force,
                            hits: hits.clone(),
                        },
                    ) {
                        weird.touch_propagation.stop();
                    }
                    handle
                }
                TouchPhase::End => {
                    if let Some(handle) = self.pressed.remove(&update.touch) {
                        handle.touch_propagation
                    } else {
                        let weird = EventPropagationHandle::new();
                        weird.stop();
                        weird
                    }
                }
                TouchPhase::Cancel => {
                    let handle = self
                        .pressed
                        .remove(&update.touch)
                        .map(|i| i.touch_propagation)
                        .unwrap_or_else(EventPropagationHandle::new);
                    handle.stop();
                    handle
                }
                TouchPhase::Move => unreachable!(),
            };

            let args = TouchInputArgs::now(
                args.window_id,
                args.device_id,
                update.touch,
                gesture_handle,
                update.position,
                update.force,
                update.phase,
                hits,
                target,
                capture_info,
                self.modifiers,
            );

            let touched_args = {
                // touched

                let (prev_target, target) = match args.phase {
                    TouchPhase::Start => (None, Some(args.target.clone())),
                    TouchPhase::End | TouchPhase::Cancel => (Some(args.target.clone()), None),
                    TouchPhase::Move => unreachable!(),
                };

                TouchedArgs::now(
                    args.window_id,
                    args.device_id,
                    args.touch,
                    args.touch_propagation.clone(),
                    args.position,
                    args.force,
                    args.phase,
                    args.hits.clone(),
                    prev_target,
                    target,
                    args.capture.clone(),
                    args.capture.clone(),
                )
            };

            TOUCH_INPUT_EVENT.notify(args);
            TOUCHED_EVENT.notify(touched_args);
        } else {
            // did not find window, cleanup touched
            for u in &args.touches {
                if let Some(i) = self.pressed.remove(&u.touch) {
                    let capture = POINTER_CAPTURE.current_capture_value();
                    let args = TouchedArgs::now(
                        args.window_id,
                        args.device_id,
                        u.touch,
                        i.touch_propagation,
                        u.position,
                        u.force,
                        u.phase,
                        HitTestInfo::no_hits(args.window_id),
                        Some(i.target),
                        None,
                        capture.clone(),
                        capture,
                    );
                    TOUCHED_EVENT.notify(args);
                }
            }
        }
    }

    fn on_move(&mut self, args: &RawTouchArgs, mut moves: Vec<TouchMove>) {
        if !moves.is_empty() {
            if let Ok(w) = WINDOWS.widget_tree(args.window_id) {
                let mut window_blocked_remove = vec![];
                for m in &mut moves {
                    m.hits = w.root().hit_test(m.position().to_px(w.scale_factor().0));
                    let target = m
                        .hits
                        .target()
                        .and_then(|t| w.get(t.widget_id))
                        .map(|t| t.interaction_path())
                        .unwrap_or_else(|| w.root().interaction_path());

                    match target.unblocked() {
                        Some(t) => m.target = t,
                        None => {
                            window_blocked_remove.push(m.touch);
                        }
                    }
                }

                let capture_info = POINTER_CAPTURE.current_capture_value();

                let mut touched_events = vec![];

                for touch in window_blocked_remove {
                    let touch_move = moves.iter().position(|t| t.touch == touch).unwrap();
                    moves.swap_remove(touch_move);

                    if let Some(i) = self.pressed.remove(&touch) {
                        i.touch_propagation.stop();
                        let args = TouchedArgs::now(
                            args.window_id,
                            args.device_id,
                            touch,
                            i.touch_propagation,
                            DipPoint::splat(Dip::new(-1)),
                            None,
                            TouchPhase::Cancel,
                            HitTestInfo::no_hits(args.window_id),
                            i.target,
                            None,
                            None,
                            None,
                        );
                        touched_events.push(args);
                    }
                }
                for m in &moves {
                    if let Some(i) = self.pressed.get_mut(&m.touch) {
                        let (position, force) = *m.moves.last().unwrap();
                        i.position = position;
                        i.force = force;
                        i.hits = m.hits.clone();
                        if i.target != m.target {
                            let args = TouchedArgs::now(
                                args.window_id,
                                args.device_id,
                                m.touch,
                                m.touch_propagation.clone(),
                                position,
                                force,
                                TouchPhase::Move,
                                m.hits.clone(),
                                i.target.clone(),
                                m.target.clone(),
                                capture_info.clone(),
                                capture_info.clone(),
                            );
                            i.target = m.target.clone();
                            touched_events.push(args);
                        }
                    }
                }

                if !moves.is_empty() {
                    let args = TouchMoveArgs::now(args.window_id, args.device_id, moves, capture_info, self.modifiers);
                    TOUCH_MOVE_EVENT.notify(args);
                }

                for args in touched_events {
                    TOUCHED_EVENT.notify(args);
                }
            }
        }
    }

    fn continue_pressed(&mut self, window_id: WindowId) {
        let mut tree = None;

        let mut window_blocked_remove = vec![];

        for (touch, info) in &mut self.pressed {
            if info.target.window_id() != window_id {
                continue;
            }

            let tree = tree.get_or_insert_with(|| WINDOWS.widget_tree(window_id).unwrap());
            info.hits = tree.root().hit_test(info.position.to_px(tree.scale_factor().0));

            let target = if let Some(t) = info.hits.target() {
                tree.get(t.widget_id).map(|w| w.interaction_path()).unwrap_or_else(|| {
                    tracing::error!("hits target `{}` not found", t.widget_id);
                    tree.root().interaction_path()
                })
            } else {
                tree.root().interaction_path()
            }
            .unblocked();

            if let Some(target) = target {
                if info.target != target {
                    let capture = POINTER_CAPTURE.current_capture_value();
                    let prev = mem::replace(&mut info.target, target.clone());

                    let args = TouchedArgs::now(
                        info.target.window_id(),
                        None,
                        *touch,
                        info.touch_propagation.clone(),
                        info.position,
                        info.force,
                        TouchPhase::Move,
                        info.hits.clone(),
                        prev,
                        target,
                        capture.clone(),
                        capture,
                    );
                    TOUCHED_EVENT.notify(args);
                }
            } else {
                window_blocked_remove.push(*touch);
            }
        }

        for touch in window_blocked_remove {
            if let Some(i) = self.pressed.remove(&touch) {
                i.touch_propagation.stop();
                let args = TouchedArgs::now(
                    i.target.window_id(),
                    None,
                    touch,
                    i.touch_propagation,
                    DipPoint::splat(Dip::new(-1)),
                    None,
                    TouchPhase::Cancel,
                    HitTestInfo::no_hits(i.target.window_id()),
                    i.target,
                    None,
                    None,
                    None,
                );
                TOUCHED_EVENT.notify(args);
            }
        }
    }
}

struct TapStart {
    window_id: WindowId,
    device_id: DeviceId,
    touch: TouchId,
    target: WidgetId,

    propagation: EventPropagationHandle,
}
impl TapStart {
    /// Returns `Some(_)` if args could be the start of a tap event.
    fn try_start(args: &TouchInputArgs) -> Option<Self> {
        if let TouchPhase::Start = args.phase {
            Some(Self {
                window_id: args.window_id,
                device_id: args.device_id,
                touch: args.touch,
                target: args.target.widget_id(),
                propagation: args.touch_propagation.clone(),
            })
        } else {
            None
        }
    }

    /// Check if the tap is still possible after a touch move..
    ///
    /// Returns `true` if it is.
    fn retain(&self, window_id: WindowId, device_id: DeviceId, touch: TouchId) -> bool {
        if self.propagation.is_stopped() {
            // cancel, gesture opportunity handled.
            return false;
        }

        if window_id != self.window_id || device_id != self.device_id {
            // cancel, not same source or target.
            return false;
        }

        if touch != self.touch {
            // cancel, multi-touch.
            return false;
        }

        // retain
        true
    }

    /// Complete or cancel the tap.
    fn try_complete(self, args: &TouchInputArgs, tree: &WidgetInfoTree) {
        if !self.retain(args.window_id, args.device_id, args.touch) {
            return;
        }

        match tree.get(self.target) {
            Some(t) => {
                if !t.hit_test(args.position.to_px(tree.scale_factor().0)).contains(self.target) {
                    // cancel, touch did not end over target.
                    return;
                }
            }
            None => return,
        };

        if let TouchPhase::End = args.phase {
            self.propagation.stop(); // touch_propagation always is stopped after touch end.

            if let Some(target) = args.target.sub_path(self.target) {
                TOUCH_TAP_EVENT.notify(TouchTapArgs::new(
                    args.timestamp,
                    args.propagation().clone(),
                    self.window_id,
                    self.device_id,
                    self.touch,
                    args.position,
                    args.hits.clone(),
                    target.into_owned(),
                    args.capture.clone(),
                    args.modifiers,
                ));
            }
        } else if let TouchPhase::Cancel = args.phase {
            self.propagation.stop();
        }
    }
}

/// Info useful for touch gestures computed from two touch points.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Touch2Info {
    /// The two touch contact points.
    pub touches: [euclid::Point2D<f32, Px>; 2],

    /// Middle of the line between the two points.
    pub center: euclid::Point2D<f32, Px>,

    /// Average deviation from the two points to the center.
    ///
    /// Min 1.0
    pub deviation: f32,

    /// Average deviation from the two points.x to the center.x.
    ///
    /// Min 1.0
    pub deviation_x: f32,

    /// Average deviation from the two points.y to the center.y.
    ///
    /// Min 1.0
    pub deviation_y: f32,

    /// Angle of the line.
    pub angle: AngleRadian,
}
impl Touch2Info {
    /// Compute the line info.
    pub fn new_f32(touches: [euclid::Point2D<f32, Px>; 2]) -> Self {
        let a = touches[0].to_vector();
        let b = touches[1].to_vector();

        let center = (a + b) / 2.0;
        let deviation = ((a - center).length() + (b - center).length()) / 2.0;
        let deviation_x = ((a.x - center.x).abs() + (b.x - center.x).abs()) / 2.0;
        let deviation_y = ((a.y - center.y).abs() + (b.y - center.y).abs()) / 2.0;

        Self {
            touches,
            center: center.to_point(),
            deviation: deviation.max(1.0),
            deviation_x: deviation_x.max(1.0),
            deviation_y: deviation_y.max(1.0),
            angle: AngleRadian((a.x - b.x).atan2(a.y - b.y)),
        }
    }

    /// Compute the line info, from round pixels.
    pub fn new(touches: [PxPoint; 2]) -> Self {
        Self::new_f32([touches[0].to_f32(), touches[1].to_f32()])
    }

    /// Compute the line info, from device independent pixels.
    pub fn new_dip(touches: [DipPoint; 2], scale_factor: Factor) -> Self {
        Self::new_f32([touches[0].to_f32().to_px(scale_factor.0), touches[1].to_f32().to_px(scale_factor.0)])
    }
}
impl Touch2Info {
    /// Computes the translation to transform from `self` to `other`.
    pub fn translation(&self, other: &Self) -> euclid::Vector2D<f32, Px> {
        other.center.to_vector() - self.center.to_vector()
    }

    /// Computes the translation-x to transform from `self` to `other`.
    pub fn translation_x(&self, other: &Self) -> f32 {
        other.center.x - self.center.x
    }

    /// Computes the translation-y to transform from `self` to `other`.
    pub fn translation_y(&self, other: &Self) -> f32 {
        other.center.y - self.center.y
    }

    /// Computes the rotation to transform from `self` to `other`.
    pub fn rotation(&self, other: &Self) -> AngleRadian {
        other.angle - self.angle
    }

    /// Computes the scale to transform from `self` to `other`.
    pub fn scale(&self, other: &Self) -> Factor {
        Factor(other.deviation / self.deviation)
    }

    /// Computes the scale-y to transform from `self` to `other`.
    pub fn scale_x(&self, other: &Self) -> Factor {
        Factor(other.deviation_x / self.deviation_x)
    }

    /// Computes the scale-y to transform from `self` to `other`.
    pub fn scale_y(&self, other: &Self) -> Factor {
        Factor(other.deviation_y / self.deviation_y)
    }

    /// Computes the transform from `self` to `other`.
    pub fn transform(&self, other: &Self, mode: TouchTransformMode) -> PxTransform {
        let mut m = PxTransform::identity();

        if mode.contains(TouchTransformMode::TRANSLATE) {
            m = m.then_translate(self.translation(other));
        } else if mode.contains(TouchTransformMode::TRANSLATE_X) {
            let t = euclid::vec2(self.translation_x(other), 0.0);
            m = m.then_translate(t);
        } else if mode.contains(TouchTransformMode::TRANSLATE_Y) {
            let t = euclid::vec2(0.0, self.translation_y(other));
            m = m.then_translate(t);
        }

        if mode.contains(TouchTransformMode::SCALE) {
            let s = self.scale(other).0;
            m = m.then(&PxTransform::scale(s, s));
        } else if mode.contains(TouchTransformMode::SCALE_X) {
            let s = self.scale_x(other);
            m = m.then(&PxTransform::scale(s.0, 1.0))
        } else if mode.contains(TouchTransformMode::SCALE_Y) {
            let s = self.scale_y(other);
            m = m.then(&PxTransform::scale(1.0, s.0))
        }

        if mode.contains(TouchTransformMode::ROTATE) {
            let a = self.rotation(other);
            m = m.then(&PxTransform::rotation(0.0, 0.0, a.layout()));
        }

        m
    }
}

bitflags! {
    /// Defines the different transforms that a [`TouchTransform`] can do to keep
    /// two touch points in a widget aligned with the touch contacts.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
    #[serde(transparent)]
    pub struct TouchTransformMode: u8 {
        /// Translate in the X dimension.
        const TRANSLATE_X = 0b0000_0001;
        /// Translate in the y dimension.
        const TRANSLATE_Y = 0b0000_0010;
        /// Translate in both dimensions.
        const TRANSLATE = Self::TRANSLATE_X.bits() | Self::TRANSLATE_Y.bits();

        /// Scale in the X dimension.
        const SCALE_X = 0b0000_0100;
        /// Scale in the Y dimension.
        const SCALE_Y = 0b0000_1000;
        /// Scale in both dimensions the same amount.
        const SCALE = 0b0001_1100;

        /// Rotate.
        const ROTATE = 0b0010_0000;

        /// Translate, scale-square and rotate.
        const ALL = Self::TRANSLATE.bits()| Self::SCALE.bits() | Self::ROTATE.bits();
    }
}
