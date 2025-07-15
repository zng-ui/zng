#![cfg(feature = "drag_drop")]

//! Drag & drop gesture events and service.

use std::{mem, sync::Arc};

use parking_lot::Mutex;
use zng_app::{
    AppExtension,
    event::{AnyEventArgs, event, event_args},
    static_id,
    update::{EventUpdate, UPDATES},
    view_process::raw_events::{
        RAW_APP_DRAG_ENDED_EVENT, RAW_DRAG_CANCELLED_EVENT, RAW_DRAG_DROPPED_EVENT, RAW_DRAG_HOVERED_EVENT, RAW_DRAG_MOVED_EVENT,
    },
    widget::{
        WIDGET, WidgetId,
        info::{HitTestInfo, InteractionPath, WidgetInfo, WidgetInfoBuilder},
    },
    window::WindowId,
};
use zng_app_context::app_local;
use zng_ext_window::{NestedWindowWidgetInfoExt as _, WINDOWS, WINDOWS_DRAG_DROP};
use zng_handle::{Handle, HandleOwner, WeakHandle};
use zng_layout::unit::{DipPoint, DipToPx as _, PxToDip as _};
use zng_state_map::StateId;
use zng_txt::{Txt, formatx};
use zng_var::{ArcVar, ReadOnlyArcVar, Var, var};
use zng_view_api::{DragDropId, mouse::ButtonState, touch::TouchPhase};

use crate::{mouse::MOUSE_INPUT_EVENT, touch::TOUCH_INPUT_EVENT};

pub use zng_view_api::drag_drop::{DragDropData, DragDropEffect};

/// Application extension that provides drag&drop events and service.
///
/// # Events
///
/// Events this extension provides.
///
/// * [`DROP_EVENT`]
/// * [`DRAG_HOVERED_EVENT`]
/// * [`DRAG_MOVE_EVENT`]
/// * [`DRAG_START_EVENT`]
/// * [`DRAG_END_EVENT`]
/// * [`DROP_EVENT`]
///
/// # Services
///
/// Services this extension provides.
///
/// * [`DRAG_DROP`]
#[derive(Default)]
pub struct DragDropManager {
    // last cursor move position (scaled).
    pos: DipPoint,
    // last cursor move over `pos_window` and source device.
    pos_window: Option<WindowId>,
    // last cursor move hit-test (on the pos_window or a nested window).
    hits: Option<HitTestInfo>,
    hovered: Option<InteractionPath>,
}

impl AppExtension for DragDropManager {
    fn event_preview(&mut self, update: &mut EventUpdate) {
        let mut update_sv = false;
        if let Some(args) = RAW_DRAG_DROPPED_EVENT.on(update) {
            // system drop
            let mut sv = DRAG_DROP_SV.write();
            let len = sv.system_dragging.len();
            for data in &args.data {
                sv.system_dragging.retain(|d| d != data);
            }
            update_sv = len != sv.system_dragging.len();

            // view-process can notify multiple drops in sequence with the same ID, so we only notify DROP_EVENT
            // on he next update
            if self.pos_window == Some(args.window_id) {
                if let Some(hovered) = &self.hovered {
                    match &mut sv.pending_drop {
                        Some((id, target, data, allowed)) => {
                            if target != hovered {
                                tracing::error!("drop sequence across different hovered")
                            } else if *id != args.drop_id {
                                tracing::error!("drop_id changed mid sequence")
                            } else if *allowed != args.allowed {
                                tracing::error!("allowed effects changed mid sequence")
                            } else {
                                data.extend(args.data.iter().cloned());
                            }
                        }
                        None => sv.pending_drop = Some((args.drop_id, hovered.clone(), args.data.clone(), args.allowed)),
                    }
                }
            }
            UPDATES.update(None);
        } else if let Some(args) = RAW_DRAG_HOVERED_EVENT.on(update) {
            // system drag hover window
            update_sv = true;
            DRAG_DROP_SV.write().system_dragging.extend(args.data.iter().cloned());
        } else if let Some(args) = RAW_DRAG_MOVED_EVENT.on(update) {
            // code adapted from the MouseManager implementation for mouse hovered
            let moved = self.pos != args.position || self.pos_window != Some(args.window_id);
            if moved {
                self.pos = args.position;
                self.pos_window = Some(args.window_id);

                let mut position = args.position;

                // mouse_move data
                let mut frame_info = match WINDOWS.widget_tree(args.window_id) {
                    Ok(f) => f,
                    Err(_) => {
                        // window not found
                        if let Some(hovered) = self.hovered.take() {
                            DRAG_HOVERED_EVENT.notify(DragHoveredArgs::now(
                                Some(hovered),
                                None,
                                position,
                                HitTestInfo::no_hits(args.window_id),
                            ));
                            self.pos_window = None;
                        }
                        return;
                    }
                };

                let mut pos_hits = frame_info.root().hit_test(position.to_px(frame_info.scale_factor()));

                let target = if let Some(t) = pos_hits.target() {
                    if let Some(w) = frame_info.get(t.widget_id) {
                        if let Some(f) = w.nested_window_tree() {
                            // nested window hit
                            frame_info = f;
                            let factor = frame_info.scale_factor();
                            let pos = position.to_px(factor);
                            let pos = w.inner_transform().inverse().and_then(|t| t.transform_point(pos)).unwrap_or(pos);
                            pos_hits = frame_info.root().hit_test(pos);
                            position = pos.to_dip(factor);
                            pos_hits
                                .target()
                                .and_then(|h| frame_info.get(h.widget_id))
                                .map(|w| w.interaction_path())
                                .unwrap_or_else(|| frame_info.root().interaction_path())
                        } else {
                            w.interaction_path()
                        }
                    } else {
                        tracing::error!("hits target `{}` not found", t.widget_id);
                        frame_info.root().interaction_path()
                    }
                } else {
                    frame_info.root().interaction_path()
                }
                .unblocked();

                self.hits = Some(pos_hits.clone());

                // drag_enter/leave.
                let hovered_args = if self.hovered != target {
                    let prev_target = mem::replace(&mut self.hovered, target.clone());
                    let args = DragHoveredArgs::now(prev_target, target.clone(), position, pos_hits.clone());
                    Some(args)
                } else {
                    None
                };

                // mouse_move
                if let Some(target) = target {
                    let args = DragMoveArgs::now(frame_info.window_id(), args.coalesced_pos.clone(), position, pos_hits, target);
                    DRAG_MOVE_EVENT.notify(args);
                }

                if let Some(args) = hovered_args {
                    DRAG_HOVERED_EVENT.notify(args);
                }
            }
        } else if let Some(args) = RAW_DRAG_CANCELLED_EVENT.on(update) {
            // system drag cancelled of dragged out of all app windows
            let mut sv = DRAG_DROP_SV.write();
            update_sv = !sv.system_dragging.is_empty();
            sv.system_dragging.clear();

            if let Some(prev) = self.hovered.take() {
                self.pos_window = None;
                DRAG_HOVERED_EVENT.notify(DragHoveredArgs::now(
                    Some(prev),
                    None,
                    self.pos,
                    self.hits.take().unwrap_or_else(|| HitTestInfo::no_hits(args.window_id)),
                ));
            }
        } else if let Some(args) = RAW_APP_DRAG_ENDED_EVENT.on(update) {
            let mut sv = DRAG_DROP_SV.write();
            sv.app_dragging.retain(|d| {
                if d.view_id != args.id {
                    return true;
                }

                if !args.applied.is_empty() && !d.allowed.contains(args.applied) {
                    tracing::error!(
                        "drop target applied disallowed effect, allowed={:?}, applied={:?}",
                        d.allowed,
                        args.applied
                    );
                }

                DRAG_END_EVENT.notify(DragEndArgs::now(d.target.clone(), args.applied));

                false
            });
        }

        if update_sv {
            DRAG_DROP.update_var();
        }
    }

    fn event(&mut self, update: &mut EventUpdate) {
        if let Some(args) = MOUSE_INPUT_EVENT.on_unhandled(update) {
            if matches!(args.state, ButtonState::Pressed) {
                if let Some(wgt) = WINDOWS.widget_info(args.target.widget_id()) {
                    if let Some(wgt) = wgt.self_and_ancestors().find(|w| w.is_draggable()) {
                        // unhandled mouse press on draggable
                        args.propagation().stop();
                        let target = wgt.interaction_path();
                        let args = DragStartArgs::now(target.clone());
                        DRAG_START_EVENT.notify(args);
                        DRAG_DROP_SV.write().app_drag = Some(AppDragging {
                            target,
                            data: vec![],
                            handles: vec![],
                            allowed: DragDropEffect::empty(),
                            view_id: DragDropId(0),
                        }); // calls to DRAG_DROP.drag are now valid
                    }
                }
            }
        } else if let Some(args) = TOUCH_INPUT_EVENT.on_unhandled(update) {
            if matches!(args.phase, TouchPhase::Start) {
                if let Some(wgt) = WINDOWS.widget_info(args.target.widget_id()) {
                    if let Some(wgt) = wgt.self_and_ancestors().find(|w| w.is_draggable()) {
                        // unhandled touch start on draggable
                        args.propagation().stop();
                        let target = wgt.interaction_path();
                        let args = DragStartArgs::now(target.clone());
                        DRAG_START_EVENT.notify(args);
                        DRAG_DROP_SV.write().app_drag = Some(AppDragging {
                            target,
                            data: vec![],
                            handles: vec![],
                            allowed: DragDropEffect::empty(),
                            view_id: DragDropId(0),
                        }); // calls to DRAG_DROP.drag are now valid
                    }
                }
            }
        } else if let Some(args) = DRAG_START_EVENT.on(update) {
            // finished notifying draggable drag start
            let mut sv = DRAG_DROP_SV.write();
            let mut data = sv.app_drag.take();
            let mut cancel = args.propagation_handle.is_stopped();
            if !cancel {
                if let Some(d) = &mut data {
                    if d.data.is_empty() {
                        d.data.push(encode_widget_id(args.target.widget_id()));
                        d.allowed = DragDropEffect::all();
                    }
                    match WINDOWS_DRAG_DROP.start_drag_drop(d.target.window_id(), mem::take(&mut d.data), d.allowed) {
                        Ok(id) => {
                            d.view_id = id;
                            sv.app_dragging.push(data.take().unwrap());
                        }
                        Err(e) => {
                            tracing::error!("cannot start drag&drop, {e}");
                            cancel = true;
                        }
                    }
                } else {
                    tracing::warn!("external notification of DRAG_START_EVENT ignored")
                }
            }
            if cancel {
                if let Some(d) = data {
                    DRAG_END_EVENT.notify(DragEndArgs::now(d.target, DragDropEffect::empty()));
                }
            }
        } else if let Some(args) = DROP_EVENT.on(update) {
            let _ = WINDOWS_DRAG_DROP.drag_dropped(args.target.window_id(), args.drop_id, *args.applied.lock());
        }
    }

    fn update_preview(&mut self) {
        let mut sv = DRAG_DROP_SV.write();

        // fulfill drop requests
        if let Some((id, target, data, allowed)) = sv.pending_drop.take() {
            let window_id = self.pos_window.take().unwrap();
            let hits = self.hits.take().unwrap_or_else(|| HitTestInfo::no_hits(window_id));
            DRAG_HOVERED_EVENT.notify(DragHoveredArgs::now(Some(target.clone()), None, self.pos, hits.clone()));
            DROP_EVENT.notify(DropArgs::now(
                target,
                data,
                allowed,
                self.pos,
                hits,
                id,
                Arc::new(Mutex::new(DragDropEffect::empty())),
            ));
        }
    }
}

/// Drag & drop service.
#[allow(non_camel_case_types)]
pub struct DRAG_DROP;
impl DRAG_DROP {
    /// All data current dragging.
    pub fn dragging_data(&self) -> ReadOnlyArcVar<Vec<DragDropData>> {
        DRAG_DROP_SV.read().data.read_only()
    }

    /// Start dragging `data`.
    ///
    /// This method will only work if a [`DRAG_START_EVENT`] is notifying. Handlers of draggable widgets
    /// can provide custom drag data using this method.
    ///
    /// Returns a handle that can be dropped to cancel the drag operation. A [`DRAG_END_EVENT`] notifies
    /// the draggable widget on cancel or drop. Logs an error message and returns a dummy handle on error.
    ///
    /// Note that the `allowed_effects` apply to all data, if a previous handler already set data with an incompatible
    /// effect the call is an error and the data ignored.
    pub fn drag(&self, data: DragDropData, allowed_effects: DragDropEffect) -> DragHandle {
        let mut sv = DRAG_DROP_SV.write();
        if let Some(d) = &mut sv.app_drag {
            if allowed_effects.is_empty() {
                tracing::error!("cannot drag, no `allowed_effects`");
                return DragHandle::dummy();
            }

            if d.allowed.is_empty() {
                d.allowed = allowed_effects;
            } else {
                if !d.allowed.contains(allowed_effects) {
                    tracing::error!("cannot drag, other data already set with incompatible `allowed_effects`");
                    return DragHandle::dummy();
                }
                d.allowed |= allowed_effects
            }

            d.data.push(data);
            let (owner, handle) = DragHandle::new();
            d.handles.push(owner);
            return handle;
        }
        tracing::error!("cannot drag, not in `DRAG_START_EVENT` interval");
        DragHandle::dummy()
    }

    fn update_var(&self) {
        let sv = DRAG_DROP_SV.read();
        sv.data.set(sv.system_dragging.clone());
    }
}

app_local! {
    static DRAG_DROP_SV: DragDropService = DragDropService {
        data: var(vec![]),
        system_dragging: vec![],
        app_drag: None,
        app_dragging: vec![],
        pending_drop: None,
    };
}
struct DragDropService {
    data: ArcVar<Vec<DragDropData>>,

    system_dragging: Vec<DragDropData>,

    app_drag: Option<AppDragging>,
    app_dragging: Vec<AppDragging>,

    pending_drop: Option<(DragDropId, InteractionPath, Vec<DragDropData>, DragDropEffect)>,
}
struct AppDragging {
    target: InteractionPath,
    data: Vec<DragDropData>,
    handles: Vec<HandleOwner<()>>,
    allowed: DragDropEffect,
    view_id: DragDropId,
}

/// Represents dragging data.
///
/// Drop all clones of this handle to cancel the drag operation.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
#[repr(transparent)]
#[must_use = "dropping the handle cancels the drag operation"]
pub struct DragHandle(Handle<()>);
impl DragHandle {
    fn new() -> (HandleOwner<()>, Self) {
        let (owner, handle) = Handle::new(());
        (owner, Self(handle))
    }

    /// New handle to nothing.
    pub fn dummy() -> Self {
        Self(Handle::dummy(()))
    }

    /// Drops the handle but does **not** cancel the drag operation.
    ///
    /// The drag data stays alive until the user completes or cancels the operation.
    pub fn perm(self) {
        self.0.perm();
    }

    /// If another handle has called [`perm`](Self::perm).
    ///
    /// If `true` operation will run to completion.
    pub fn is_permanent(&self) -> bool {
        self.0.is_permanent()
    }

    /// Drops the handle and forces operation the cancel.
    pub fn cancel(self) {
        self.0.force_drop()
    }

    /// If another handle has called [`cancel`](Self::cancel).
    pub fn is_canceled(&self) -> bool {
        self.0.is_dropped()
    }

    /// Create a weak handle.
    pub fn downgrade(&self) -> WeakDragHandle {
        WeakDragHandle(self.0.downgrade())
    }
}
/// Weak [`DragHandle`].
#[derive(Clone, PartialEq, Eq, Hash, Default, Debug)]
pub struct WeakDragHandle(WeakHandle<()>);
impl WeakDragHandle {
    /// New weak handle that does not upgrade.
    pub fn new() -> Self {
        Self(WeakHandle::new())
    }

    /// Gets the strong handle if it is still subscribed.
    pub fn upgrade(&self) -> Option<DragHandle> {
        self.0.upgrade().map(DragHandle)
    }
}

/// [`WidgetInfo`] extensions for drag & drop service.
pub trait WidgetInfoDragDropExt {
    /// If this widget can be dragged and dropped.
    fn is_draggable(&self) -> bool;
}
impl WidgetInfoDragDropExt for WidgetInfo {
    fn is_draggable(&self) -> bool {
        self.meta().flagged(*IS_DRAGGABLE_ID)
    }
}

/// [`WidgetInfoBuilder`] extensions for drag & drop service.
pub trait WidgetInfoBuilderDragDropExt {
    /// Flag the widget as draggable.
    fn draggable(&mut self);
}
impl WidgetInfoBuilderDragDropExt for WidgetInfoBuilder {
    fn draggable(&mut self) {
        self.flag_meta(*IS_DRAGGABLE_ID);
    }
}

static_id! {
    static ref IS_DRAGGABLE_ID: StateId<()>;
}

event_args! {
    /// Arguments for [`DROP_EVENT`].
    pub struct DropArgs {
        /// Hovered target of the drag&drop gesture.
        pub target: InteractionPath,
        /// Drag&drop data payload.
        pub data: Vec<DragDropData>,
        /// Drop effects that the drag source allows.
        pub allowed: DragDropEffect,
        /// Position of the cursor in the window's content area.
        pub position: DipPoint,
        /// Hit-test result for the cursor point in the window.
        pub hits: HitTestInfo,

        drop_id: DragDropId,
        applied: Arc<Mutex<DragDropEffect>>,

        ..

        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.insert_wgt(&self.target);
        }
    }

    /// Arguments for [`DRAG_HOVERED_EVENT`].
    pub struct DragHoveredArgs {
        /// Previous hovered target.
        pub prev_target: Option<InteractionPath>,
        /// New hovered target.
        pub target: Option<InteractionPath>,
        /// Position of the cursor in the window's content area.
        pub position: DipPoint,
        /// Hit-test result for the cursor point in the window.
        pub hits: HitTestInfo,

        ..

        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            if let Some(p) = &self.prev_target {
                list.insert_wgt(p);
            }
            if let Some(p) = &self.target {
                list.insert_wgt(p);
            }
        }
    }

    /// [`DRAG_MOVE_EVENT`] arguments.
    pub struct DragMoveArgs {
        /// Id of window that received the event.
        pub window_id: WindowId,

        /// Positions of the cursor in between the previous event and this one.
        ///
        /// Drag move events can be coalesced, i.e. multiple moves packed into a single event.
        pub coalesced_pos: Vec<DipPoint>,

        /// Position of the cursor in the window's content area.
        pub position: DipPoint,

        /// Hit-test result for the cursor point in the window.
        pub hits: HitTestInfo,

        /// Full path to the top-most hit in [`hits`](DragMoveArgs::hits).
        pub target: InteractionPath,

        ..

        /// The [`target`].
        ///
        /// [`target`]: Self::target
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.insert_wgt(&self.target);
        }
    }

    /// Arguments for [`DRAG_START_EVENT`].
    pub struct DragStartArgs {
        /// Draggable widget that has started dragging.
        pub target: InteractionPath,

        ..

        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.insert_wgt(&self.target);
        }
    }

    /// Arguments for [`DRAG_END_EVENT`].
    pub struct DragEndArgs {
        /// Draggable widget that was dragging.
        pub target: InteractionPath,

        /// Effect applied by the drop target on the data.
        ///
        /// Is empty or a single flag.
        pub applied: DragDropEffect,

        ..

        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.insert_wgt(&self.target);
        }

        /// The `applied` field can only be empty or only have a single flag set.
        fn validate(&self) -> Result<(), Txt> {
            if self.applied.is_empty() && self.applied.len() > 1 {
                return Err("only one or none `DragDropEffect` can be applied".into());
            }
            Ok(())
        }
    }
}
event! {
    /// Drag&drop action finished over some drop target widget.
    pub static DROP_EVENT: DropArgs;
    /// Drag&drop enter or exit a drop target widget.
    pub static DRAG_HOVERED_EVENT: DragHoveredArgs;
    /// Drag&drop is dragging over the target widget.
    pub static DRAG_MOVE_EVENT: DragMoveArgs;
    /// Drag&drop started dragging a draggable widget.
    ///
    /// If propagation is stopped the drag operation is cancelled. Handlers can use
    /// [`DRAG_DROP.drag`] to set the data, otherwise the widget ID will be dragged.
    ///
    /// [`DRAG_DROP.drag`]: DRAG_DROP::drag
    pub static DRAG_START_EVENT: DragStartArgs;

    /// Drag&drop gesture started from the draggable widget has ended.
    pub static DRAG_END_EVENT: DragEndArgs;
}

impl DropArgs {
    /// Deprecated
    #[deprecated = "use `self.target.contains_enabled`"]
    pub fn is_enabled(&self, widget_id: WidgetId) -> bool {
        self.target.contains_enabled(widget_id)
    }

    /// Deprecated
    #[deprecated = "use `self.target.contains_disabled`"]
    pub fn is_disabled(&self, widget_id: WidgetId) -> bool {
        self.target.contains_disabled(widget_id)
    }

    /// Stop propagation and set the `effect` that was applied to the data.
    ///
    /// Logs an error if propagation is already stopped.
    ///
    /// # Panics
    ///
    /// Panics if `effect` sets more then one flag or is not [`allowed`].
    ///
    /// [`allowed`]: Self::allowed
    pub fn applied(&self, effect: DragDropEffect) {
        assert!(effect.len() > 1, "can only apply one effect");
        assert!(self.allowed.contains(effect), "source does not allow this effect");

        let mut e = self.applied.lock();
        if !self.propagation().is_stopped() {
            self.propagation().stop();
            *e = effect;
        } else {
            tracing::error!("drop already handled");
        }
    }
}

impl DragHoveredArgs {
    /// Gets the [`DRAG_DROP.dragging_data`].
    ///
    /// [`DRAG_DROP.dragging_data`]: DRAG_DROP::dragging_data
    pub fn data(&self) -> ReadOnlyArcVar<Vec<DragDropData>> {
        DRAG_DROP.dragging_data()
    }

    /// Returns `true` if the [`WIDGET`] was not hovered, but now is.
    ///
    /// [`WIDGET`]: zng_app::widget::WIDGET
    pub fn is_drag_enter(&self) -> bool {
        !self.was_over() && self.is_over()
    }

    /// Returns `true` if the [`WIDGET`] was hovered, but now isn't.
    ///
    /// [`WIDGET`]: zng_app::widget::WIDGET
    pub fn is_drag_leave(&self) -> bool {
        self.was_over() && !self.is_over()
    }

    /// Returns `true` if the [`WIDGET`] is in [`prev_target`].
    ///
    /// [`prev_target`]: Self::prev_target
    /// [`WIDGET`]: zng_app::widget::WIDGET
    pub fn was_over(&self) -> bool {
        if let Some(t) = &self.prev_target {
            return t.contains(WIDGET.id());
        }

        false
    }

    /// Returns `true` if the [`WIDGET`] is in [`target`].
    ///
    /// [`target`]: Self::target
    /// [`WIDGET`]: zng_app::widget::WIDGET
    pub fn is_over(&self) -> bool {
        if let Some(t) = &self.target {
            return t.contains(WIDGET.id());
        }

        false
    }

    /// Returns `true` if the widget was enabled in [`prev_target`].
    ///
    /// [`prev_target`]: Self::prev_target
    pub fn was_enabled(&self, widget_id: WidgetId) -> bool {
        match &self.prev_target {
            Some(t) => t.contains_enabled(widget_id),
            None => false,
        }
    }

    /// Returns `true` if the widget was disabled in [`prev_target`].
    ///
    /// [`prev_target`]: Self::prev_target
    pub fn was_disabled(&self, widget_id: WidgetId) -> bool {
        match &self.prev_target {
            Some(t) => t.contains_disabled(widget_id),
            None => false,
        }
    }

    /// Returns `true` if the widget is enabled in [`target`].
    ///
    /// [`target`]: Self::target
    pub fn is_enabled(&self, widget_id: WidgetId) -> bool {
        match &self.target {
            Some(t) => t.contains_enabled(widget_id),
            None => false,
        }
    }

    /// Returns `true` if the widget is disabled in [`target`].
    ///
    /// [`target`]: Self::target
    pub fn is_disabled(&self, widget_id: WidgetId) -> bool {
        match &self.target {
            Some(t) => t.contains_disabled(widget_id),
            None => false,
        }
    }

    /// Returns `true` if the [`WIDGET`] was not hovered or was disabled, but now is hovered and enabled.
    ///
    /// [`WIDGET`]: zng_app::widget::WIDGET
    pub fn is_drag_enter_enabled(&self) -> bool {
        (!self.was_over() || self.was_disabled(WIDGET.id())) && self.is_over() && self.is_enabled(WIDGET.id())
    }

    /// Returns `true` if the [`WIDGET`] was hovered and enabled, but now is not hovered or is disabled.
    ///
    /// [`WIDGET`]: zng_app::widget::WIDGET
    pub fn is_drag_leave_enabled(&self) -> bool {
        self.was_over() && self.was_enabled(WIDGET.id()) && (!self.is_over() || self.is_disabled(WIDGET.id()))
    }

    /// Returns `true` if the [`WIDGET`] was not hovered or was enabled, but now is hovered and disabled.
    ///
    /// [`WIDGET`]: zng_app::widget::WIDGET
    pub fn is_drag_enter_disabled(&self) -> bool {
        (!self.was_over() || self.was_enabled(WIDGET.id())) && self.is_over() && self.is_disabled(WIDGET.id())
    }

    /// Returns `true` if the [`WIDGET`] was hovered and disabled, but now is not hovered or is enabled.
    ///
    /// [`WIDGET`]: zng_app::widget::WIDGET
    pub fn is_drag_leave_disabled(&self) -> bool {
        self.was_over() && self.was_disabled(WIDGET.id()) && (!self.is_over() || self.is_enabled(WIDGET.id()))
    }
}

impl DragEndArgs {
    /// Data was dropped on a valid target.
    pub fn was_dropped(&self) -> bool {
        !self.applied.is_empty()
    }

    /// Stopped dragging without dropping on a valid drop target.
    pub fn was_canceled(&self) -> bool {
        self.applied.is_empty()
    }
}

/// Encode an widget ID for drag&drop data.
pub fn encode_widget_id(id: WidgetId) -> DragDropData {
    DragDropData::Text {
        format: formatx!("zng/{}", APP_GUID.read().simple()),
        data: formatx!("wgt-{}", id.get()),
    }
}

/// Decode an widget ID from drag&drop data.
///
/// The ID will only decode if it was encoded by the same app instance.
pub fn decode_widget_id(data: &DragDropData) -> Option<WidgetId> {
    if let DragDropData::Text { format, data } = data {
        if let Some(guid) = format.strip_prefix("zng/") {
            if let Some(id) = data.strip_prefix("wgt-") {
                if guid == APP_GUID.read().simple().to_string() {
                    if let Ok(id) = id.parse::<u64>() {
                        return Some(WidgetId::from_raw(id));
                    }
                }
            }
        }
    }
    None
}

app_local! {
    static APP_GUID: uuid::Uuid = uuid::Uuid::new_v4();
}
