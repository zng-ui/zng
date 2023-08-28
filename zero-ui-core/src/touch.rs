//! Touch events and service.
//!
//! The app extension [`TouchManager`] provides the events and service. It is included in the default application.

use std::mem;

pub use zero_ui_view_api::{TouchForce, TouchId, TouchPhase, TouchUpdate};

use crate::{
    app::{raw_events::*, *},
    context::*,
    event::*,
    pointer_capture::{CaptureInfo, POINTER_CAPTURE},
    units::*,
    var::*,
    widget_info::{HitTestInfo, InteractionPath},
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
pub struct TouchManager {}

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
    /// All active touches.
    pub fn touches(&self) -> ReadOnlyArcVar<Vec<(TouchId, DipPoint, Option<TouchForce>)>> {
        todo!()
    }
}

event_args! {
    /// Arguments for [`TOUCH_MOVE_EVENT`].
    pub struct TouchMoveArgs {
        /// Id of window that received the event.
        pub window_id: WindowId,

        /// Id of device that generated the event.
        pub device_id: DeviceId,

        /// Positions and force of touch moves in between the previous event and this one.
        ///
        /// Touch move events can be coalesced, i.e. multiple moves packed into a single event.
        pub coalesced: Vec<(TouchId, DipPoint, Option<TouchForce>)>,

        /// Identify a the touch contact or *finger*.
        ///
        /// Multiple points of contact can happen in the same device at the same time,
        /// this ID identifies each uninterrupted contact. IDs are unique only among other concurrent touches
        /// on the same device, after a touch is ended an ID may be reused.
        pub touch: TouchId,

        /// Center of the touch in the window's content area.
        pub position: DipPoint,

        /// Touch pressure force and angle.
        pub force: Option<TouchForce>,

        /// Hit-test result for the touch point in the window.
        pub hits: HitTestInfo,

        /// Full path to the top-most hit in [`hits`](TouchMoveArgs::hits).
        pub target: InteractionPath,

        /// Current touch capture.
        pub capture: Option<CaptureInfo>,

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

    /// Touch tap.
    pub static TOUCH_TAP_EVENT: TouchTapArgs;
}

impl AppExtension for TouchManager {
    fn event_preview(&mut self, update: &mut EventUpdate) {
        if let Some(args) = RAW_TOUCH_EVENT.on(update) {
            let mut pending_move = vec![];

            for u in &args.touches {
                if let TouchPhase::Move = u.phase {
                    pending_move.push((u.touch, u.position, u.force));
                } else {
                    self.on_move(args, mem::take(&mut pending_move));
                    self.on_input(args, u);
                }
            }

            self.on_move(args, pending_move);
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

            let args = TouchInputArgs::now(
                args.window_id,
                args.device_id,
                update.touch,
                update.position,
                update.force,
                update.phase,
                hits,
                target,
                capture_info,
            );
            TOUCH_INPUT_EVENT.notify(args);
        }
    }

    fn on_move(&mut self, args: &RawTouchArgs, mut moves: Vec<(TouchId, DipPoint, Option<TouchForce>)>) {
        if let Some((touch, position, force)) = moves.pop() {
            if let Ok(w) = WINDOWS.widget_tree(args.window_id) {
                let hits = w.root().hit_test(position.to_px(w.scale_factor().0));
                let target = hits
                    .target()
                    .and_then(|t| w.get(t.widget_id))
                    .map(|t| t.interaction_path())
                    .unwrap_or_else(|| w.root().interaction_path());

                let capture_info = POINTER_CAPTURE.current_capture_value();

                let args = TouchMoveArgs::now(
                    args.window_id,
                    args.device_id,
                    moves,
                    touch,
                    position,
                    force,
                    hits,
                    target,
                    capture_info,
                );
                TOUCH_MOVE_EVENT.notify(args);
            }
        }
    }
}
