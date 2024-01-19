//! Keyboard service, properties, events and types.
//!
//! The example below defines a window that shows the pressed keys and prints the key state changes.
//!
//! ```
//! use zero_ui::prelude::*;
//! # let _scope = APP.defaults();
//!
//! # let _ =
//! Window! {
//!     child = Text!(keyboard::KEYBOARD.keys().map_debug());
//!     keyboard::on_key_input = hn!(|args: &keyboard::KeyInputArgs| {
//!         println!("key {:?} {:?}", args.key, args.state);
//!     });
//! }
//! # ;
//! ```
//!
//! Keyboard events are send to the focused widget, if there is no focused widget no event is send. You can
//! subscribe directly to the [`KEY_INPUT_EVENT`] to monitor all keyboard events for any focused widget.
//!
//! # Full API
//!
//! See [`zero_ui_ext_input::keyboard`] and [`zero_ui_wgt_input::keyboard`] for the full keyboard API.
//! See [`zero_ui_app::view_process::raw_events`] for raw keyboard events that are processed to generate the key input event.

pub use zero_ui_app::shortcut::ModifiersState;

pub use zero_ui_ext_input::keyboard::{
    HeadlessAppKeyboardExt, Key, KeyCode, KeyInputArgs, KeyRepeatConfig, KeyState, ModifiersChangedArgs, NativeKeyCode, KEYBOARD,
    KEY_INPUT_EVENT, MODIFIERS_CHANGED_EVENT,
};

pub use zero_ui_wgt_input::keyboard::{
    on_disabled_key_input, on_key_down, on_key_input, on_key_up, on_pre_disabled_key_input, on_pre_key_down, on_pre_key_input,
    on_pre_key_up,
};

/// Raw keyboard hardware events, received independent of what window or widget is focused.
///
/// You must enable device events in the app to receive this events.
///
/// ```no_run
/// use zero_ui::prelude::*;
///
/// APP.defaults().enable_device_events().run_window(async {
///     keyboard::raw_device_events::KEY_EVENT.on_pre_event(app_hn!(|args: &keyboard::raw_device_events::KeyArgs, _| {
///         if args.state == keyboard::KeyState::Pressed {
///             println!("key pressed {:?}", args.key_code);
///         }
///     })).perm();
/// });
/// ```
pub mod raw_device_events {
    pub use zero_ui_app::view_process::raw_device_events::{KeyArgs, TextArgs, KEY_EVENT, TEXT_EVENT};
}
