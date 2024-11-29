//! Drag & drop gesture events and service.

use core::fmt;

use zng_app::{
    event::{event, event_args}, static_id,
    update::EventUpdate,
    view_process::raw_events::{RAW_DRAG_CANCELLED_EVENT, RAW_DRAG_DROPPED_EVENT, RAW_DRAG_HOVERED_EVENT},
    widget::info::{InteractionPath, WidgetInfo},
    AppExtension,
};
use zng_state_map::StateId;
use zng_txt::Txt;
use zng_var::ReadOnlyArcVar;

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
        if let Some(args) = RAW_DRAG_DROPPED_EVENT.on(update) {
            tracing::info!("!!: DROPPED {:?}", args.data)
        } else if let Some(args) = RAW_DRAG_HOVERED_EVENT.on(update) {
            tracing::info!("!!: HOVERED {:?}", args.data)
        } else if let Some(_args) = RAW_DRAG_CANCELLED_EVENT.on(update) {
            tracing::info!("!!: CANCELLED")
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
        todo!()
    }

    /// Start dragging `data`.
    /// 
    /// This method will only work if a [`DRAG_EVENT`] has just started. Handlers of draggable widgets
    /// can stop propagation of the event to provide custom drag data, set here.
    pub fn drag(&self, data: DragDropData) {
        todo!("!!: return handle")
    }
}

/// [`WidgetInfo`] extensions for drag & drop service.
pub trait WidgetDragDropExt {
    /// If this widget can be dragged and dropped.
    fn is_draggable(&self) -> bool;
}
impl WidgetDragDropExt for WidgetInfo {
    fn is_draggable(&self) -> bool {
        self.meta().flagged(*IS_DRAGGABLE_ID)
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
    pub struct DragHoverArgs {
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
    pub static DRAG_HOVER_EVENT: DragHoverArgs;
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
