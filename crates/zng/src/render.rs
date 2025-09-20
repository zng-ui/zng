//! Frame builder and other types.
//!
//! Frame rendering means building a display list and updating all widget transforms, no actual pixel rendering happens
//! during the render pass, the built display list is send to the view-process where it is actually rendered.
//!
//! Widgets render is centered around [`UiNode::render`] and [`UiNode::render_update`] using the [`FrameBuilder`]
//! and [`FrameUpdate`] types. Render builds a display list, updates widget transforms and hit-test shapes, during
//! render some values in the display list can be bound to a [`FrameValueKey`], this key can be used during `render_update`
//! to replace the value in the last display list instead of rebuilding it.
//!
//! Note that even without render-updates all widgets that do not request render and are not ancestor to one are reused.
//! Reused widgets only include a range of display items to copy from the previous display list. A normal release built window
//! can easily achieve 60FPS rendering even without render-updates, but reusable components should try to achieve best performance.
//!
//! ```
//! use zng::prelude_wgt::*;
//!
//! /// Fills the available space with a centered circle of the color.
//! ///
//! /// This node disables inline layout for the widget.
//! pub fn color_circle(color: impl IntoVar<Rgba>) -> UiNode {
//!     let color = color.into_var();
//!     let mut area = PxRect::zero();
//!
//!     // key to the color in a rendered frame,
//!     // can be used to update the frame without rebuilding the display list
//!     let color_key = FrameValueKey::new_unique();
//!
//!     match_node_leaf(move |op| match op {
//!         UiNodeOp::Init => {
//!             // request a frame update when the color changes
//!             WIDGET.sub_var_render_update(&color);
//!         }
//!         UiNodeOp::Measure { wm, desired_size } => {
//!             wm.disable_inline(); // is inline-block
//!             *desired_size = LAYOUT.constraints().fill_size();
//!         }
//!         UiNodeOp::Layout { final_size, .. } => {
//!             *final_size = LAYOUT.constraints().fill_size();
//!
//!             // centered square
//!             let mut a = PxRect::from_size(*final_size);
//!             if a.size.width < a.size.height {
//!                 a.origin.y = (a.size.height - a.size.width) / Px(2);
//!                 a.size.height = a.size.width;
//!             } else {
//!                 a.origin.x = (a.size.width - a.size.height) / Px(2);
//!                 a.size.width = a.size.height;
//!             }
//!
//!             if a != area {
//!                 area = a;
//!                 // request a full render because are is not keyed for updates
//!                 WIDGET.render();
//!             }
//!         }
//!         UiNodeOp::Render { frame } => {
//!             // clip a circle at the area
//!             frame.push_clip_rounded_rect(area, PxCornerRadius::new_all(area.size), false, false, |frame| {
//!                 // fill the are with color, bind the color_key to the color
//!                 frame.push_color(area, color_key.bind_var(&color, |&c| c.into()));
//!             });
//!         }
//!         UiNodeOp::RenderUpdate { update } => {
//!             // update the color in the existing frame, this is an optimization
//!             update.update_color_opt(color_key.update_var(&color, |&c| c.into()));
//!         }
//!         _ => {}
//!     })
//! }
//! ```
//!
//! The example above declares a simple node that draws a colored circle, the circle color is keyed for render updates.
//!
//! ```
//! # use zng::prelude::*;
//! # fn example() {
//! # fn color_circle(_color: impl IntoVar<zng::color::Rgba>) -> UiNode { UiNode::nil() }
//! let color = var(colors::RED);
//! let mut i = 0u8;
//! # let _ =
//! Container! {
//!     child = color_circle(color.easing_with(1.secs(), easing::linear, color::rgba_sampler));
//!     gesture::on_click = hn!(|_| {
//!         color.set(match i {
//!             0 => colors::YELLOW,
//!             1 => colors::GREEN,
//!             2 => colors::RED,
//!             _ => unreachable!(),
//!         });
//!         i += 1;
//!         if i == 3 {
//!             i = 0;
//!         }
//!     });
//! }
//! # ; }
//! ```
//!
//! [`UiNode::render`]: crate::widget::node::UiNode::render
//! [`UiNode::render_update`]: crate::widget::node::UiNode::render_update
//!
//! # Full API
//!
//! See [`zng_app::render`] for the full API.

pub use zng_app::render::{
    ClipBuilder, FontSynthesis, FrameBuilder, FrameUpdate, FrameValue, FrameValueKey, FrameValueUpdate, HitTestBuilder, HitTestClipBuilder,
    ImageRendering, ReferenceFrameId, ReuseRange, SpatialFrameId,
};
pub use zng_view_api::window::FrameId;
