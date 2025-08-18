//! Window layers.
//!
//! The window layers is a z-order stacking panel that fills the window content area, widgets can be inserted
//! with a *z-index* that is the [`LayerIndex`]. Layers can be anchored to a normal widget, positioned relative
//! to it with linked visibility.
//!
//! The [`LAYERS`] service can be used to insert and remove layers, the example below uses it to *toggle* a
//! an adorner positioned relative to the button that inserts and removes it.
//!
//! ```
//! use zng::prelude::*;
//! # let _scope = APP.defaults();
//!
//! let inserted = var(false);
//! let anchored = WidgetId::new_unique();
//! # let _ =
//! Button! {
//!     on_click = hn!(inserted, |_| {
//!         if !inserted.get() {
//!             LAYERS.insert_anchored(
//!                 LayerIndex::ADORNER,
//!                 WIDGET.id(),
//!                 layer::AnchorOffset::out_top(),
//!                 Text! {
//!                     id = anchored;
//!                     txt = "Example";
//!                     widget::background_color = colors::BLUE;
//!                     layout::y = 5;
//!                 },
//!             );
//!         } else {
//!             LAYERS.remove(anchored);
//!         }
//!         inserted.set(!inserted.get());
//!     });
//!     layout::align = layout::Align::CENTER;
//!     child = Text!(inserted.map(|&o| if o { "Remove Layer" } else { "Insert Layer" }.into()));
//! }
//! # ;
//! ```
//!
//! Node operations always apply to the window content first then the layers, even with parallelism enabled,
//! this means that layers always render over the window content and that layer widgets can react to normal widget
//! updates within the same frame.
//!
//! # Full API
//!
//! See [`zng_wgt_layer`] for the full layers API.

pub use zng_wgt_layer::{
    AnchorMode, AnchorOffset, AnchorSize, AnchorTransform, LAYERS, LAYERS_INSERT_CMD, LAYERS_REMOVE_CMD, LayerIndex, adorner, adorner_fn,
};
