#![cfg(feature = "tooltip")]

//! Tooltip properties and widget.
//!
//! The [`tooltip`](fn@tooltip) and [`tooltip_fn`](fn@tooltip_fn) properties can be set on any widget to git it a tooltip.
//! The tooltip itself can also be any widget, but the [`Tip!`](struct@Tip) widget is recommended. You can also set a tooltip
//! that only appears when the widget is disabled using [`disabled_tooltip`](fn@disabled_tooltip).
//!
//! The example below declares a button that toggles enabled showing different tooltips depending on the state.
//!
//! ```
//! use zng::prelude::*;
//! # let _app = APP.defaults();
//!
//! let enabled = var(true);
//! # let _ =
//! Button! {
//!     tip::tooltip = Tip!(Text!("enabled tooltip"));
//!     tip::disabled_tooltip = Tip!(Text!("disabled tooltip"));
//!
//!     on_click = hn!(enabled, |_| enabled.set(false));
//!     gesture::on_disabled_click = hn!(enabled, |_| enabled.set(true));
//!     child = Text!(enabled.map(|&e| formatx!("enabled = {e}")));
//!     widget::enabled;
//! }
//! # ;
//! ```
//!
//! # Full API
//!
//! See [`zng_wgt_tooltip`] for the full tooltip API.

pub use zng_wgt_tooltip::{
    DefaultStyle, Tip, TooltipArgs, access_tooltip_anchor, access_tooltip_duration, disabled_tooltip, disabled_tooltip_fn, style_fn,
    tooltip, tooltip_anchor, tooltip_context_capture, tooltip_delay, tooltip_duration, tooltip_fn, tooltip_interval,
};
