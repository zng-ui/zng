//! Frame builder and other types.
//!
//! # Full API
//!
//! See [`zero_ui_app::render`] for the full API.

pub use zero_ui_app::render::{
    ClipBuilder, Font, FontSynthesis, FrameBuilder, FrameUpdate, FrameValue, FrameValueKey, FrameValueUpdate, HitTestBuilder,
    HitTestClipBuilder, ImageRendering, RepeatMode, SpatialFrameId, SpatialFrameKey, StaticSpatialFrameId,
};
pub use zero_ui_view_api::window::FrameId;
