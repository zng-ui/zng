//! Panel layout widget.
//!
//! The [`Panel!`](struct@Panel) widget allows widgets to
//!
//! ```
//! use zero_ui::prelude::*;
//! # let _scope = APP.defaults();
//!
//! let layouts = [
//!     (
//!         "Stack!",
//!         wgt_fn!(|args: zero_ui::panel::PanelArgs| {
//!             Stack! {
//!                 direction = StackDirection::top_to_bottom();
//!                 spacing = 5;
//!                 children = args.children;
//!             }
//!         }),
//!     ),
//!     (
//!         "Wrap!",
//!         wgt_fn!(|args: zero_ui::panel::PanelArgs| {
//!             Wrap! {
//!                 spacing = 5;
//!                 children = args.children;
//!             }
//!         }),
//!     ),
//!     (
//!         "Grid!",
//!         wgt_fn!(|args: zero_ui::panel::PanelArgs| {
//!             Grid! {
//!                 columns = ui_vec![grid::Column!(100.pct())];
//!                 auto_grow_fn = wgt_fn!(|_| grid::Row!(1.lft()));
//!                 spacing = 5;
//!                 cells = args.children;
//!             }
//!         }),
//!     ),
//! ];
//! let selected_layout = var(0usize);
//!
//! # let _ =
//! zero_ui::panel::Panel! {
//!     children = layouts.iter().enumerate().map(|(i, (name, _))| {
//!         Toggle! {
//!             value::<usize> = i;
//!             child = Text!(*name);
//!             grid::cell::at = grid::cell::AT_AUTO;
//!         }
//!     }).collect::<UiNodeVec>();
//!     toggle::selector = toggle::Selector::single(selected_layout.clone());
//!
//!     panel_fn = selected_layout.map(move |&i| layouts[i].1.clone());
//! }
//! # ;
//! ```
//!
//! # Full API
//!
//! See [`zero_ui_wgt_panel`] for the full widget API.

pub use zero_ui_wgt_panel::{node, panel_fn, Panel, PanelArgs};
