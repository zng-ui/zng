//! Gesture service, properties, events, shortcuts and other types.
//!
//! A gesture is an event that is generated from multiple lower-level events. A shortcut is a gesture generated
//! from one or more keyboard inputs, a click is also a gesture generated from mouse clicks, accessibility clicks,
//! touch taps and some shortcuts. In essence, events, types and states that aggregate multiple difference sources
//! are found here, gestures generated from a single event source are defined in other modules, for example touch gestures
//! are defined in [`touch`](crate::touch).
//!
//! ```
//! use zng::prelude::*;
//!
//! # fn example() {
//! # let _ =
//! Window! {
//!     gesture::on_click = hn!(|args| {
//!         use gesture::ClickArgsSource::*;
//!         match args.source {
//!             Mouse { .. } => println!("mouse click"),
//!             Touch { .. } => println!("touch tap"),
//!             Shortcut { .. } => println!("shortcut press"),
//!             Access { .. } => println!("access click"),
//!         }
//!     });
//! }
//! # ; }
//! ```
//!
//! The example above handles the click gesture on a window and prints what underlying event was interpreted as a click.
//!
//! # Full API
//!
//! See [`zng_ext_input::gesture`] and [`zng_wgt_input::gesture`] for the full gesture API
//! and [`zng_app::shortcut`] for the shortcut API.
//!
//! [`zng_app::shortcut`]: mod@zng_app::shortcut

pub use zng_ext_input::gesture::{
    CLICK_EVENT, ClickArgs, ClickArgsSource, CommandShortcutMatchesExt, GESTURES, HeadlessAppGestureExt, SHORTCUT_EVENT, ShortcutActions,
    ShortcutArgs, ShortcutClick, ShortcutsHandle, WeakShortcutsHandle,
};

pub use zng_app::shortcut::{
    CommandShortcutExt, GestureKey, KeyChord, KeyGesture, ModifierGesture, Shortcut, ShortcutFilter, Shortcuts, shortcut,
};

pub use zng_wgt_input::gesture::{
    click_shortcut, context_click_shortcut, on_any_click, on_any_double_click, on_any_single_click, on_any_triple_click, on_click,
    on_context_click, on_disabled_click, on_double_click, on_pre_any_click, on_pre_any_double_click, on_pre_any_single_click,
    on_pre_any_triple_click, on_pre_click, on_pre_context_click, on_pre_disabled_click, on_pre_double_click, on_pre_single_click,
    on_pre_triple_click, on_single_click, on_triple_click,
};

pub use zng_wgt_input::{
    is_cap_hovered, is_cap_pointer_pressed, is_cap_pressed, is_hovered, is_hovered_disabled, is_pointer_active, is_pressed,
};
