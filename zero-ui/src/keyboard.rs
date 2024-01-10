//! Keyboard service, properties, events and types.
//!
//! # Full API
//!
//! See [`zero_ui_ext_input::keyboard`] and [`zero_ui_wgt_input::keyboard`] for the full keyboard API.

pub use zero_ui_app::shortcut::ModifiersState;

pub use zero_ui_ext_input::keyboard::{
    HeadlessAppKeyboardExt, Key, KeyCode, KeyInputArgs, KeyRepeatConfig, KeyState, ModifiersChangedArgs, NativeKeyCode, KEYBOARD,
    KEY_INPUT_EVENT, MODIFIERS_CHANGED_EVENT,
};

pub use zero_ui_wgt_input::keyboard::{
    on_disabled_key_input, on_key_down, on_key_input, on_key_up, on_pre_disabled_key_input, on_pre_key_down, on_pre_key_input,
    on_pre_key_up,
};
