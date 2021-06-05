//! App timers.

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::{Duration, Instant},
};

use crate::{
    context::AppContext,
    var::{var, ForceReadOnlyVar, RcVar, Var},
};

/// App timers.
pub struct Timers {
    deadlines: Vec<RcVar<OnceTimerInfo>>,
}
impl Timers {
    /// Returns a variable that will update once when the `deadline` is reached.
    ///
    /// If the `deadline` is in the past the variable will still update once in the next app update.
    #[inline]
    pub fn deadline(&mut self, deadline: Instant) -> TimeoutVar {
        let timer = var(OnceTimerInfo { deadline, elapsed: false });
        self.deadlines.push(timer.clone());
        timer.into_read_only()
    }

    /// Returns a variable that will update once when the `duration` has elapsed.
    #[inline]
    pub fn timeout(&mut self, timeout: Duration) -> TimeoutVar {
        self.deadline(Instant::now() + timeout)
    }

    pub fn interval(&mut self, duration: Duration) -> TimerVar {
        todo!()
    }

    pub fn on_timeout<F: FnOnce(&mut AppContext) + 'static>(&mut self, run: F) -> TimeoutHandler {
        TimeoutHandler(Rc::new(Box::new(run)))
    }

    pub fn on_interval<F: FnMut(&mut AppContext, &IntervalTimerInfo) + 'static>(&mut self, run: F) -> IntervalHandler {
        IntervalHandler(Rc::new(RefCell::new(Box::new(run))))
    }
}

pub struct TimeoutHandler(Rc<dyn FnOnce(&mut AppContext)>);
#[allow(clippy::type_complexity)]
pub struct IntervalHandler(Rc<RefCell<Box<dyn FnMut(&mut AppContext, &IntervalTimerInfo)>>>);

/// Represents the state of a [`TimerVar`].
#[derive(Debug, Clone)]
pub struct OnceTimerInfo {
    /// Deadline for the timer to elapse.
    pub deadline: Instant,
    /// If the timer has elapsed.
    pub elapsed: bool,
}

pub type TimeoutVar = ForceReadOnlyVar<OnceTimerInfo, RcVar<OnceTimerInfo>>;

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
