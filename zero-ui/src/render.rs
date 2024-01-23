//! Frame builder and other types.
//! 
//! Frame rendering means building a display list and updating all widget transforms, no actual pixel rendering happens
//! during the render pass, the built display list is send to the view-process where it is actually rendered.
//! 
//! Widgets render is centered around [`UiNode::render`] and [`UiNode::render_update`] using the [`FrameBuilder`]
//! and [`FrameUpdate`] types. During render 
//! 
//! [`UiNode::render`]: crate::widget::node::UiNode::render
//! [`UiNode::render_update`]: crate::widget::node::UiNode::render_update
//!
//! # Full API
//!
//! See [`zero_ui_app::render`] for the full API.

pub use zero_ui_app::render::{
    ClipBuilder, Font, FontSynthesis, FrameBuilder, FrameUpdate, FrameValue, FrameValueKey, FrameValueUpdate, HitTestBuilder,
    HitTestClipBuilder, ImageRendering, RepeatMode, SpatialFrameId, SpatialFrameKey, StaticSpatialFrameId,
};
pub use zero_ui_view_api::window::FrameId;
