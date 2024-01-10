//! App timers service and types.
//!
//! # Full API
//!
//! See [`zero_ui_app::timer`] for the full time API. Also see [`task::deadline`] for a timer decoupled from the app loop.
//! 
//! [`task::deadline`]: crate::task::deadline

pub use zero_ui_app::timer::{
    DeadlineArgs, DeadlineHandle, DeadlineVar, Timer, TimerArgs, TimerHandle, TimerVar, WeakDeadlineHandle, WeakTimerHandle, TIMERS,
};
