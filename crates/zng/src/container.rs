//! Container widget.
//!
//! Base widget for all widgets that are designed around a single child widget or a primary child widget surrounded by secondary widgets.
//!
//! # Child Inserts
//!
//! The example below demonstrates a container with a primary child that fills the available space not taken by the other children.
//! The top child is also separated from the primary child by 5dip.
//!
//! ```
//! use zng::prelude::*;
//!
//! # fn example() {
//! # let _ =
//! Container! {
//!     child_top = {
//!         node: Text!("secondary (top)"),
//!         spacing: 5,
//!     };
//!     child = Text! {
//!         txt = "primary";
//!         widget::background_color = colors::BLUE;
//!     };
//!     child_bottom = {
//!         node: Text!("secondary (bottom)"),
//!         spacing: 0,
//!     };
//! }
//! # ; }
//! ```
//!
//! Note that `Window!` inherits from `Container!` to the example above could become the skeleton of a classic app window:
//!
//! ```
//! # use zng::prelude::*;
//! # fn example() {
//! # fn tools() -> UiNode { widget::node::UiNode::nil() }
//! # fn content() -> UiNode { widget::node::UiNode::nil() }
//! # fn status() -> UiNode { widget::node::UiNode::nil() }
//! # let _ =
//! Window! {
//!     child_top = tools(), 0;
//!     child = content();
//!     child_bottom = status(), 0;
//! }
//! # ; }
//! ```
//!
//! Note that a similar layout could be achieved using widgets like [`Grid!`], but the child insert properties are a convenient
//! way to define this kind of widget, also a container widget without child inserts does not pay any extra cost, the insertion
//! layout implementation if fully contained to the insert properties.
//!
//! [`Grid!`]: struct@crate::grid::Grid
//!
//! # Child Nodes
//!
//! The child can by any [`IntoUiNode`] type, not just widgets, you can use this to plug nodes directly on the UI.
//!
//! ```
//! use zng::{prelude::*, prelude_wgt::*};
//!
//! # fn example() {
//! # let _ =
//! Container! {
//!     widget::background_color = colors::BLACK;
//!     child_align = layout::Align::CENTER;
//!     child = {
//!         let size = Size::splat(40);
//!         let mut render_size = PxSize::zero();
//!         match_node_leaf(move |op| match op {
//!             UiNodeOp::Measure { desired_size, .. } => *desired_size = size.layout(),
//!             UiNodeOp::Layout { final_size, .. } => {
//!                 render_size = Size::splat(40).layout();
//!                 *final_size = render_size;
//!             }
//!             UiNodeOp::Render { frame } => frame.push_color(PxRect::from_size(render_size), FrameValue::Value(colors::GREEN.into())),
//!             _ => {}
//!         })
//!     };
//! }
//! # ; }
//! ```
//!
//! [`IntoUiNode`]: crate::widget::node::IntoUiNode
//!
//! # Full API
//!
//! See [`zng_wgt_container`] for the full widget API.

pub use zng_wgt_container::{
    ChildInsert, Container, child, child_bottom, child_end, child_insert, child_left, child_out_bottom, child_out_end, child_out_insert,
    child_out_left, child_out_right, child_out_start, child_out_top, child_over, child_right, child_start, child_top, child_under,
};
