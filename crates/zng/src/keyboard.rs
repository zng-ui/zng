//! Keyboard service, properties, events and other types.
//!
//! The example below defines a window that shows the pressed keys and prints the key state changes.
//!
//! ```
//! use zng::prelude::*;
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
//! See [`zng_ext_input::keyboard`] and [`zng_wgt_input::keyboard`] for the full keyboard API.
//! See [`zng_app::view_process::raw_events`] for raw keyboard events that are processed to generate the key input event.

pub use zng_app::shortcut::ModifiersState;

pub use zng_ext_input::keyboard::{
    HeadlessAppKeyboardExt, KEY_INPUT_EVENT, KEYBOARD, Key, KeyCode, KeyInputArgs, KeyLocation, KeyRepeatConfig, KeyState,
    MODIFIERS_CHANGED_EVENT, ModifiersChangedArgs, NativeKeyCode,
};

pub use zng_wgt_input::keyboard::{
    on_disabled_key_input, on_key_down, on_key_input, on_key_up, on_pre_disabled_key_input, on_pre_key_down, on_pre_key_input,
    on_pre_key_up,
};

/// Raw keyboard hardware events, received independent of what window or widget is focused.
///
/// You must enable device events in the app to receive this events.
///
/// ```no_run
/// use zng::prelude::*;
///
/// APP.defaults().enable_device_events().run_window(async {
///     keyboard::raw_device_events::KEY_EVENT.on_pre_event(app_hn!(|args: &keyboard::raw_device_events::KeyArgs, _| {
///         if args.state == keyboard::KeyState::Pressed {
///             println!("key pressed {:?}", args.key_code);
///         }
///     })).perm();
///
///     Window!()
/// });
/// ```
pub mod raw_device_events {
    pub use zng_app::view_process::raw_device_events::{KEY_EVENT, KeyArgs};
    #[allow(deprecated)] // TODO(breaking)
    pub use zng_app::view_process::raw_device_events::{TEXT_EVENT, TextArgs};
}
