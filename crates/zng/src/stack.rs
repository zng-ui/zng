#![cfg(feature = "stack")]

//! Stack layout widget, nodes and properties.
//!
//! The [`Stack!`](struct@Stack) widget is a layout panel stacks children, in Z and in a [`direction`](struct@Stack#method.direction).
//!
//! The example below declares a stack that animates between directions.
//!
//! ```
//! use zng::prelude::*;
//!
//! # let _scope = APP.defaults();
//! let direction = var(StackDirection::top_to_bottom());
//! # let _ =
//! Stack! {
//!     direction = direction.easing(1.secs(), |t| easing::ease_out(easing::expo, t));
//!     spacing = 10;
//!     children_align = layout::Align::CENTER;
//!
//!     toggle::selector = toggle::Selector::single(direction);
//!     children = [
//!         ("top_to_bottom", StackDirection::top_to_bottom()),
//!         ("left_to_right", StackDirection::left_to_right()),
//!         ("bottom_to_top", StackDirection::bottom_to_top()),
//!         ("right_to_left", StackDirection::right_to_left()),
//!         ("diagonal", StackDirection {
//!             place: layout::Point::bottom_right(),
//!             origin: layout::Point::top_left(),
//!             is_rtl_aware: false,
//!         }),
//!     ]
//!     .into_iter()
//!     .map(|(label, direction)| Toggle! {
//!         child = Text!(label);
//!         value::<StackDirection> = direction;
//!     })
//!     .collect::<UiVec>();
//! }
//! # ;
//! ```
//!
//! Note that the [`StackDirection`] is defined by two points, the stack widget resolves the `place` point in the previous
//! child and the `origin` point in the next child and then positions the next child so that both points overlap. This enables
//! custom layouts like partially overlapping children and the traditional horizontal and vertical stack.
//!
//! # Full API
//!
//! See [`zng_wgt_stack`] for the full widget API.

pub use zng_wgt_stack::{
    Stack, StackDirection, WidgetInfoStackExt, get_index, get_index_len, get_rev_index, is_even, is_first, is_last, is_odd, lazy_sample,
    lazy_size, node, stack_nodes,
};
