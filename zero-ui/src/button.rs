//! Button widget, styles and properties.
//!
//! A simple clickable container widget, it can be used by directly handling the click events or by setting it to
//! operate a [`Command`].
//! 
//! [`Command`]: crate::event::Command
//! 
//! # Click Events
//! 
//! The button widget implements the [`gesture::on_click`] event so you can use it directly, but like any
//! other widget all events can be set. The example below demonstrates both ways of setting events.
//! 
//! [`gesture::on_click`]: fn@crate::gesture::on_click
//! 
//! ```
//! use zero_ui::prelude::*;
//!
//! # let _scope = APP.defaults();
//! let count = var(0u8);
//! # let _ =
//! Button! {
//!     child = Text!(count.map(|c| match *c {
//!         0 => Txt::from("Click Me!"),
//!         1 => Txt::from("Clicked 1 time."),
//!         n => formatx!("Clicked {n} times."),
//!     }));
//!     on_click = hn!(count, |_| {
//!         count.set(count.get() + 1);
//!     });
//!     gesture::on_pre_click = hn!(|args: &gesture::ClickArgs| {
//!         if count.get() == 10 {
//!             args.propagation().stop();
//!             count.set(0u8);
//!         }
//!     });
//! }
//! # ;
//! ```
//! 
//! # Command
//! 
//! Instead of handling events directly the button widget can be set to represents a command.
//! If the [`cmd`](struct@Button#cmd) property is set the button widget will automatically set properties
//! from command metadata, you can manually set some of these properties to override the command default.
//! 
//! ```
//! use zero_ui::prelude::*;
//! 
//! # let _scope = APP.defaults();
//! # let _ =
//! Stack!(left_to_right, 5, ui_vec![
//!     // shorthand
//!     Button!(zero_ui::clipboard::COPY_CMD),
//!     // cmd with custom child
//!     Button! {
//!         cmd = zero_ui::clipboard::PASTE_CMD;
//!         child = Text!("Custom Label");
//!     },
//! ])
//! # ;
//! ```
//! 
//! The properties a command button sets are documented in the [`cmd`](struct@Button#cmd) property docs.
//!
//! <details>
//! <summary>Equivalent command button.</summary>
//! 
//! This example shows an equivalent command button implementation, for a single command.
//! There are some differences, the real `cmd` is a variable so commands can dynamically change and
//! the handlers also pass on the[`cmd_param`](struct@Button#cmd_param) if set.
//! 
//! ```
//! # use zero_ui::prelude::*;
//! let cmd = zero_ui::clipboard::COPY_CMD;
//! # let _scope = APP.defaults(); let _ = 
//! Button! {
//!     child = Text!(cmd.name());
//!     widget::enabled = cmd.is_enabled();
//!     widget::visibility = cmd.has_handlers().map_into();
//!     on_click = hn!(|args: &gesture::ClickArgs| {
//!         if cmd.is_enabled_value() {
//!             args.propagation().stop();
//!             cmd.notify();
//!         }
//!     });
//!     gesture::on_disabled_click = hn!(|args: &gesture::ClickArgs| {
//!         if !cmd.is_enabled_value() {
//!             args.propagation().stop();
//!             cmd.notify();
//!         }
//!     });
//! }
//! # ;
//! ```
//! 
//! </details>
//! 
//! # Style
//! 
//! TODO, also mention BUTTON.cmd.
//! 
//! ## Base Colors
//! 
//! # Full API
//!
//! See [`zero_ui_wgt_button`] for the full widget API.

pub use zero_ui_wgt_button::{base_colors, extend_style, replace_style, Button, DefaultStyle, BUTTON};

pub use zero_ui_wgt_link::LinkStyle;
