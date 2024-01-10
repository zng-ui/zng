//! Gesture service, properties, events, shortcuts and other types.
//!
//! # Full API
//!
//! See [`zero_ui_ext_input::gesture`] and [`zero_ui_wgt_input::gesture`] for the full gesture API
//! and [`zero_ui_app::shortcut`] for the shortcut API.
//!
//! [`zero_ui_app::shortcut`]: mod@zero_ui_app::shortcut

pub use zero_ui_ext_input::gesture::{
    ClickArgs, ClickArgsSource, CommandShortcutMatchesExt, HeadlessAppGestureExt, ShortcutActions, ShortcutArgs, ShortcutClick,
    ShortcutsHandle, WeakShortcutsHandle, CLICK_EVENT, GESTURES, SHORTCUT_EVENT,
};

pub use zero_ui_app::shortcut::{
    shortcut, CommandShortcutExt, GestureKey, KeyChord, KeyGesture, ModifierGesture, Shortcut, ShortcutFilter, Shortcuts,
};

pub use zero_ui_wgt_input::gesture::{
    click_shortcut, context_click_shortcut, on_any_click, on_any_double_click, on_any_single_click, on_any_triple_click, on_click,
    on_context_click, on_disabled_click, on_double_click, on_pre_any_click, on_pre_any_double_click, on_pre_any_single_click,
    on_pre_any_triple_click, on_pre_click, on_pre_context_click, on_pre_disabled_click, on_pre_double_click, on_pre_single_click,
    on_pre_triple_click, on_single_click, on_triple_click,
};

pub use zero_ui_wgt_input::{is_cap_hovered, is_cap_pointer_pressed, is_cap_pressed, is_hovered, is_hovered_disabled, is_pressed};
