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
//! use zero_ui::prelude::*;
//!
//! # let _scope = APP.defaults();
//! # let _ =
//! Container! {
//!     child_top = {
//!         insert: Text!("secondary (top)"),
//!         spacing: 5,
//!     };
//!     child = Text! { txt = "primary"; widget::background_color = colors::BLUE };
//!     child_bottom = {
//!         insert: Text!("secondary (bottom)"),
//!         spacing: 0,
//!     };
//! }
//! # ;
//! ```
//!
//! Note that `Window!` inherits from `Container!` to the example above could become the skeleton of a classic app window:
//!
//! ```
//! # use zero_ui::prelude::*;
//! # let _scope = APP.defaults();
//! # fn tools() -> impl UiNode { widget::node::NilUiNode }
//! # fn content() -> impl UiNode { widget::node::NilUiNode }
//! # fn status() -> impl UiNode { widget::node::NilUiNode }
//! # let _ =
//! Window! {
//!     child_top = tools(), 0;
//!     child = content();
//!     child_bottom = status(), 0;
//! }
//! # ;
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
//! The child can by any [`UiNode`] type, not just widgets, you can use this to plug nodes directly on the UI.
//!
//! ```
//! use zero_ui::{prelude::*, prelude_wgt::*};
//!
//! # let _scope = APP.defaults();
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
//!             },
//!             UiNodeOp::Render { frame } => frame.push_color(
//!                 PxRect::from_size(render_size),
//!                 FrameValue::Value(colors::GREEN.into())
//!             ),
//!             _ => {}
//!         })
//!     }
//! }
//! # ;
//! ```
//!
//! [`UiNode`]: crate::widget::node::UiNode
//!
//! # Full API
//!
//! See [`zero_ui_wgt_container`] for the full widget API.

pub use zero_ui_wgt_container::{
    child_bottom, child_end, child_insert, child_left, child_out_insert, child_right, child_start, child_top, ChildInsert, Container,
};
