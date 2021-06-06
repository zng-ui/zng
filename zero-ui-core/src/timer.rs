//! App timers.

use core::fmt;
use std::{
    cell::Cell,
    rc::Rc,
    time::{Duration, Instant},
};

use crate::{
    context::AppContext,
    var::{var, ForceReadOnlyVar, RcVar, Var, VarObj, Vars, VarsRead},
};

/// App timers.
pub struct Timers {
    deadlines: Vec<RcVar<TimeoutInfo>>,
    timers: Vec<RcVar<TimerInfo>>,
    #[allow(clippy::type_complexity)]
    deadline_handlers: Vec<(TimeoutHandler, Box<dyn FnOnce(&mut AppContext)>)>,
    #[allow(clippy::type_complexity)]
    timer_handlers: Vec<(TimerHandler, Box<dyn FnMut(&mut AppContext, &TimerInfo)>)>,
}
impl Timers {
    pub(crate) fn new() -> Self {
        Timers {
            deadlines: vec![],
            timers: vec![],
            deadline_handlers: vec![],
            timer_handlers: vec![],
        }
    }

    /// Returns a variable that will update once when the `deadline` is reached.
    ///
    /// If the `deadline` is in the past the variable will still update once in the next app update.
    #[inline]
    #[must_use]
    pub fn deadline(&mut self, deadline: Instant) -> TimeoutVar {
        let timer = var(TimeoutInfo { deadline, elapsed: false });
        self.deadlines.push(timer.clone());
        timer.into_read_only()
    }

    /// Returns a variable that will update once when the `timeout` has elapsed.
    #[inline]
    #[must_use]
    pub fn timeout(&mut self, timeout: Duration) -> TimeoutVar {
        self.deadline(Instant::now() + timeout)
    }

    /// Returns a [`TimerVar`] that will update every time the `interval` elapses.
    ///
    /// The timer can be controlled using methods in the variable value.
    #[inline]
    #[must_use]
    pub fn interval(&mut self, interval: Duration) -> TimerVar {
        let timer = var(TimerInfo {
            state: Rc::new(Self::timer_state(interval)),
        });
        self.timers.push(timer.clone());
        timer.into_read_only()
    }

    /// Returns a [`Timer`] that will update every time the `interval` elapses.
    ///
    /// This is similar to [`interval`](Self::interval) but allows control of the timer without needing a [`VarsRead`]
    /// to access to the variable value.
    #[inline]
    #[must_use]
    pub fn interval_timer(&mut self, interval: Duration) -> Timer {
        let info = TimerInfo {
            state: Rc::new(Self::timer_state(interval)),
        };
        let state = Rc::clone(&info.state);
        let timer = var(info);
        self.timers.push(timer.clone());
        Timer {
            var: timer.into_read_only(),
            state,
        }
    }

    fn timer_state(interval: Duration) -> TimerState {
        TimerState {
            status: Cell::new(TimerStatus::Enabled),
            deadline: Cell::new(Instant::now() + interval),
            interval: Cell::new(interval),
            count: Cell::new(0),
        }
    }

    /// Register a `handler` that will be called once when the `deadline` is reached.
    ///
    /// If the `deadline` is in the past the `handler` will be called in the next app update.
    pub fn on_deadline<F: FnOnce(&mut AppContext) + 'static>(&mut self, deadline: Instant, handler: F) -> TimeoutHandler {
        let h = TimeoutHandler(Rc::new(TimeoutHandlerInfo {
            deadline,
            forget: Cell::new(false),
        }));
        self.deadline_handlers.push((h.clone(), Box::new(handler)));
        h
    }

    /// Register a `handler` that will be called once when `timeout` elapses.
    pub fn on_timeout<F: FnOnce(&mut AppContext) + 'static>(&mut self, timeout: Duration, handler: F) -> TimeoutHandler {
        self.on_deadline(Instant::now() + timeout, handler)
    }

    /// Register a `handler` that will be called every time the `interval` elapses.
    pub fn on_interval<F: FnMut(&mut AppContext, &TimerInfo) + 'static>(&mut self, interval: Duration, handler: F) -> TimerHandler {
        let h = TimerHandler(Rc::new(TimerHandlerInfo {
            state: Self::timer_state(interval),
            forget: Cell::new(false),
        }));
        self.timer_handlers.push((h.clone(), Box::new(handler)));
        h
    }

    /// Update timers, returns new app wake time.
    pub(crate) fn apply_updates(&mut self, vars: &Vars) -> Option<Instant> {
        let now = Instant::now();
        self.deadlines.retain(|t| {
            let retain = t.strong_count() > 1;
            if retain && t.get(vars).deadline <= now {
                t.modify(vars, |t| t.elapsed = true);
            }
            retain
        });

        self.timers.retain(|t| {
            let info = t.get(vars);
            let retain = t.strong_count() > 1 && !info.destroyed();
            if retain && info.enabled() && info.enabled() && info.deadline() <= now {
                info.state.deadline.set(now + info.interval());
                info.state.count.set(info.state.count.get().wrapping_add(1));
                t.modify(vars, |_| {});
            } else {
                info.destroy();
            }
            retain
        });

        self.deadlines
            .iter()
            .map(|t| t.get(vars).deadline)
            .chain(self.timers.iter().map(|t| t.get(vars).deadline()))
            .min()
    }
}

/// Represents the state of a [`TimeoutVar`].
#[derive(Debug, Clone)]
pub struct TimeoutInfo {
    /// Deadline for the timer to elapse, this value does not change.
    pub deadline: Instant,
    /// If the timer has elapsed, the initial value is `false`, once the timer elapses the value is updated to `true`.
    pub elapsed: bool,
}

/// A [`timeout`](Timers::timeout) or [`deadline`](Timers::deadline) timer.
///
/// This is a variable of type [`TimeoutInfo`], it will update once when the timer elapses.
///
/// Drop all clones of this variable to cancel the timer.
pub type TimeoutVar = ForceReadOnlyVar<TimeoutInfo, RcVar<TimeoutInfo>>;

/// Represents the state of a [`TimerVar`].
#[derive(Debug, Clone)]
pub struct TimerInfo {
    state: Rc<TimerState>,
}
macro_rules! timer_methods {
    ($Type:ident, $self:ident => $state:expr) => {
        impl $Type {
            /// Gets the current timer interval.
            #[inline]
            pub fn interval(&$self) -> Duration {
                let state = $state;
                state.interval.get()
            }

            /// Change the timer interval.
            #[inline]
            pub fn set_interval(&$self, interval: Duration) -> Result<(), TimerDestroyed> {
                let state = $state;
                state.set_interval(interval)
            }

            /// Next timer deadline.
            #[inline]
            pub fn deadline(&$self) -> Instant {
                let state = $state;
                state.deadline.get()
            }

            /// Number of times the timer elapsed since it was created or [`restart`](Self::restart).
            #[inline]
            pub fn count(&$self) -> usize {
                let state = $state;
                state.count.get()
            }

            /// If the timer was destroyed.
            #[inline]
            pub fn destroyed(&$self) -> bool {
                let state = $state;
                state.destroyed()
            }

            /// Permanently stop the timer.
            ///
            /// This unregisters the timer in [`Timers`], the same as if all clones of the timer are dropped.
            #[inline]
            pub fn destroy(&$self) {
                let state = $state;
                state.status.set(TimerStatus::Destroyed);
            }

            /// Stop the timer but keep it registered.
            #[inline]
            pub fn stop(&$self) -> Result<(), TimerDestroyed> {
                let state = $state;
                state.stop()
            }

            /// Starts the timer.
            ///
            /// This resets the [`deadline`](Self::deadline) but continues the [`count`](Self::count).
            #[inline]
            pub fn start(&$self) -> Result<(), TimerDestroyed> {
                let state = $state;
                state.start()
            }

            /// Restarts the timer.
            ///
            /// This restarts the [`deadline`](Self::deadline) and the [`count`](Self::count).
            #[inline]
            pub fn restart(&$self) -> Result<(), TimerDestroyed> {
                let state = $state;
                state.restart()
            }

            /// If the timer is not stopped nor destroyed.
            #[inline]
            pub fn enabled(&$self) -> bool {
                let state = $state;
                state.enabled()
            }
        }
    };
}
timer_methods!(TimerInfo, self => &self.state);

#[derive(Debug)]
struct TimerState {
    status: Cell<TimerStatus>,
    interval: Cell<Duration>,
    deadline: Cell<Instant>,
    count: Cell<usize>,
}
impl TimerState {
    pub fn stop(&self) -> Result<(), TimerDestroyed> {
        if let TimerStatus::Destroyed = self.status.get() {
            Err(TimerDestroyed)
        } else {
            self.status.set(TimerStatus::Disabled);
            Ok(())
        }
    }

    fn start(&self) -> Result<(), TimerDestroyed> {
        if let TimerStatus::Destroyed = self.status.get() {
            Err(TimerDestroyed)
        } else {
            self.status.set(TimerStatus::Enabled);
            self.deadline.set(Instant::now() + self.interval.get());
            Ok(())
        }
    }

    fn restart(&self) -> Result<(), TimerDestroyed> {
        if let TimerStatus::Destroyed = self.status.get() {
            Err(TimerDestroyed)
        } else {
            self.status.set(TimerStatus::Enabled);
            self.deadline.set(Instant::now() + self.interval.get());
            self.count.set(0);
            Ok(())
        }
    }

    fn set_interval(&self, interval: Duration) -> Result<(), TimerDestroyed> {
        if let TimerStatus::Destroyed = self.status.get() {
            Err(TimerDestroyed)
        } else {
            self.interval.set(interval);
            Ok(())
        }
    }

    fn enabled(&self) -> bool {
        matches!(self.status.get(), TimerStatus::Enabled)
    }

    fn destroyed(&self) -> bool {
        matches!(self.status.get(), TimerStatus::Destroyed)
    }
}
#[derive(Clone, Copy, Debug)]
enum TimerStatus {
    Enabled,
    Disabled,
    Destroyed,
}

/// Error when an attempt is made to modify a destroyed [`Timer`] or [`TimerVar`].
#[derive(Debug, Clone, Copy)]
pub struct TimerDestroyed;
impl fmt::Display for TimerDestroyed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "cannot change timer because it its destroyed")
    }
}
impl std::error::Error for TimerDestroyed {}

/// An [`interval`](Timers::interval) timer.
///
/// THis is a variable of type [`TimerInfo`], it will update every time the timer elapses.
///
/// Drop all clones of this variable to stop the timer, you can also control the timer
/// with methods in the [timer value](TimerInfo).
pub type TimerVar = ForceReadOnlyVar<TimerInfo, RcVar<TimerInfo>>;

/// A controller for a [`TimerVar`].
///
/// The [`TimerVar`] can be controlled using its value, this `struct` allows controlling the
/// var without access to [`VarsRead`] to get the value.
#[derive(Clone)]
pub struct Timer {
    var: TimerVar,
    state: Rc<TimerState>,
}
impl fmt::Debug for Timer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Timer").field("var", &()).field("state", &self.state).finish()
    }
}
impl Timer {
    /// Construct a timer controller for the `var`.
    pub fn from_var(var: TimerVar, vars: &VarsRead) -> Timer {
        Timer {
            state: Rc::clone(&var.get(vars).state),
            var,
        }
    }

    /// The variable that updates every time the timer elapses.
    #[inline]
    pub fn var(&self) -> &TimerVar {
        &self.var
    }
}
timer_methods!(Timer, self => &self.state);

/// A [`on_timeout`](Timers::on_timeout) or [`on_deadline`](Timers::on_deadline) handler.
///
/// Drop all clones of this handler to cancel the timer, or call [`forget`](Self::forget) to drop the handler
/// without cancelling the timer.
#[derive(Clone)]
#[must_use = "the timer is canceled if the handler is dropped"]
pub struct TimeoutHandler(Rc<TimeoutHandlerInfo>);
struct TimeoutHandlerInfo {
    deadline: Instant,
    forget: Cell<bool>,
}
impl TimeoutHandler {
    /// Drops the handler but does **not** cancel the timer and will still call the handler function when the timer elapses.
    ///
    /// The handler function is still dropped after the timer elapses, this does not work like [`std::mem::forget`].
    #[inline]
    pub fn forget(self) {
        self.0.forget.set(true);
    }
}

/// A [`on_interval`](Timers::on_interval) handler.
///
/// Drop all clones of this handler to cancel the timer, or call [`forget`](Self::forget) to drop the handler
/// without cancelling the timer.
#[derive(Clone)]
pub struct TimerHandler(Rc<TimerHandlerInfo>);
struct TimerHandlerInfo {
    state: TimerState,
    forget: Cell<bool>,
}
impl TimerHandler {
    /// Drops the handler but does **not** destroy the timer and will still call the handler function every time the timer elapses.
    ///
    /// The handler function is still dropped if the timer is destroyed, this does not work like [`std::mem::forget`]. To destroy
    /// the timer from within the function call [`TimerInfo::destroy`].
    #[inline]
    pub fn forget(self) {
        self.0.forget.set(true);
    }
}
timer_methods!(TimerHandler, self => &self.0.state);
