//! Drag & drop gesture events and service.

use zng_app::{
    update::EventUpdate,
    view_process::raw_events::{RAW_DRAG_CANCELLED_EVENT, RAW_DRAG_DROPPED_EVENT, RAW_DRAG_HOVERED_EVENT},
    AppExtension,
};

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
#[allow(non_camel_case_types)]
pub struct DRAG_DROP;
