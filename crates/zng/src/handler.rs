//! Event handler API.
//!
//! A handler is a closure that takes a *context* and *arguments*, the context can be [`WIDGET`] or the app. The [`Handler<A>`]
//! type supports both synchronous and asynchronous handlers. The handler is not usually instantiated directly, macros are provided
//! for declaring handlers.
//!
//! The handler macros are [`hn!`], [`hn_once!`], [`async_hn!`], [`async_hn_once!`].
//! These macros are built on top of the primitive macros [`clmv!`], [`async_clmv_fn!`] and [`async_clmv_fn_once!`] to
//! provide a very easy way to *clone-move* captured variables into the handler.
//!
//! ```
//! use zng::prelude::*;
//! # fn example() {
//!
//! let last_clicked = var(Txt::from(""));
//! # let _ =
//! Stack!(
//!     top_to_bottom,
//!     5,
//!     ui_vec![
//!         Button! {
//!             child = Text!("hn!");
//!             on_click = hn!(last_clicked, |_| {
//!                 last_clicked.set("hn!");
//!             });
//!         },
//!         Button! {
//!             child = Text!("hn_once!");
//!             on_click = hn_once!(last_clicked, |_| {
//!                 last_clicked.set("hn_once!");
//!             });
//!         },
//!         {
//!             let enabled = var(true);
//!             Button! {
//!                 child = Text!("async_hn!");
//!                 on_click = async_hn!(last_clicked, enabled, |_| {
//!                     last_clicked.set("async_hn!");
//!                     enabled.set(false);
//!                     task::deadline(1.secs()).await;
//!                     enabled.set(true);
//!                 });
//!                 widget::enabled;
//!             }
//!         },
//!         Text!(last_clicked),
//!     ]
//! )
//! # ; }
//! ```
//!
//! App Context
//!
//! When a handler is not set in a widget the [`APP_HANDLER`] contextual service is available !!: TODO docs
//!
//! Args Type Inference
//!
//! The [`Handler<A>`] type is an alias for `Box<dyn FnMut(&A) ...>` by necessity as this is the only way to have a type where
//! the closure args is inferred. !!: TODO limitations
//!
//! [`WIDGET`]: crate::widget::WIDGET
//! [`clmv!`]: crate::clmv
//! [`async_clmv_fn!`]: crate::async_clmv_fn
//! [`async_clmv_fn_once!`]: crate::async_clmv_fn_once
//!
//! # Full API
//!
//! See [`zng_app::handler`] for the full handler API.

pub use zng_app::handler::{APP_HANDLER, AppWeakHandle, ArcHandler, Handler, HandlerExt, async_hn, async_hn_once, hn, hn_once};
