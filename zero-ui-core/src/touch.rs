//! Touch events and service.
//!
//! The app extension [`TouchManager`] provides the events and service. It is included in the default application.

use std::mem;

pub use zero_ui_view_api::{TouchForce, TouchId, TouchPhase, TouchUpdate};

use crate::{
    app::{raw_events::*, *},
    event::*,
    units::*,
    widget_info::{HitTestInfo, InteractionPath},
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
/// TODO
///
/// # Provider
///
/// This service is provided by the [`TouchManager`] extension.
pub struct TOUCH;

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

        /// TODO
        pub capture: (),
        ..

        /// The [`target`] and [`capture`].
        ///
        /// [`target`]: Self::target
        /// [`capture`]: Self::capture
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.insert_path(&self.target);
            // if let Some(c) = &self.capture {
            //     list.insert_path(&c.target);
            // }
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

        /// TODO
        pub capture: (),

        ..

        /// The [`target`] and [`capture`].
        ///
        /// [`target`]: Self::target
        /// [`capture`]: Self::capture
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.insert_path(&self.target);
            // if let Some(c) = &self.capture {
            //     list.insert_path(&c.target);
            // }
        }
    }
}

event! {
    /// Touch contact moved.
    pub static TOUCH_MOVE_EVENT: TouchMoveArgs;

    /// Touch contact started or ended.
    pub static TOUCH_INPUT_EVENT: TouchInputArgs;
}

impl AppExtension for TouchManager {
    fn event_preview(&mut self, update: &mut EventUpdate) {
        if let Some(args) = RAW_TOUCH_EVENT.on(update) {
            let mut pending_move = vec![];

            for u in &args.touches {
                if let TouchPhase::Moved = u.phase {
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

            let args = TouchInputArgs::now(
                args.window_id,
                args.device_id,
                update.touch,
                update.position,
                update.force,
                update.phase,
                hits,
                target,
                (),
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

                let args = TouchMoveArgs::now(args.window_id, args.device_id, moves, touch, position, force, hits, target, ());
                TOUCH_MOVE_EVENT.notify(args);
            }
        }
    }
}
