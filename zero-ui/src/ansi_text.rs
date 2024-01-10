//! ANSI text widget.
//!
//! This widget displays text styled using [ANSI escape codes], commonly used to style terminal text.
//! 
//! [ANSI escape codes]: https://en.wikipedia.org/wiki/ANSI_escape_code
//! 
//! ```
//! # let _scope = zero_ui::APP.defaults(); let _ =
//! zero_ui::ansi_text::AnsiText! {
//!     txt = "[32;1mGREEN&BOLD[47m";
//! }
//! # ;
//! ```
//! 
//! The example above renders <code style="color:green;font-weight:bold;">GREEN&BOLD</code>.
//! 
//! # Full API
//!
//! See [`zero_ui_wgt_ansi_text`] for the full widget API.

pub use zero_ui_wgt_ansi_text::{
    AnsiColor, AnsiStyle, AnsiText, AnsiTextParser, AnsiTxt, AnsiWeight, LineFnArgs, PageFnArgs, PanelFnArgs, TextFnArgs,
};
