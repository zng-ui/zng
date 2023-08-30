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
    pointer_capture::{CaptureInfo, POINTER_CAPTURE},
    units::*,
    var::*,
    widget_info::{HitTestInfo, InteractionPath, WidgetInfoTree},
    widget_instance::WidgetId,
    window::{WindowId, WINDOWS},
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
    tap_start: Option<TapStart>,
    modifiers: ModifiersState,
    gesture_handles: HashMap<TouchId, EventPropagationHandle>,
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
    /// Identify a the touch contact or *finger*.
    ///
    /// Multiple points of contact can happen in the same device at the same time,
    /// this ID identifies each uninterrupted contact. IDs are unique only among other concurrent touches
    /// on the same device, after a touch is ended an ID may be reused.
    pub touch: TouchId,

    /// Handle across the lifetime of `touch`.
    ///
    /// See [`TouchInputArgs::gesture_propagation`] for more details.
    pub gesture_propagation: EventPropagationHandle,

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

        /// Current touch capture.
        pub capture: Option<CaptureInfo>,

        /// What modifier keys where pressed when this event happened.
        pub modifiers: ModifiersState,

        ..

        /// Each [`TouchMove::target`] and [`capture`].
        ///
        /// [`target`]: Self::target
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

        /// Identify a the touch contact or *finger*.
        ///
        /// Multiple points of contact can happen in the same device at the same time,
        /// this ID identifies each uninterrupted contact. IDs are unique only among other concurrent touches
        /// on the same device, after a touch is ended an ID may be reused.
        pub touch: TouchId,

        /// Signals if a touch gesture observer consumed the [`touch`].
        ///
        /// The [`TOUCH_INPUT_EVENT`] and [`TOUCH_MOVE_EVENT`] have their own separate propagation handles, but
        /// touch gesture events aggregate all these events to produce a single *gesture event*, usually only a single
        /// gesture should be generated, multiple gestures can disambiguate using this `gesture_propagation` handle.
        ///
        /// [`touch`]: Self::touch
        pub gesture_propagation: EventPropagationHandle,

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

        /// Current touch capture.
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

        /// Identify a the touch contact or *finger*.
        ///
        /// Multiple points of contact can happen in the same device at the same time,
        /// this ID identifies each uninterrupted contact. IDs are unique only among other concurrent touches
        /// on the same device, after a touch is ended an ID may be reused.
        pub touch: TouchId,

        /// Signals if a touch gesture observer consumed the [`touch`].
        ///
        /// The [`TOUCH_INPUT_EVENT`] and [`TOUCH_MOVE_EVENT`] have their own separate propagation handles, but
        /// touch gesture events aggregate all these events to produce a single *gesture event*, usually only a single
        /// gesture should be generated, multiple gestures can disambiguate using this `gesture_propagation` handle.
        ///
        /// [`touch`]: Self::touch
        pub gesture_propagation: EventPropagationHandle,

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

        /// Previous touch capture.
        pub prev_capture: Option<CaptureInfo>,

        /// Current touch capture.
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

        /// Identify a the touch contact or *finger*.
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

        /// Current touch capture.
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

event! {
    /// Touch contact moved.
    pub static TOUCH_MOVE_EVENT: TouchMoveArgs;

    /// Touch contact started or ended.
    pub static TOUCH_INPUT_EVENT: TouchInputArgs;

    /// Touch made first contact or lost contact with a widget.
    pub static TOUCHED_EVENT: TouchedArgs;

    /// Touch tap.
    pub static TOUCH_TAP_EVENT: TouchTapArgs;
}

impl AppExtension for TouchManager {
    fn event_preview(&mut self, update: &mut EventUpdate) {
        if let Some(args) = RAW_TOUCH_EVENT.on(update) {
            let mut pending_move: Vec<TouchMove> = vec![];

            for u in &args.touches {
                if let TouchPhase::Move = u.phase {
                    if let Some(e) = pending_move.iter_mut().find(|e| e.touch == u.touch) {
                        e.moves.push((u.position, u.force));
                    } else {
                        pending_move.push(TouchMove {
                            touch: u.touch,
                            gesture_propagation: if let Some(handle) = self.gesture_handles.get(&u.touch) {
                                handle.clone()
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
        } else if let Some(args) = MODIFIERS_CHANGED_EVENT.on(update) {
            self.modifiers = args.modifiers;
        } else if let Some(args) = RAW_TOUCH_CONFIG_CHANGED_EVENT.on(update) {
            TOUCH_SV.read().touch_config.set(args.config);
        } else if let Some(args) = view_process::VIEW_PROCESS_INITED_EVENT.on(update) {
            TOUCH_SV.read().touch_config.set(args.touch_config);

            if args.is_respawn {
                self.tap_start = None;
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

            let capture_info = POINTER_CAPTURE.current_capture_value();

            let gesture_handle = match update.phase {
                TouchPhase::Start => {
                    let handle = EventPropagationHandle::new();
                    if let Some(weird) = self.gesture_handles.insert(update.touch, handle.clone()) {
                        weird.stop();
                    }
                    handle
                }
                TouchPhase::End => {
                    if let Some(handle) = self.gesture_handles.remove(&update.touch) {
                        handle
                    } else {
                        let weird = EventPropagationHandle::new();
                        weird.stop();
                        weird
                    }
                }
                TouchPhase::Cancel => {
                    let handle = self
                        .gesture_handles
                        .remove(&update.touch)
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

            if let Some(s) = self.tap_start.take() {
                s.try_complete(&args, update, &w);
            } else {
                self.tap_start = TapStart::try_start(&args, update);
            }

            TOUCH_INPUT_EVENT.notify(args);
        }
    }

    fn on_move(&mut self, args: &RawTouchArgs, mut moves: Vec<TouchMove>) {
        if !moves.is_empty() {
            if let Ok(w) = WINDOWS.widget_tree(args.window_id) {
                for m in &mut moves {
                    m.hits = w.root().hit_test(m.position().to_px(w.scale_factor().0));
                    m.target = m
                        .hits
                        .target()
                        .and_then(|t| w.get(t.widget_id))
                        .map(|t| t.interaction_path())
                        .unwrap_or_else(|| w.root().interaction_path());
                }

                let capture_info = POINTER_CAPTURE.current_capture_value();

                if let Some(s) = &self.tap_start {
                    for m in &moves {
                        if !s.retain(args.window_id, args.device_id, m.touch) {
                            self.tap_start = None;
                            break;
                        }
                    }
                }

                let args = TouchMoveArgs::now(args.window_id, args.device_id, moves, capture_info, self.modifiers);
                TOUCH_MOVE_EVENT.notify(args);
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
    fn try_start(args: &TouchInputArgs, update: &TouchUpdate) -> Option<Self> {
        if let TouchPhase::Start = update.phase {
            Some(Self {
                window_id: args.window_id,
                device_id: args.device_id,
                touch: update.touch,
                target: args.target.widget_id(),
                propagation: args.gesture_propagation.clone(),
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
    fn try_complete(self, args: &TouchInputArgs, update: &TouchUpdate, tree: &WidgetInfoTree) {
        if !self.retain(args.window_id, args.device_id, update.touch) {
            return;
        }

        match tree.get(self.target) {
            Some(t) => {
                if !t.hit_test(update.position.to_px(tree.scale_factor().0)).contains(self.target) {
                    // cancel, touch did not end over target.
                    return;
                }
            }
            None => return,
        };

        if let TouchPhase::End = update.phase {
            TOUCH_TAP_EVENT.notify(TouchTapArgs::new(
                args.timestamp,
                args.propagation().clone(),
                self.window_id,
                self.device_id,
                self.touch,
                update.position,
                args.hits.clone(),
                args.target.clone(),
                args.capture.clone(),
                args.modifiers,
            ));
        }
    }
}
