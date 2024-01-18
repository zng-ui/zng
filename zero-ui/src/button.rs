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
//! Of particular importance is the [`widget::visibility`], it is set so that the button is only visible if
//! the command has any handlers, enabled or disabled, this is done because commands are considered irrelevant
//! in the current context if they don't even have a disabled handler. The example above will only be
//! visible if you set handlers for those commands.
//!
//! ```
//! # use zero_ui::prelude::*;
//! # let _scope = APP.defaults();
//! # fn cmd_btn_example() -> impl UiNode { widget::node::NilUiNode }
//! # let _ =
//! zero_ui::clipboard::COPY_CMD.on_event(true, app_hn!(|_, _| { println!("copy") })).perm();
//! zero_ui::clipboard::PASTE_CMD.on_event(true, app_hn!(|_, _| { println!("paste") })).perm();
//! Window! {
//!     child = cmd_btn_example();
//! }
//! # ;
//! ```
//!
//! [`widget::visibility`]: fn@crate::widget::visibility
//!
//! # Style
//!
//! The button widget is styleable, the [`style_fn`](fn@style_fn) property can be set in any parent widget or the button
//! itself to extend or replace the button style.
//!
//! ## Base Colors
//!
//! The default style derive all colors from the [`base_colors`](fn@base_colors), so if you
//! only want to change color of buttons you can use this property.
//!
//! The example below extends the button style to change the button color to red when it represents
//! an specific command.
//!
//! ```
//! use zero_ui::prelude::*;
//! use zero_ui::button;
//!
//! # let _scope = APP.defaults(); let _ =
//! Window! {
//!     button::style_fn = Style! {
//!         when *#{button::BUTTON.cmd()} == Some(window::cmd::CLOSE_CMD) {
//!             button::base_colors = color::ColorPair {
//!                 // dark theme base
//!                 dark: colors::BLACK.with_alpha(80.pct()).mix_normal(colors::RED),
//!                 // light theme base
//!                 light: colors::WHITE.with_alpha(80.pct()).mix_normal(colors::RED),
//!             };
//!         }
//!     };
//! }
//! # ;
//! ```
//!
//! # Full API
//!
//! See [`zero_ui_wgt_button`] for the full widget API.

pub use zero_ui_wgt_button::{base_colors, style_fn, Button, DefaultStyle, BUTTON};

pub use zero_ui_wgt_link::LinkStyle;