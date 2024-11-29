//! Drag&drop service, types and events.
//!
//! The example below defines a window that shows the current dragging data that has entered any of the app windows.
//!
//! ```
//! use zng::prelude::*;
//! # let _scope = APP.defaults();
//!
//! # let _ =
//! Window! {
//!     padding = 10;
//!     child = Container! {
//!         widget::border = {
//!             widths: 5,
//!             sides: widget::BorderSides::dashed(colors::GRAY),
//!         };
//!         widget::corner_radius = 25;
//!         child_align = Align::CENTER;
//!         child = Text! {
//!             txt = zng::drag_drop::DRAG_DROP.dragging_data().map(|d| {
//!                 if d.is_empty() {
//!                     Txt::from("drag over to inspect")
//!                 } else {
//!                     formatx!("{d:#?}")
//!                 }
//!             });
//!         };
//!     }
//! }
//! # ;
//! ```
//!
//! # Full API
//!
//! See [`zng_ext_input::drag_drop`] and [`zng_wgt_input::drag_drop`] for the full drag&drop API.

pub use zng_ext_input::drag_drop::{
    DragDropData, DragHandle, SystemDragDropData, WeakDragHandle, DRAG_DROP, DRAG_END_EVENT, DRAG_HOVER_EVENT, DRAG_START_EVENT, DROP_EVENT,
};

pub use zng_wgt_input::drag_drop::{
    draggable, on_drag_end, on_drag_enter, on_drag_hover, on_drag_leave, on_drag_start, on_drop, on_pre_drag_end, on_pre_drag_enter,
    on_pre_drag_hover, on_pre_drag_leave, on_pre_drag_start, on_pre_drop,
};
