#![cfg(feature = "drag_drop")]

//! Drag&drop service, types and events.
//!
//! The example below defines a window that shows the current dragging data that has entered any of the app windows.
//!
//! ```
//! use zng::prelude::*;
//! # let _scope = APP.defaults();
//! use zng::drag_drop::*;
//!
//! let data = var::<Vec<DragDropData>>(vec![]);
//! # let _ =
//! Window! {
//!     padding = 20;
//!     child = Container! {
//!         widget::border = 5, widget::BorderSides::dashed(colors::GRAY);
//!         widget::corner_radius = 15;
//!         child_align = Align::CENTER;
//!         on_drag_enter = hn!(data, |_| {
//!             data.set(DRAG_DROP.dragging_data().get());
//!         });
//!         on_drag_leave = hn!(data, |_| {
//!             data.set(vec![]);
//!         });
//!         on_drop = hn!(data, |args: &DropArgs| {
//!             data.set(args.data.clone());
//!         });
//!         child = Text!(data.map(|d| if d.is_empty() {
//!             Txt::from("drag over to inspect")
//!         } else {
//!             formatx!("{d:#?}")
//!         }));
//!     };
//! }
//! # ;
//! ```
//!
//!
//! # Limitations
//!
//! Drag&drop depends on the view-process backend, the default view-process (`zng-view`) is currently very limited:
//!
//! * No drag start from the app.
//! * Only file path drops.
//! * No support in Wayland, you can work around by calling `std::env::remove_var("WAYLAND_DISPLAY");` before `zng::env::init!()` in
//!   your main function, this enables XWayland that has support for the basic file path drop.
//! * In X11 and macOS there is no cursor position notification on hover, just on drop, `DRAG_HOVERED_EVENT` and `DRAG_MOVE_EVENT`
//!   based event properties will only fire once for the widget that is about to receive a drop.
//!
//! # Full API
//!
//! See [`zng_ext_input::drag_drop`] and [`zng_wgt_input::drag_drop`] for the full drag&drop API.

pub use zng_ext_input::drag_drop::{
    DRAG_DROP, DRAG_END_EVENT, DRAG_HOVERED_EVENT, DRAG_MOVE_EVENT, DRAG_START_EVENT, DROP_EVENT, DragDropData, DragDropEffect,
    DragEndArgs, DragHandle, DragHoveredArgs, DragMoveArgs, DragStartArgs, DropArgs, WeakDragHandle,
};

pub use zng_wgt_input::drag_drop::{
    draggable, on_drag_end, on_drag_enter, on_drag_hovered, on_drag_leave, on_drag_start, on_drop, on_pre_drag_end, on_pre_drag_enter,
    on_pre_drag_hovered, on_pre_drag_leave, on_pre_drag_start, on_pre_drop,
};
