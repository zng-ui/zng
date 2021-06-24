//! Event handler properties, [`on_click`](fn@gesture::on_click), [`on_key_down`](fn@keyboard::on_key_down),
//! and more.
//!
//! # Route
//!
//! Events are broadcasted to the UI tree, starting at the root node and visiting all leaf nodes, depth first. Event properties
//! however only raise their event handler if it is relevant for their widget, so most events follow a *route* from the root parent
//! to the target widget and back to the root parent. The route going to the target is called the *preview* route and the route going
//! back to root is the main route.
//!
//! Most events are represented by two properties that represents these two routes, for example, the [`ClickEvent`](crate::core::gesture::ClickEvent)
//! is represented by the [`on_pre_click`](fn@gesture::on_pre_click) in the preview route and by [`on_click`](fn@gesture::on_click) in
//! the main route. Usually you handle [`on_click`](fn@gesture::on_click) in the widget that is expected to be clicked, but you can
//! use [`on_pre_click`](fn@gesture::on_pre_click) to *preview* the event in a parent widget and potentially stop it from being raised
//! in the main event handler by calling [`stop_propagation`](EventArgs::stop_propagation).
//!
//! # Handlers
//!
//! A property event handler is any type that implements [`WidgetHandler`](crate::core::handler::WidgetHandler), usually they are a
//! closure declared with the assistance of a macro, the widget handler macros are [`hn!`](crate::core::handler::hn!),
//! [`hn_once!`](crate::core::handler::hn_once!), [`async_hn!`](crate::core::handler::async_hn!) and
//! [`async_hn_once!`](crate::core::handler::async_hn_once!).
//!
//! ```
//! use zero_ui::prelude::*;
//!
//! let txt = var_from("Click Me!");
//! let btn = button! {
//!     content = text(txt.clone());
//!     on_click = hn!(|ctx, _| {
//!         txt.set(ctx, "Clicked!");
//!     });
//! };
//! ```
//!
//! To use `.await` just use one of the async handlers:
//!
//! ```
//! use zero_ui::prelude::*;
//!
//! let enabled = var(true);
//! let btn = button! {
//!     content = text(enabled.map(|&e| if e { "Click Me!".to_text() } else { "Busy..".to_text() }));
//!     on_click = async_hn!(enabled, |ctx, _| {
//!         enabled.set(&ctx, false);
//!         let data = task::wait(|| std::fs::read_to_string("data.txt")).await;
//!         if let Ok(data) = data {
//!             println!("Data: {}", data);
//!         }
//!         enabled.set(&ctx, true);
//!     });
//!     enabled;
//! };

use crate::core::event::*;

pub mod gesture;
pub mod keyboard;
pub mod mouse;
pub mod widget;
pub mod window;
