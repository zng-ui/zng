//! Asynchronous task running, timers, event and variable channels.

use crate::var::{response_var, var, ForceReadOnlyVar, RcVar, ResponderVar, ResponseVar, Vars};

use super::{context::AppSyncContext, var::Var};
use retain_mut::*;
use std::{
    fmt,
    future::Future,
    sync::{atomic::AtomicBool, Arc},
    time::{Duration, Instant},
};

/// Asynchronous task running, timers, event and variable channels.
pub struct Sync {
    once_timers: Vec<OnceTimer>,
    interval_timers: Vec<IntervalTimer>,

    new_wake_time: Option<Instant>,
}
impl Sync {
    pub(super) fn new() -> Self {
        Sync {
            once_timers: vec![],
            interval_timers: vec![],
            new_wake_time: None,
        }
    }

    pub(super) fn update(&mut self, ctx: &mut AppSyncContext) -> Option<Instant> {
        let now = Instant::now();

        self.once_timers.retain(|t| t.retain(now, ctx.vars));
        self.interval_timers.retain_mut(|t| t.retain(now, ctx.vars));

        let mut wake_time = None;

        for t in &self.once_timers {
            if let Some(wake_time) = &mut wake_time {
                if t.due_time < *wake_time {
                    *wake_time = t.due_time;
                }
            } else {
                wake_time = Some(t.due_time);
            }
        }
        for t in &self.interval_timers {
            if let Some(wake_time) = &mut wake_time {
                if t.due_time < *wake_time {
                    *wake_time = t.due_time;
                }
            } else {
                wake_time = Some(t.due_time);
            }
        }

        wake_time
    }

    /// Run a CPU bound task.
    ///
    /// The task runs in a [`rayon`] thread-pool, this function is not blocking.
    ///
    /// # Example
    ///
    /// ```
    /// # use zero_ui_core::{context::WidgetContext, var::{ResponseVar, response_var}};
    /// # struct SomeStruct { sum_response: ResponseVar<usize> }
    /// # impl SomeStruct {
    /// fn on_event(&mut self, ctx: &mut WidgetContext) {
    ///     let (responder, response) = response_var();
    ///     self.sum_response = response;
    ///     let sender = ctx.vars.sender(responder);
    ///     self.sum_response = ctx.sync.run(move ||{
    ///         let r = (0..1000).sum();
    ///         sender.send_response(r);
    ///     });
    /// }
    ///
    /// fn on_update(&mut self, ctx: &mut WidgetContext) {
    ///     if let Some(result) = self.sum_response.response_new(ctx.vars) {
    ///         println!("sum of 0..1000: {}", result);   
    ///     }
    /// }
    /// # }
    /// ```
    pub fn run<T: FnOnce() + Send + 'static>(&mut self, task: T) {
        rayon::spawn(task);
    }

    /// Run an IO bound task.
    ///
    /// The task runs in an [`async-global-executor`] thread-pool, this function is not blocking.
    ///
    /// # Example
    ///
    /// ```
    /// # use zero_ui_core::{context::WidgetContext, var::{ResponseVar, response_var}};
    /// # struct SomeStruct { file_response: ResponseVar<Vec<u8>> }
    /// # impl SomeStruct {
    /// fn on_event(&mut self, ctx: &mut WidgetContext) {
    ///     let (responder, response) = response_var();
    ///     self.file_response = response;
    ///     let sender = ctx.vars.sender(responder);
    ///     self.file_response = ctx.sync.run_async(async move {
    ///         todo!("use async_std to read a file");
    ///         let file = vec![];
    ///         sender.send(file);    
    ///     });
    /// }
    ///
    /// fn on_update(&mut self, ctx: &mut WidgetContext) {
    ///     if let Some(result) = self.file_response.response_new(ctx.vars) {
    ///         println!("file loaded: {} bytes", result.len());   
    ///     }
    /// }
    /// # }
    /// ```
    pub fn run_async<T: Future<Output = ()> + Send + 'static>(&mut self, task: T) {
        // TODO run block-on?
        async_global_executor::spawn(task).detach();
    }

    fn update_wake_time(&mut self, due_time: Instant) {
        if let Some(already) = &mut self.new_wake_time {
            if due_time < *already {
                *already = due_time;
            }
        } else {
            self.new_wake_time = Some(due_time);
        }
    }

    /// Gets a response var that updates once after the `duration`.
    ///
    /// The response will update once at the moment of now + duration or a little later.
    #[inline]
    pub fn update_after(&mut self, duration: Duration) -> ResponseVar<TimeElapsed> {
        self.update_when(Instant::now() + duration)
    }

    /// Gets a response var that updates once after the number of milliseconds.
    ///
    /// The response will update once at the moment of now + duration or a little later.
    #[inline]
    pub fn update_after_millis(&mut self, millis: u64) -> ResponseVar<TimeElapsed> {
        self.update_after(Duration::from_millis(millis))
    }

    /// Gets a response var that updates once after the number of seconds.
    ///
    /// The response will update once at the moment of now + duration or a little later.
    #[inline]
    pub fn update_after_secs(&mut self, secs: u64) -> ResponseVar<TimeElapsed> {
        self.update_after(Duration::from_secs(secs))
    }

    /// Gets a response var that updates every `interval`.
    ///
    /// The var will update after every interval elapse.
    pub fn update_every(&mut self, interval: Duration) -> TimerVar {
        let (timer, var) = IntervalTimer::new(interval);
        self.update_wake_time(timer.due_time);
        self.interval_timers.push(timer);
        var
    }

    /// Gets a var that updated every *n* seconds.
    ///
    // The var will update after every interval elapse.
    #[inline]
    pub fn update_every_secs(&mut self, secs: u64) -> TimerVar {
        self.update_every(Duration::from_secs(secs))
    }

    /// Gets a var that updated every *n* milliseconds.
    ///
    // The var will update after every interval elapse.
    #[inline]
    pub fn update_every_millis(&mut self, millis: u64) -> TimerVar {
        self.update_every(Duration::from_millis(millis))
    }

    /// Gets a response var that updates once when `time` is reached.
    ///
    /// The response will update once at the moment of now + duration or a little later.
    pub fn update_when(&mut self, time: Instant) -> ResponseVar<TimeElapsed> {
        let (timer, response) = OnceTimer::new(time);
        self.update_wake_time(timer.due_time);
        self.once_timers.push(timer);
        response
    }
}

/// Message of a [`Sync`] timer listener.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TimeElapsed {
    /// Moment the timer notified.
    pub timestamp: Instant,
}
impl fmt::Debug for TimeElapsed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            f.debug_struct("TimeElapsed").field("timestamp", &self.timestamp).finish()
        } else {
            write!(f, "{:?}", self.timestamp)
        }
    }
}

struct OnceTimer {
    due_time: Instant,
    responder: ResponderVar<TimeElapsed>,
}
impl OnceTimer {
    fn new(due_time: Instant) -> (Self, ResponseVar<TimeElapsed>) {
        let (responder, response) = response_var();
        (OnceTimer { due_time, responder }, response)
    }

    /// Notifies the listeners if the timer elapsed.
    ///
    /// Returns if the timer is still active, once timer deactivate
    /// when they elapse or when there are no more listeners alive.
    fn retain(&self, now: Instant, vars: &Vars) -> bool {
        if self.responder.strong_count() == 1 {
            return false;
        }

        let elapsed = self.due_time <= now;
        if elapsed {
            self.responder.respond(vars, TimeElapsed { timestamp: now });
        }

        !elapsed
    }
}

/// A variable that is set every time an [interval timer](Sync::update_every) elapses.
pub type TimerVar = ForceReadOnlyVar<TimerArgs, RcVar<TimerArgs>>;

/// Value of [`TimerVar`].
#[derive(Clone, Debug)]
pub struct TimerArgs {
    /// Moment the timer notified.
    pub timestamp: Instant,

    stop: Arc<AtomicBool>,
}
impl TimerArgs {
    fn now() -> Self {
        Self {
            timestamp: Instant::now(),
            stop: Arc::default(),
        }
    }

    /// Stop the timer.
    #[inline]
    pub fn stop(&self) {
        self.stop.store(true, std::sync::atomic::Ordering::Relaxed);
    }

    fn stop_requested(&self) -> bool {
        self.stop.load(std::sync::atomic::Ordering::Relaxed)
    }
}

struct IntervalTimer {
    due_time: Instant,
    interval: Duration,
    responder: RcVar<TimerArgs>,
}
impl IntervalTimer {
    fn new(interval: Duration) -> (Self, TimerVar) {
        let responder = var(TimerArgs::now());
        let response = responder.clone().into_read_only();
        (
            IntervalTimer {
                due_time: Instant::now() + interval,
                interval,
                responder,
            },
            response,
        )
    }

    /// Notifier the listeners if the time elapsed and resets the timer.
    ///
    /// Returns if the timer is still active, interval timers deactivate
    /// when there are no more listeners alive.
    fn retain(&mut self, now: Instant, vars: &Vars) -> bool {
        if self.responder.strong_count() == 1 || self.responder.get(vars).stop_requested() {
            return false;
        }
        if self.due_time <= now {
            self.responder.modify(vars, move |t| t.timestamp = now);
            self.due_time = now + self.interval;
        }

        true
    }
}
