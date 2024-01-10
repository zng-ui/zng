//! App update service and types.
//!
//! # Full API
//!
//! See [`zero_ui_app::update`] for the full update API.

pub use zero_ui_app::update::{
    ContextUpdates, EventUpdate, InfoUpdates, LayoutUpdates, OnUpdateHandle, RenderUpdates, UpdateArgs, UpdateDeliveryList, UpdateOp,
    UpdateSubscribers, UpdatesTraceUiNodeExt, WeakOnUpdateHandle, WidgetUpdates, UPDATES,
};
