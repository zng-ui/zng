//! App timers service and types.
//!
//! The [`TIMERS`] service provides timers that operate directly off the app main loop. UI bound timers
//! should prefer using this service instead of [`task::deadline`] as the service does not need spawn a
//! timer executor and naturally awakes the main loop when elapsed. The downside is that timer precision is
//! tied to the app, timers can only elapse once per frame so they are only as precise as the frame rate.
//! 
//! The example below creates a timer that elapses every 1 second using [`TIMERS.interval`]. The timer is a variable
//! so it can be mapped to a value, in the example this is used to create a countdown variable that counts from
//! 10 to 0 and then stops the timer.
//! 
//! ```
//! use zero_ui::prelude::*;
//! # let _scope = APP.defaults();
//! 
//! let countdown = timer::TIMERS.interval(1.secs(), false).map(move |t| {
//!     let count = 10 - t.count();
//!     if count == 0 {
//!         t.stop();
//!     }
//!     count
//! });
//! ```
//!
//! [`task::deadline`]: crate::task::deadline
//! [`TIMERS.interval`]: TIMERS::interval
//!
//! # Full API
//!
//! See [`zero_ui_app::timer`] for the full time API.
//!

pub use zero_ui_app::timer::{
    DeadlineArgs, DeadlineHandle, DeadlineVar, Timer, TimerArgs, TimerHandle, TimerVar, WeakDeadlineHandle, WeakTimerHandle, TIMERS,
};
