#![cfg(feature = "data_view")]

//! Data view widget.
//!
//! The [`DataView!`](struct@DataView) widget can be used to dynamically presents data from a variable, unlike
//! the [`widget::node::presenter`](crate::widget::node::presenter) node the generated UI can be retained
//! across updates of the data variable.
//!
//! The example below declares a `DataView!` using the shorthand syntax:
//!
//! ```
//! use zng::prelude::*;
//!
//! fn countdown(n: impl IntoVar<usize>) -> UiNode {
//!     DataView!(::<usize>, n, hn!(|a: &DataViewArgs<usize>| {
//!         // we generate a new view on the first call or when the data has changed to zero.
//!         if a.view_is_nil() || a.data().get_new() == Some(0) {
//!             a.set_view(if a.data().get() > 0 {
//!                 // countdown view
//!                 Text! {
//!                     font_size = 28;
//!                     // bind data, same view will be used for all n > 0 values.
//!                     txt = a.data().map_to_txt();
//!                 }
//!             } else {
//!                 // finished view
//!                 Text! {
//!                     font_color = rgb(0, 128, 0);
//!                     font_size = 18;
//!                     txt = "Congratulations!";
//!                 }
//!             });
//!         }
//!     }))
//! }
//! ```
//!
//! You can also use the normal widget syntax and set the `view` property.
//!
//! ```
//! # use zng::prelude::*;
//! # let _scope = APP.defaults(); let n = var(0usize); let _ =
//! DataView! {
//!     view::<usize> = {
//!         data: n,
//!         update: hn!(|a: &DataViewArgs<usize>| { }),
//!     };
//! }
//! # ;
//! ```
//!
//! # Full API
//!
//! See [`zng_wgt_data_view`] for the full view API.

pub use zng_wgt_data_view::{DataView, DataViewArgs};
