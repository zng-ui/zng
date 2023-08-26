//! Touch events and service.
//!
//! The app extension [`TouchManager`] provides the events and service. It is included in the default application.

pub use zero_ui_view_api::{TouchForce, TouchId, TouchPhase, TouchUpdate};

use crate::app::AppExtension;

/// Application extension that provides touch events and service.
#[derive(Default)]
pub struct TouchManager {}
impl AppExtension for TouchManager {}
