#![cfg(feature = "wrap")]

//! Wrap layout widget and properties.
//!
//! The [`Wrap!`](struct@Wrap) widget implements [inline layout](crate::layout#inline). The example below demonstrates
//! a *rich text* composed of multiple `Wrap!` and `Text!` widgets.
//!
//! ```
//! use zng::prelude::*;
//! # fn example() {
//!
//! # let _ =
//! Wrap!(ui_vec![
//!     Text!("Some text that "),
//!     text::Strong!("wraps"),
//!     Text!(" together."),
//!     Wrap! {
//!         text::font_color = colors::GREEN;
//!         children = ui_vec![
//!             Text!(" Nested Wrap panels can be used to set "),
//!             text::Em!("contextual"),
//!             Text!(" properties for a sequence of widgets.")
//!         ];
//!     },
//!     Text!(" The nested Wrap panel content items "),
//!     text::Strong!("wrap"),
//!     Text!(" with the parent items."),
//! ])
//! # ; }
//! ```
//!
//! Note that only some widgets and properties support inline layout, see the [`layout`](crate::layout#inline)
//! module documentation for more details.
//!
//! # Full API
//!
//! See [`zng_wgt_wrap`] for the full view API.

pub use zng_wgt_wrap::{
    WidgetInfoWrapExt, Wrap, get_index, get_index_len, get_rev_index, is_even, is_first, is_last, is_odd, lazy_sample, lazy_size, node,
};
