//! Event handler API.
//!
//! A handler is a closure that takes a *context* and *arguments*, the context can be [`WIDGET`] or the app,
//! handler types implement [`WidgetHandler`] or [`AppHandler`] respectively. These traits allow a single caller
//! to support multiple different flavors of handlers, both synchronous and asynchronous, and both `FnMut` and `FnOnce` all
//! by implementing a single entry point.
//!
//! Macros are provided for declaring the various flavors of handlers, [`hn!`], [`hn_once!`], [`async_hn!`], [`async_hn_once!`]
//! for widget contexts and [`app_hn!`], [`app_hn_once!`], [`async_app_hn!`], [`async_app_hn_once!`] for the app context. These
//! macros also build on top of the primitive macros [`clmv!`], [`async_clmv_fn!`] and [`async_clmv_fn_once!`] to
//! provide a very easy way to *clone-move* captured variables into the handler.
//!
//! ```
//! use zng::prelude::*;
//! # let _scope = APP.defaults();
//!
//! let last_clicked = var(Txt::from(""));
//! # let _ =
//! Stack!(top_to_bottom, 5, ui_vec![
//!     Button! {
//!         child = Text!("hn!");
//!         on_click = hn!(last_clicked, |_| {
//!             last_clicked.set("hn!");
//!         });
//!     },
//!     Button! {
//!         child = Text!("hn_once!");
//!         on_click = hn_once!(last_clicked, |_| {
//!             last_clicked.set("hn_once!");
//!         });
//!     },
//!     {
//!         let enabled = var(true);
//!         Button! {
//!             child = Text!("async_hn!");
//!             on_click = async_hn!(last_clicked, enabled, |_| {
//!                 last_clicked.set("async_hn!");
//!                 enabled.set(false);
//!                 task::deadline(1.secs()).await;
//!                 enabled.set(true);
//!             });
//!             widget::enabled;
//!         }
//!     },
//!     Text!(last_clicked),
//! ])
//! # ;
//! ```
//!
//! [`WIDGET`]: crate::widget::WIDGET
//! [`clmv!`]: crate::clmv
//! [`async_clmv_fn!`]: crate::async_clmv_fn
//! [`async_clmv_fn_once!`]: crate::async_clmv_fn_once
//!
//! # Full API
//!
//! See [`zng_app::handler`] for the full handler API.

pub use zng_app::handler::{
    AppHandler, AppHandlerArgs, AppWeakHandle, FilterAppHandler, FilterWidgetHandler, WidgetHandler, app_hn, app_hn_once, async_app_hn,
    async_app_hn_once, async_hn, async_hn_once, hn, hn_once,
};
