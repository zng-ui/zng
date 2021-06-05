//! App timers.

use std::{cell::Cell, time::{Duration, Instant}};

use crate::var::{var, ForceReadOnlyVar, RcVar, Var};

/// App timers.
pub struct Timers {
    deadlines: Vec<RcVar<OnceTimerInfo>>,
}
impl Timers {
    /// Returns a variable that will update once when the `deadline` is reached.
    ///
    /// If the `deadline` is in the past the variable will still update once in the next app update.
    #[inline]
    pub fn deadline(&mut self, deadline: Instant) -> OnceTimerVar {
        let timer = var(OnceTimerInfo { deadline, elapsed: false });
        self.deadlines.push(timer.clone());
        timer.into_read_only()
    }

    /// Returns a variable that will update once when the `duration` has elapsed.
    #[inline]
    pub fn once(&mut self, duration: Duration) -> OnceTimerVar {
        self.deadline(Instant::now() + duration)
    }

    pub fn interval(&mut self, duration: Duration) -> TimerVar {
        todo!()
    }
}

/// Represents the state of a [`TimerVar`].
#[derive(Debug, Clone)]
pub struct OnceTimerInfo {
    /// Deadline for the timer to elapse.
    pub deadline: Instant,
    /// If the timer has elapsed.
    pub elapsed: bool,
}

pub type OnceTimerVar = ForceReadOnlyVar<OnceTimerInfo, RcVar<OnceTimerInfo>>;

#[derive(Debug, Clone)]
pub struct IntervalTimerInfo {
    /// Interval duration, the timer updates every `duration`.
    pub duration: Duration,
    /// The number of times the timer has elapsed.
    pub count: usize,
    ctrl: Cell<TimerControl>,
}
impl IntervalTimerInfo {
    pub fn stop(&self) {
        self.ctrl.set(TimerControl::Stop);
    }

    pub fn restart(&self) {
        self.ctrl.set(TimerControl::Restart);
    }
}

#[derive(Clone, Copy, Debug)]
enum TimerControl {
    Continue,
    Stop,
    Restart,
}

pub type TimerVar = ForceReadOnlyVar<IntervalTimerInfo, RcVar<IntervalTimerInfo>>;