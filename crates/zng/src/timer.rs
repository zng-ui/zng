//! App timers service and other types.
//!
//! The [`TIMERS`] service provides timers that operate directly off the app main loop.
//!
//! The example below creates a timer that elapses every 1 second using [`TIMERS.interval`]. The timer is a variable
//! so it can be mapped to a value, in the example this is used to create a countdown variable that counts from
//! 10 to 0 and then stops the timer.
//!
//! ```
//! use zng::prelude::*;
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
//! Note that you can also use the [`task::deadline`] function to `.await` a deadline, in app threads this function
//! uses the [`TIMERS`] service too.
//!
//! [`task::deadline`]: crate::task::deadline
//! [`TIMERS.interval`]: TIMERS::interval
//!
//! # Full API
//!
//! See [`zng_app::timer`] for the full time API.
//!

pub use zng_app::timer::{
    DeadlineArgs, DeadlineHandle, DeadlineVar, TIMERS, Timer, TimerArgs, TimerHandle, TimerVar, WeakDeadlineHandle, WeakTimerHandle,
};
