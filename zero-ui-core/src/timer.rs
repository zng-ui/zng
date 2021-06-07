//! App timers.

use core::fmt;
use std::{
    cell::Cell,
    mem,
    rc::Rc,
    time::{Duration, Instant},
};

use retain_mut::RetainMut;

use crate::{
    context::AppContext,
    var::{var, ForceReadOnlyVar, RcVar, Var, VarObj, Vars, VarsRead},
};

struct DeadlineHandlerEntry {
    handle: TimeoutHandle,
    handler: Option<Box<dyn FnOnce(&mut AppContext)>>,
    pending: bool,
}

struct TimerHandlerEntry {
    handle: TimerHandle,
    handler: Box<dyn FnMut(&mut AppContext, &TimerArgs)>,
    pending: bool,
}

/// App timers.
pub struct Timers {
    deadlines: Vec<RcVar<TimeoutInfo>>,
    timers: Vec<RcVar<TimerInfo>>,
    deadline_handlers: Vec<DeadlineHandlerEntry>,
    timer_handlers: Vec<TimerHandlerEntry>,
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
            status: Cell::new(TimerStatus::Enable),
            deadline: Cell::new(Instant::now() + interval),
            interval: Cell::new(interval),
            count: Cell::new(0),
        }
    }

    /// Register a `handler` that will be called once when the `deadline` is reached.
    ///
    /// If the `deadline` is in the past the `handler` will be called in the next app update.
    pub fn on_deadline<F: FnOnce(&mut AppContext) + 'static>(&mut self, deadline: Instant, handler: F) -> TimeoutHandle {
        let h = TimeoutHandle(Rc::new(TimeoutHandleData {
            deadline,
            forget: Cell::new(false),
        }));
        self.deadline_handlers.push(DeadlineHandlerEntry {
            handle: h.clone(),
            handler: Some(Box::new(handler)),
            pending: false,
        });
        h
    }

    /// Register a `handler` that will be called once when `timeout` elapses.
    pub fn on_timeout<F: FnOnce(&mut AppContext) + 'static>(&mut self, timeout: Duration, handler: F) -> TimeoutHandle {
        self.on_deadline(Instant::now() + timeout, handler)
    }

    /// Register a `handler` that will be called every time the `interval` elapses.
    pub fn on_interval<F: FnMut(&mut AppContext, &TimerArgs) + 'static>(&mut self, interval: Duration, handler: F) -> TimerHandle {
        let h = TimerHandle(Rc::new(TimerHandleData {
            args: TimerArgs {
                state: Self::timer_state(interval),
            },
            forget: Cell::new(false),
        }));
        self.timer_handlers.push(TimerHandlerEntry {
            handle: h.clone(),
            handler: Box::new(handler),
            pending: false,
        });
        h
    }

    /// Update timers, returns new app wake time.
    pub(crate) fn apply_updates(&mut self, vars: &Vars) -> Option<Instant> {
        let now = Instant::now();

        let mut min_next_some = false;
        let mut min_next = now + Duration::from_secs(60 * 60 * 60);

        self.deadlines.retain(|t| {
            let mut retain = t.strong_count() > 1;
            let deadline = t.get(vars).deadline;
            if retain && deadline <= now {
                t.modify(vars, |t| t.elapsed = true);
                retain = false;
            } else {
                min_next_some = true;
                min_next = min_next.min(deadline);
            }
            retain
        });

        self.timers.retain(|t| {
            let info = t.get(vars);
            let retain = t.strong_count() > 1 && !info.stopped();
            if retain && info.enabled() && info.enabled() {
                if info.deadline() <= now {
                    info.state.deadline.set(now + info.interval());
                    info.state.count.set(info.state.count.get().wrapping_add(1));
                    t.modify(vars, |_| {});
                }
                min_next_some = true;
                min_next = min_next.min(info.state.deadline.get());
            } else {
                info.stop();
            }
            retain
        });

        self.deadline_handlers.retain_mut(|e| {
            let retain = e.handle.0.forget.get() || Rc::strong_count(&e.handle.0) > 1;
            if retain {
                e.pending = e.handle.0.deadline <= now;
                if !e.pending {
                    min_next_some = true;
                    min_next = min_next.min(e.handle.0.deadline);
                }
            }
            retain
        });

        self.timer_handlers.retain_mut(|e| {
            let retain = !e.handle.0.args.state.stopped() && (e.handle.0.forget.get() || Rc::strong_count(&e.handle.0) > 1);
            if retain {
                let state = &e.handle.0.args.state;
                e.pending = state.deadline.get() <= now;
                if e.pending {
                    state.deadline.set(now + state.interval.get());
                    state.count.set(state.count.get().wrapping_add(1));
                }
                min_next_some = true;
                min_next = min_next.min(state.deadline.get());
            }
            retain
        });

        if min_next_some {
            Some(min_next)
        } else {
            None
        }
    }

    pub(crate) fn notify(ctx: &mut AppContext) {
        let mut handlers = mem::take(&mut ctx.timers.deadline_handlers);
        handlers.retain_mut(|h| {
            if h.pending {
                h.handler.take().unwrap()(ctx);
            }
            !h.pending
        });
        handlers.extend(ctx.timers.deadline_handlers.drain(..));
        ctx.timers.deadline_handlers = handlers;

        let mut handlers = mem::take(&mut ctx.timers.timer_handlers);
        handlers.retain_mut(|h| {
            if h.pending {
                (h.handler)(ctx, &h.handle.0.args);
                h.pending = false;
            }
            !h.handle.0.args.state.stopped()
        });
        handlers.extend(ctx.timers.timer_handlers.drain(..));
        ctx.timers.timer_handlers = handlers;
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
            pub fn set_interval(&$self, interval: Duration) -> Result<(), TimerStopped> {
                let state = $state;
                state.set_interval(interval)
            }

            /// Next timer deadline.
            #[inline]
            pub fn deadline(&$self) -> Instant {
                let state = $state;
                state.deadline.get()
            }

            /// Number of times the timer elapsed since it was created or reset.
            #[inline]
            pub fn count(&$self) -> usize {
                let state = $state;
                state.count.get()
            }

            /// If the timer is permanently stopped.
            #[inline]
            pub fn stopped(&$self) -> bool {
                let state = $state;
                state.stopped()
            }

            /// Permanently stop the timer.
            ///
            /// This drops the timer in [`Timers`].
            #[inline]
            pub fn stop(&$self) {
                let state = $state;
                state.status.set(TimerStatus::Stop);
            }

            /// Stop the timer but keep it registered.
            #[inline]
            pub fn disable(&$self) -> Result<(), TimerStopped> {
                let state = $state;
                state.disable()
            }

            /// Starts the timer again.
            #[inline]
            pub fn enable(&$self) -> Result<(), TimerStopped> {
                let state = $state;
                state.enable()
            }

            /// Reset the timer.
            ///
            /// This restarts the [`deadline`](Self::deadline) and the [`count`](Self::count).
            #[inline]
            pub fn reset(&$self) -> Result<(), TimerStopped> {
                let state = $state;
                state.reset()
            }

            /// If the timer is not stopped nor destroyed.
            #[inline]
            pub fn enabled(&$self) -> bool {
                let state = $state;
                state.enabled()
            }

            /// Enable or disable the timer.
            #[inline]
            pub fn set_enabled(&$self, enabled: bool) -> Result<(), TimerStopped> {
                if enabled {
                    $self.enable()
                } else {
                    $self.disable()
                }
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
    fn disable(&self) -> Result<(), TimerStopped> {
        if let TimerStatus::Stop = self.status.get() {
            Err(TimerStopped)
        } else {
            self.status.set(TimerStatus::Disable);
            Ok(())
        }
    }

    fn enable(&self) -> Result<(), TimerStopped> {
        if let TimerStatus::Stop = self.status.get() {
            Err(TimerStopped)
        } else {
            self.status.set(TimerStatus::Enable);
            Ok(())
        }
    }

    fn reset(&self) -> Result<(), TimerStopped> {
        if let TimerStatus::Stop = self.status.get() {
            Err(TimerStopped)
        } else {
            self.status.set(TimerStatus::Enable);
            self.deadline.set(Instant::now() + self.interval.get());
            self.count.set(0);
            Ok(())
        }
    }

    fn set_interval(&self, interval: Duration) -> Result<(), TimerStopped> {
        if let TimerStatus::Stop = self.status.get() {
            Err(TimerStopped)
        } else {
            self.interval.set(interval);
            Ok(())
        }
    }

    fn enabled(&self) -> bool {
        matches!(self.status.get(), TimerStatus::Enable)
    }

    fn stopped(&self) -> bool {
        matches!(self.status.get(), TimerStatus::Stop)
    }
}
#[derive(Clone, Copy, Debug)]
enum TimerStatus {
    Enable,
    Disable,
    Stop,
}

/// Error when an attempt is made to modify a permanently stopped [`Timer`] or [`TimerVar`].
#[derive(Debug, Clone, Copy)]
pub struct TimerStopped;
impl fmt::Display for TimerStopped {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "cannot change timer because it its permanently stopped")
    }
}
impl std::error::Error for TimerStopped {}

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

/// Represents a [`on_timeout`](Timers::on_timeout) or [`on_deadline`](Timers::on_deadline) handler.
///
/// Drop all clones of this handle to cancel the timer, or call [`forget`](Self::forget) to drop the handle
/// without cancelling the timer.
#[derive(Clone)]
#[must_use = "the timer is canceled if the handler is dropped"]
pub struct TimeoutHandle(Rc<TimeoutHandleData>);
struct TimeoutHandleData {
    deadline: Instant,
    forget: Cell<bool>,
}
impl TimeoutHandle {
    /// Drops the handle but does **not** drop the timeout handler closure.
    ///
    /// This method does not work like [`std::mem::forget`], **no memory is leaked**, the handle
    /// memory is released immediately and the handler memory is released when the time elapses or
    /// application shuts-down.
    #[inline]
    pub fn forget(self) {
        self.0.forget.set(true);
    }
}

/// Represents a [`on_interval`](Timers::on_interval) handler.
///
/// Drop all clones of this handler to cancel the timer, or call [`forget`](Self::forget) to drop the handler
/// without cancelling the timer.
#[derive(Clone)]
#[must_use = "the timer is stopped if the handler is dropped"]
pub struct TimerHandle(Rc<TimerHandleData>);
struct TimerHandleData {
    args: TimerArgs,
    forget: Cell<bool>,
}
impl TimerHandle {
    /// Drops the handler but does **not** drop the timer handler closure.
    ///
    /// This method does not work like [`std::mem::forget`], **no memory is leaked**, the handle
    /// memory is released immediately and the handler memory is released when the timer is stopped or
    /// application shuts-down.
    #[inline]
    pub fn forget(self) {
        self.0.forget.set(true);
    }
}
timer_methods!(TimerHandle, self => &self.0.args.state);

/// Arguments for a [`on_interval`](Timers::on_interval) handler.
pub struct TimerArgs {
    state: TimerState,
}
timer_methods!(TimerArgs, self => &self.state);
