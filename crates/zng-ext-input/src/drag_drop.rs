//! Drag & drop gesture events and service.

use core::fmt;
use std::{mem, ops::ControlFlow};

use zng_app::{
    event::{event, event_args, AnyEventArgs},
    static_id,
    update::{EventUpdate, UPDATES},
    view_process::raw_events::{RAW_DRAG_CANCELLED_EVENT, RAW_DRAG_DROPPED_EVENT, RAW_DRAG_HOVERED_EVENT},
    widget::{
        info::{InteractionPath, WidgetInfo, WidgetInfoBuilder},
        WidgetId, WIDGET,
    },
    AppExtension,
};
use zng_app_context::app_local;
use zng_ext_window::WINDOWS;
use zng_handle::{Handle, HandleOwner, WeakHandle};
use zng_state_map::StateId;
use zng_txt::Txt;
use zng_var::{var, ArcVar, ReadOnlyArcVar, Var};
use zng_view_api::mouse::ButtonState;

use crate::mouse::{MOUSE_HOVERED_EVENT, MOUSE_INPUT_EVENT};

/// System wide drag&drop data payload.
pub type SystemDragDropData = zng_view_api::DragDropData;

/// Application extension that provides drag&drop events and service.
///
/// # Events
///
/// Events this extension provides.
///
/// # Services
///
/// Services this extension provides.
///
/// * [`DRAG_DROP`]
#[derive(Default)]
pub struct DragDropManager {}

impl AppExtension for DragDropManager {
    fn event_preview(&mut self, update: &mut EventUpdate) {
        let mut update_sv = false;
        if let Some(args) = RAW_DRAG_DROPPED_EVENT.on(update) {
            let mut sv = DRAG_DROP_SV.write();
            let len = sv.system_dragging.len();
            sv.system_dragging.retain(|d| match d {
                DragDropData::System { mime, data } => mime != &args.mime && data != &args.data,
                DragDropData::Widget(_) => unreachable!(),
            });
            update_sv = len != sv.system_dragging.len();
            drop(sv);

            // !!: TODO drop event
        } else if let Some(args) = RAW_DRAG_HOVERED_EVENT.on(update) {
            update_sv = true;
            DRAG_DROP_SV.write().system_dragging.push(DragDropData::System {
                mime: args.mime.clone(),
                data: args.data.clone(),
            });
        } else if let Some(_args) = RAW_DRAG_CANCELLED_EVENT.on(update) {
            let mut sv = DRAG_DROP_SV.write();
            update_sv = !sv.system_dragging.is_empty();
            sv.system_dragging.clear();
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
                        args.propagation().stop();
                        let args = DragStartArgs::now(wgt.interaction_path());
                        DRAG_START_EVENT.notify(args);
                    }
                }
            }
        } else if let Some(args) = DRAG_START_EVENT.on_unhandled(update) {
            args.propagation_handle.stop();

            if let Some(wgt) = WINDOWS.widget_info(args.target.widget_id()) {
                let (owner, handle) = DragHandle::new();
                handle.perm();
                DRAG_DROP_SV.write().app_dragging.push((owner, DragDropData::Widget(wgt)));
                DRAG_DROP.update_var();
            }
        } else if DROP_EVENT.has_subscribers() {
            if let Some(args) = MOUSE_HOVERED_EVENT.on_unhandled(update) {
                let mut prev_target = None;
                let mut target = None;
                fn check_target(path: &Option<InteractionPath>, out: &mut Option<InteractionPath>) {
                    if let Some(path) = path {
                        if let Some(true) = DROP_EVENT.visit_subscribers(|id| {
                            if path.as_path().widgets_path().contains(&id) {
                                ControlFlow::Break(true)
                            } else {
                                ControlFlow::Continue(())
                            }
                        }) {
                            *out = Some(path.clone());
                        }
                    }
                }
                check_target(&args.prev_target, &mut prev_target);
                check_target(&args.target, &mut target);

                if prev_target.is_some() || target.is_some() {
                    let args = DragHoveredArgs::now(prev_target, target);
                    DRAG_HOVERED_EVENT.notify(args);
                }
            }
        }
    }

    fn update_preview(&mut self) {
        let mut sv = DRAG_DROP_SV.write();
        let mut requests = mem::take(&mut sv.drag);
        sv.can_drag = false;

        requests.retain(|(h, _)| !h.is_dropped());
        if !requests.is_empty() {
            sv.app_dragging.extend(requests);
            DRAG_DROP.update_var();
        }
    }
}

/// Drag & drop service.
///
/// # Support
///
/// The default view-process implementer depends on `winit` for drag&drop events, this has some limitations:
///
/// * Only file path drop.
/// * No support in Linux/Wayland, you can work around by calling `std::env::remove_var("WAYLAND_DISPLAY");` before `zng::env::init!()` in
///   your main function, this enables XWayland that has support for the basic file path drop.
#[allow(non_camel_case_types)]
pub struct DRAG_DROP;
impl DRAG_DROP {
    /// All data current dragging.
    pub fn dragging_data(&self) -> ReadOnlyArcVar<Vec<DragDropData>> {
        DRAG_DROP_SV.read().data.read_only()
    }

    /// Start dragging `data`.
    ///
    /// This method will only work if a [`DRAG_EVENT`] has just started. Handlers of draggable widgets
    /// can stop propagation of the event to provide custom drag data, set here.
    pub fn drag(&self, data: DragDropData) -> DragHandle {
        let mut sv = DRAG_DROP_SV.write();
        if !sv.can_drag {
            return DragHandle::dummy();
        }

        let (owner, handle) = DragHandle::new();
        sv.drag.push((owner, data));
        UPDATES.update(None);
        handle
    }

    fn update_var(&self) {
        let mut sv = DRAG_DROP_SV.write();
        sv.app_dragging.retain(|(h, _)| !h.is_dropped());
        let mut data = sv.system_dragging.clone();
        data.extend(sv.app_dragging.iter().map(|(_, d)| d.clone()));
        sv.data.set(data);
    }
}

app_local! {
    static DRAG_DROP_SV: DragDropService = DragDropService {
        data: var(vec![]),
        can_drag: false,
        drag: vec![],
        system_dragging: vec![],
        app_dragging: vec![],
    };
}
struct DragDropService {
    data: ArcVar<Vec<DragDropData>>,
    can_drag: bool,
    drag: Vec<(HandleOwner<()>, DragDropData)>,

    system_dragging: Vec<DragDropData>,
    app_dragging: Vec<(HandleOwner<()>, DragDropData)>,
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

/// Drag&drop gesture payload.
#[derive(Clone, PartialEq)]
pub enum DragDropData {
    /// Another widget in the app.
    Widget(WidgetInfo),
    /// System wide data.
    System {
        /// Data type.
        mime: Txt,
        /// Data payload.
        data: SystemDragDropData,
    },
}
impl fmt::Debug for DragDropData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "DragDropData::")?;
        }
        match self {
            Self::Widget(arg0) => f.debug_tuple("Widget").field(&arg0.path()).finish(),
            Self::System { mime, data } => f.debug_struct("System").field("mime", mime).field("data", data).finish(),
        }
    }
}

event_args! {
    /// Arguments for [`DROP_EVENT`].
    pub struct DropArgs {
        /// Hovered target of the drag&drop gesture.
        pub target: InteractionPath,

        /// Drag&drop data payload.
        pub data: DragDropData,

        ..

        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.insert_wgt(&self.target);
        }
    }

    /// Arguments for [`DRAG_HOVER_EVENT`].
    pub struct DragHoveredArgs {
        /// Previous hovered target.
        pub prev_target: Option<InteractionPath>,
        /// New hovered target.
        pub target: Option<InteractionPath>,

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

        /// If the drag was dropped on a valid drop target.
        pub was_dropped: bool,

        ..

        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.insert_wgt(&self.target);
        }
    }
}
event! {
    /// Drag&drop action finished over some widget.
    pub static DROP_EVENT: DropArgs;
    /// Drag&drop enter or exit a widget.
    pub static DRAG_HOVERED_EVENT: DragHoveredArgs;
    /// Drag&drop started dragging a draggable widget.
    ///
    /// If the event propagation is not stopped the widget will be dragged by default. Handlers can stop and call [`DRAG_DROP.drag`]
    /// to set custom drag data.
    ///
    /// [`DRAG_DROP.drag`]: DRAG_DROP::drag
    pub static DRAG_START_EVENT: DragStartArgs;

    /// Drag&drop gesture started from the draggable widget has ended.
    pub static DRAG_END_EVENT: DragEndArgs;
}

impl DropArgs {
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

impl DragHoveredArgs {
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
    /// [`prev_capture`]: Self::prev_capture
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
    /// [`capture`]: Self::capture
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
