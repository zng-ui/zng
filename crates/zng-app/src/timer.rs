//! App timers, deadlines and timeouts.
//!
//! The primary `struct` of this module is [`TIMERS`]. You can use it to
//! create UI bound timers that run using only the main thread and can awake the app event loop
//! to notify updates.

use crate::{
    Deadline,
    handler::{Handler, HandlerExt as _},
};
use parking_lot::Mutex;
use std::{
    fmt, mem,
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    task::Waker,
    time::Duration,
};
use zng_app_context::app_local;
use zng_handle::{Handle, HandleOwner, WeakHandle};
use zng_time::{DInstant, INSTANT, INSTANT_APP};
use zng_var::{Var, WeakVar, var};

use crate::{LoopTimer, handler::AppWeakHandle, update::UPDATES};

struct DeadlineHandlerEntry {
    handle: HandleOwner<DeadlineState>,
    handler: Mutex<Box<dyn FnMut(&dyn AppWeakHandle) + Send>>, // not actually locked, just makes this Sync
    pending: bool,
}

struct TimerHandlerEntry {
    handle: HandleOwner<TimerState>,
    handler: Mutex<Box<dyn FnMut(&TimerArgs, &dyn AppWeakHandle) + Send>>, // not actually locked, just makes this Sync
    pending: Option<Deadline>,                                             // the last expected deadline
}

struct WaitDeadline {
    deadline: Deadline,
    wakers: Mutex<Vec<Waker>>,
}
struct WaitDeadlineFut(Arc<WaitDeadline>);
impl Future for WaitDeadlineFut {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        if self.0.deadline.has_elapsed() {
            std::task::Poll::Ready(())
        } else {
            let waker = cx.waker().clone();
            self.0.wakers.lock().push(waker);
            std::task::Poll::Pending
        }
    }
}

struct TimerVarEntry {
    handle: HandleOwner<TimerState>,
    weak_var: WeakVar<Timer>,
}

app_local! {
    pub(crate) static TIMERS_SV: TimersService = const { TimersService::new() };
}

pub(crate) struct TimersService {
    deadlines: Vec<WeakVar<Deadline>>,
    wait_deadlines: Vec<std::sync::Weak<WaitDeadline>>,
    timers: Vec<TimerVarEntry>,
    deadline_handlers: Vec<DeadlineHandlerEntry>,
    timer_handlers: Vec<TimerHandlerEntry>,
    has_pending_handlers: bool,
}
impl TimersService {
    const fn new() -> Self {
        Self {
            deadlines: vec![],
            wait_deadlines: vec![],
            timers: vec![],
            deadline_handlers: vec![],
            timer_handlers: vec![],
            has_pending_handlers: false,
        }
    }

    fn deadline(&mut self, deadline: Deadline) -> DeadlineVar {
        let timer = var(deadline);
        self.deadlines.push(timer.downgrade());
        UPDATES.send_awake();
        timer.read_only()
    }

    fn wait_deadline(&mut self, deadline: Deadline) -> impl Future<Output = ()> + Send + Sync + use<> {
        let deadline = Arc::new(WaitDeadline {
            deadline,
            wakers: Mutex::new(vec![]),
        });
        self.wait_deadlines.push(Arc::downgrade(&deadline));
        UPDATES.send_awake();
        WaitDeadlineFut(deadline)
    }

    fn interval(&mut self, interval: Duration, paused: bool) -> TimerVar {
        let (owner, handle) = TimerHandle::new(interval, paused);
        let timer = var(Timer(handle));
        self.timers.push(TimerVarEntry {
            handle: owner,
            weak_var: timer.downgrade(),
        });
        UPDATES.send_awake();
        timer.read_only()
    }

    fn on_deadline(&mut self, deadline: Deadline, mut handler: Handler<DeadlineArgs>) -> DeadlineHandle {
        let (handle_owner, handle) = DeadlineHandle::new(deadline);
        self.deadline_handlers.push(DeadlineHandlerEntry {
            handle: handle_owner,
            handler: Mutex::new(Box::new(move |handle| {
                handler.app_event(
                    handle.clone_boxed(),
                    true,
                    &DeadlineArgs {
                        timestamp: INSTANT.now(),
                        deadline,
                    },
                );
            })),
            pending: false,
        });
        UPDATES.send_awake();
        handle
    }

    fn on_interval(&mut self, interval: Duration, paused: bool, mut handler: Handler<TimerArgs>) -> TimerHandle {
        let (owner, handle) = TimerHandle::new(interval, paused);

        self.timer_handlers.push(TimerHandlerEntry {
            handle: owner,
            handler: Mutex::new(Box::new(move |args, handle| {
                handler.app_event(handle.clone_boxed(), true, args);
            })),
            pending: None,
        });
        UPDATES.send_awake();
        handle
    }

    pub(crate) fn next_deadline(&self, timer: &mut LoopTimer) {
        for wk in &self.deadlines {
            if let Some(var) = wk.upgrade() {
                timer.register(var.get());
            }
        }

        for wk in &self.wait_deadlines {
            if let Some(e) = wk.upgrade() {
                timer.register(e.deadline);
            }
        }

        for t in &self.timers {
            if let Some(var) = t.weak_var.upgrade()
                && !t.handle.is_dropped()
                && !t.handle.data().paused.load(Ordering::Relaxed)
            {
                // not dropped and not paused
                var.with(|t| {
                    let deadline = t.0.0.data().deadline.lock();
                    timer.register(deadline.current_deadline());
                });
            }
        }

        for e in &self.deadline_handlers {
            if !e.handle.is_dropped() {
                let deadline = e.handle.data().deadline;
                timer.register(deadline);
            }
        }

        for t in &self.timer_handlers {
            if !t.handle.is_dropped() {
                let state = t.handle.data();
                if !state.paused.load(Ordering::Relaxed) {
                    let deadline = state.deadline.lock();
                    timer.register(deadline.current_deadline());
                }
            }
        }
    }

    /// if the last `apply_updates` observed elapsed timers.
    pub(crate) fn has_pending_updates(&self) -> bool {
        self.has_pending_handlers
    }

    /// Update timer vars, flag handlers to be called in [`Self::notify`], returns new app wake time.
    pub(crate) fn apply_updates(&mut self, timer: &mut LoopTimer) {
        let now = INSTANT.now();

        // update `deadline` vars
        self.deadlines.retain(|wk| {
            if let Some(var) = wk.upgrade() {
                if !timer.elapsed(var.get()) {
                    return true; // retain
                }

                var.update();
            }
            false // don't retain
        });

        // update `wait_deadline` vars
        self.wait_deadlines.retain(|wk| {
            if let Some(e) = wk.upgrade() {
                if !e.deadline.has_elapsed() {
                    return true; // retain
                }
                for w in mem::take(&mut *e.wakers.lock()) {
                    w.wake();
                }
            }
            false // don't retain
        });

        // update `interval` vars
        self.timers.retain(|t| {
            if let Some(var) = t.weak_var.upgrade()
                && !t.handle.is_dropped()
            {
                if !t.handle.data().paused.load(Ordering::Relaxed) {
                    var.with(|t| {
                        let mut deadline = t.0.0.data().deadline.lock();

                        if timer.elapsed(deadline.current_deadline()) {
                            t.0.0.data().count.fetch_add(1, Ordering::Relaxed);
                            var.update();

                            deadline.last = now;
                            timer.register(deadline.current_deadline());
                        }
                    })
                }

                return true; // retain, var is alive and did not call stop.
            }
            false // don't retain.
        });

        // flag `on_deadline` handlers that need to run.
        self.deadline_handlers.retain_mut(|e| {
            if e.handle.is_dropped() {
                return false; // cancel
            }

            let deadline = e.handle.data().deadline;
            e.pending = timer.elapsed(deadline);

            self.has_pending_handlers |= e.pending;

            true // retain if not canceled, elapsed deadlines will be dropped in [`Self::notify`].
        });

        // flag `on_interval` handlers that need to run.
        self.timer_handlers.retain_mut(|e| {
            if e.handle.is_dropped() {
                return false; // stop
            }

            let state = e.handle.data();
            if !state.paused.load(Ordering::Relaxed) {
                let mut deadline = state.deadline.lock();

                if timer.elapsed(deadline.current_deadline()) {
                    state.count.fetch_add(1, Ordering::Relaxed);
                    e.pending = Some(deadline.current_deadline());
                    self.has_pending_handlers = true;

                    deadline.last = now;
                    timer.register(deadline.current_deadline());
                }
            }

            true // retain if stop was not called
        });
    }

    /// does on_* notifications.
    pub(crate) fn notify() {
        let _s = tracing::trace_span!("TIMERS").entered();

        let _t = INSTANT_APP.pause_for_update();

        // we need to detach the handlers, so we can pass the context for then
        // so we `mem::take` for the duration of the call. But new timers can be registered inside
        // the handlers, so we add those handlers using `extend`.

        let mut timers = TIMERS_SV.write();

        if !mem::take(&mut timers.has_pending_handlers) {
            return;
        }

        // call `on_deadline` handlers.
        let mut handlers = mem::take(&mut timers.deadline_handlers);
        drop(timers);
        handlers.retain_mut(|h| {
            if h.pending {
                (h.handler.get_mut())(&h.handle.weak_handle());
                h.handle.data().executed.store(true, Ordering::Relaxed);
            }
            !h.pending // drop if just called, deadline handlers are *once*.
        });
        let mut timers = TIMERS_SV.write();
        handlers.append(&mut timers.deadline_handlers);
        timers.deadline_handlers = handlers;

        // call `on_interval` handlers.
        let mut handlers = mem::take(&mut timers.timer_handlers);
        drop(timers);
        handlers.retain_mut(|h| {
            if let Some(deadline) = h.pending.take() {
                let args = TimerArgs {
                    timestamp: INSTANT.now(),
                    deadline,
                    wk_handle: h.handle.weak_handle(),
                };
                (h.handler.get_mut())(&args, &h.handle.weak_handle());
            }

            !h.handle.is_dropped() // drop if called stop inside the handler.
        });
        let mut timers = TIMERS_SV.write();
        handlers.append(&mut timers.timer_handlers);
        timers.timer_handlers = handlers;
    }
}

/// App timers, deadlines and timeouts.
///
/// You can use this service to create UI bound timers, these timers run using only the app loop and awake the app
/// to notify updates.
///
/// Timer updates can be observed using variables that update when the timer elapses, or you can register
/// handlers to be called directly when the time elapses. Timers can be *one-time*, updating only once when
/// a [`deadline`] is reached; or they can update every time on a set [`interval`].
///
/// Note that you can also use the [`task::deadline`](zng_task::deadline) function to `.await` deadlines, in app
/// threads this function uses the `TIMERS` service too.
///
/// # Precision
///
/// Timers elapse at the specified time or a little later, depending on how busy the app main loop is. High frequency
/// timers can also have an effective lower frequency of updates because timers only elapse once per frame cycle.
///
/// [variable]: Var
/// [`deadline`]: TIMERS::deadline
/// [`interval`]: TIMERS::interval
pub struct TIMERS;
impl TIMERS {
    /// Returns a [`DeadlineVar`] that will update once when the `deadline` is reached.
    ///
    /// If the `deadline` is in the past the variable will still update once in the next app update.
    /// Drop all clones of the variable to cancel the timer.
    ///
    /// ```
    /// # use zng_app::timer::*;
    /// # use zng_app::handler::*;
    /// # use zng_layout::unit::*;
    /// # use zng_app::var::*;
    /// # use std::time::Instant;
    /// # fn foo() {
    /// let deadline = TIMERS.deadline(20.secs());
    ///
    /// # let
    /// text = deadline.map(|d| if d.has_elapsed() { "20 seconds have passed" } else { "..." });
    /// # }
    /// ```
    ///
    /// In the example above the deadline variable will update 20 seconds later when the deadline [`has_elapsed`]. The variable
    /// is read-only and will only update once.
    ///
    /// [`has_elapsed`]: Deadline::has_elapsed
    #[must_use]
    pub fn deadline(&self, deadline: impl Into<Deadline>) -> DeadlineVar {
        TIMERS_SV.write().deadline(deadline.into())
    }

    /// Returns a [`TimerVar`] that will update every time the `interval` elapses.
    ///
    /// The timer can be controlled using methods in the variable value. The timer starts
    /// running immediately if `paused` is `false`.
    ///
    /// ```
    /// # use zng_app::timer::*;
    /// # use zng_app::handler::*;
    /// # use zng_layout::unit::*;
    /// # use zng_app::var::*;
    /// # use zng_txt::*;
    /// # use std::time::Instant;
    /// # fn foo() {
    /// let timer = TIMERS.interval(1.secs(), false);
    ///
    /// # let
    /// text = timer.map(|t| match t.count() {
    ///     0 => formatx!(""),
    ///     1 => formatx!("1 second elapsed"),
    ///     c => formatx!("{c} seconds elapsed"),
    /// });
    /// # }
    /// ```
    ///
    /// In the example above the timer variable will update every second, the variable keeps a [`count`](Timer::count)
    /// of times the time elapsed, that is incremented every update. The variable is read-only but the value can
    /// be used to control the timer to some extent, see [`TimerVar`] for details.
    #[must_use]
    pub fn interval(&self, interval: Duration, paused: bool) -> TimerVar {
        TIMERS_SV.write().interval(interval, paused)
    }

    /// Register a `handler` that will be called once when the `deadline` is reached.
    ///
    /// If the `deadline` is in the past the `handler` will be called in the next app update.
    ///
    /// ```
    /// # use zng_app::timer::*;
    /// # use zng_app::handler::*;
    /// # use zng_layout::unit::*;
    /// # use std::time::Instant;
    /// # fn foo() {
    /// let handle = TIMERS.on_deadline(
    ///     20.secs(),
    ///     hn_once!(|_| {
    ///         println!("20 seconds have passed");
    ///     }),
    /// );
    /// # }
    /// ```
    ///
    /// # Handler
    ///
    /// The `handler` can be any of the *once* [`Handler<A>`] flavors. You can use the macros
    /// [`hn_once!`](crate::handler::hn_once!) or [`async_hn_once!`](crate::handler::async_hn_once!)
    /// to declare a handler closure.
    ///
    /// Async handlers execute up to the first `.await` immediately when the `deadline` is reached, subsequent awakes
    /// are scheduled like an async *preview* event handler.
    ///
    /// # Handle
    ///
    /// Returns a [`DeadlineHandle`] that can be used to cancel the timer, either by dropping the handle or by
    /// calling [`cancel`](DeadlineHandle::cancel). You can also call [`perm`](DeadlineHandle::perm)
    /// to drop the handle without cancelling.
    pub fn on_deadline(&self, deadline: impl Into<Deadline>, handler: Handler<DeadlineArgs>) -> DeadlineHandle {
        TIMERS_SV.write().on_deadline(deadline.into(), handler)
    }

    /// Register a `handler` that will be called every time the `interval` elapses.
    ///
    /// The timer starts running immediately if `paused` is `false`.
    pub fn on_interval(&self, interval: Duration, paused: bool, handler: Handler<TimerArgs>) -> TimerHandle {
        TIMERS_SV.write().on_interval(interval, paused, handler)
    }
}

impl TIMERS {
    /// Implementation of the [`task::deadline`] function when called from app threads.
    ///
    /// [`task::deadline`]: zng_task::deadline
    pub fn wait_deadline(&self, deadline: impl Into<Deadline>) -> impl Future<Output = ()> + Send + Sync + 'static {
        TIMERS_SV.write().wait_deadline(deadline.into())
    }
}

/// A [`deadline`](TIMERS::deadline) timer.
///
/// This is a read-only variable of type [`Deadline`], it will update once when the timer elapses.
///
/// Drop all clones of this variable to cancel the timer.
///
/// ```
/// # use zng_app::timer::*;
/// # use zng_app::handler::*;
/// # use zng_layout::unit::*;
/// # use zng_app::var::*;
/// # use std::time::Instant;
/// # fn foo() {
/// let deadline: DeadlineVar = TIMERS.deadline(20.secs());
///
/// # let
/// text = deadline.map(|d| if d.has_elapsed() { "20 seconds have passed" } else { "..." });
/// # }
/// ```
///
/// In the example above the variable is mapped to a text, there are many other things you can do with variables,
/// including `.await` for the update in UI bound async tasks. See [`Var<T>`] for details.
///
/// [`Var<T>`]: zng_var::Var
pub type DeadlineVar = Var<Deadline>;

/// Represents a [`on_deadline`](TIMERS::on_deadline) handler.
///
/// Drop all clones of this handle to cancel the timer, or call [`perm`](Self::perm) to drop the handle
/// without cancelling the timer.
#[derive(Clone, PartialEq, Eq, Hash)]
#[repr(transparent)]
#[must_use = "the timer is canceled if the handler is dropped"]
pub struct DeadlineHandle(Handle<DeadlineState>);
struct DeadlineState {
    deadline: Deadline,
    executed: AtomicBool,
}
impl DeadlineHandle {
    /// Create a handle to nothing, the handle always in the *canceled* state.
    ///
    /// Note that `Option<DeadlineHandle>` takes up the same space as `DeadlineHandle` and avoids an allocation.
    pub fn dummy() -> DeadlineHandle {
        DeadlineHandle(Handle::dummy(DeadlineState {
            deadline: Deadline(DInstant::EPOCH),
            executed: AtomicBool::new(false),
        }))
    }

    fn new(deadline: Deadline) -> (HandleOwner<DeadlineState>, Self) {
        let (owner, handle) = Handle::new(DeadlineState {
            deadline,
            executed: AtomicBool::new(false),
        });
        (owner, DeadlineHandle(handle))
    }

    /// Drops the handle but does **not** drop the handler closure.
    ///
    /// The handler closure will be dropped after it is executed or when the app exits.
    pub fn perm(self) {
        self.0.perm();
    }

    /// If [`perm`](Self::perm) was called in another handle.
    ///
    /// If `true` the closure will be dropped when it executes, when the app exits or if [`cancel`](Self::cancel) is called.
    pub fn is_permanent(&self) -> bool {
        self.0.is_permanent()
    }

    /// Drops the handle and forces the handler to drop.
    ///
    /// If the deadline has not been reached the handler will not be called, and will drop in the next app update.
    pub fn cancel(self) {
        self.0.force_drop();
    }

    /// The timeout deadline.
    ///
    /// The handler is called once when this deadline is reached.
    pub fn deadline(&self) -> Deadline {
        self.0.data().deadline
    }

    /// If the handler has executed. The handler executes once when the deadline is reached.
    pub fn has_executed(&self) -> bool {
        self.0.data().executed.load(Ordering::Relaxed)
    }

    /// If the timeout handler will never execute. Returns `true` if [`cancel`](Self::cancel) was called
    /// before the handler could execute.
    pub fn is_canceled(&self) -> bool {
        !self.has_executed() && self.0.is_dropped()
    }

    /// Create a weak handle to the deadline.
    pub fn downgrade(&self) -> WeakDeadlineHandle {
        WeakDeadlineHandle(self.0.downgrade())
    }
}
impl fmt::Debug for DeadlineHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DeadlineHandle")
            .field("deadline", &self.deadline())
            .field("handle", &self.0)
            .field(
                "state",
                &if self.has_executed() {
                    "has_executed"
                } else if self.is_canceled() {
                    "is_canceled"
                } else {
                    "awaiting"
                },
            )
            .finish()
    }
}

/// Weak [`DeadlineHandle`]
#[derive(Clone, PartialEq, Eq, Hash, Default, Debug)]
pub struct WeakDeadlineHandle(WeakHandle<DeadlineState>);
impl WeakDeadlineHandle {
    /// New weak handle that does not upgrade.
    pub fn new() -> Self {
        Self(WeakHandle::new())
    }

    /// Get the strong handle is still waiting the deadline.
    pub fn upgrade(&self) -> Option<DeadlineHandle> {
        self.0.upgrade().map(DeadlineHandle)
    }
}

/// Arguments for the handler of [`on_deadline`](TIMERS::on_deadline).
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct DeadlineArgs {
    /// When the handler was called.
    pub timestamp: DInstant,
    /// Timer deadline, is less-or-equal to the [`timestamp`](Self::timestamp).
    pub deadline: Deadline,
}

/// Represents a [`on_interval`](TIMERS::on_interval) handler.
///
/// Drop all clones of this handler to stop the timer, or call [`perm`](Self::perm) to drop the handler
/// without cancelling the timer.
#[derive(Clone, PartialEq, Eq, Hash)]
#[repr(transparent)]
#[must_use = "the timer is stopped if the handler is dropped"]
pub struct TimerHandle(Handle<TimerState>);
struct TimerState {
    paused: AtomicBool,
    deadline: Mutex<TimerDeadline>,
    count: AtomicUsize,
}
struct TimerDeadline {
    interval: Duration,
    last: DInstant,
}
impl TimerDeadline {
    fn current_deadline(&self) -> Deadline {
        Deadline(self.last + self.interval)
    }
}
impl TimerHandle {
    fn new(interval: Duration, paused: bool) -> (HandleOwner<TimerState>, TimerHandle) {
        let (owner, handle) = Handle::new(TimerState {
            paused: AtomicBool::new(paused),
            deadline: Mutex::new(TimerDeadline {
                interval,
                last: INSTANT.now(),
            }),
            count: AtomicUsize::new(0),
        });
        (owner, TimerHandle(handle))
    }

    /// Create a handle to nothing, the handle is always in the *stopped* state.
    ///
    /// Note that `Option<TimerHandle>` takes up the same space as `TimerHandle` and avoids an allocation.
    pub fn dummy() -> TimerHandle {
        TimerHandle(Handle::dummy(TimerState {
            paused: AtomicBool::new(true),
            deadline: Mutex::new(TimerDeadline {
                interval: Duration::MAX,
                last: DInstant::EPOCH,
            }),
            count: AtomicUsize::new(0),
        }))
    }

    /// Drops the handle but does **not** drop the handler closure.
    ///
    /// The handler closure will be dropped when the app exits or if it is stopped from the inside or using another handle.
    pub fn perm(self) {
        self.0.perm();
    }

    /// If [`perm`](Self::perm) was called in another handle.
    ///
    /// If `true` the closure will keep being called until the app exits or the timer is stopped from the inside or using
    /// another handle.
    pub fn is_permanent(&self) -> bool {
        self.0.is_permanent()
    }

    /// Drops the handle and forces the handler to drop.
    ///
    /// The handler will no longer be called and will drop in the next app update.
    pub fn stop(self) {
        self.0.force_drop();
    }

    /// If the timer was stopped. The timer can be stopped from the inside, from another handle calling [`stop`](Self::stop)
    /// or from the app shutting down.
    pub fn is_stopped(&self) -> bool {
        self.0.is_dropped()
    }

    /// The timer interval. Enabled handlers are called every time this interval elapses.
    pub fn interval(&self) -> Duration {
        self.0.data().deadline.lock().interval
    }

    /// Sets the [`interval`](Self::interval).
    ///
    /// Note that this method does not awake the app, so if this is called from outside the app
    /// thread it will only apply on the next app update.
    pub fn set_interval(&self, new_interval: Duration) {
        self.0.data().deadline.lock().interval = new_interval;
    }

    /// Last elapsed time, or the start time if the timer has not elapsed yet.
    pub fn timestamp(&self) -> DInstant {
        self.0.data().deadline.lock().last
    }

    /// The next deadline.
    ///
    /// This is the [`timestamp`](Self::timestamp) plus the [`interval`](Self::interval).
    pub fn deadline(&self) -> Deadline {
        self.0.data().deadline.lock().current_deadline()
    }

    /// If the timer is not ticking, but can be started again.
    pub fn is_paused(&self) -> bool {
        self.0.data().paused.load(Ordering::Relaxed)
    }

    /// Disable the timer, this causes the timer to stop ticking until [`play`] is called.
    ///
    /// [`play`]: Self::play
    pub fn pause(&self) {
        self.0.data().paused.store(true, Ordering::Relaxed);
    }

    /// If the timer is ticking.
    pub fn is_playing(&self) -> bool {
        !self.is_paused() && !self.is_stopped()
    }

    /// Enable the timer, this causes it to start ticking again.
    ///
    /// If `reset` is `true` the last [`timestamp`] is set to now.
    ///
    /// Note that this method does not wake the app, so if this is called from outside the app
    /// the timer will only start ticking in next app update.
    ///
    /// [`timestamp`]: Self::timestamp
    pub fn play(&self, reset: bool) {
        self.0.data().paused.store(false, Ordering::Relaxed);
        if reset {
            self.0.data().deadline.lock().last = INSTANT.now();
        }
    }

    /// Count incremented by one every time the timer elapses.
    pub fn count(&self) -> usize {
        self.0.data().count.load(Ordering::Relaxed)
    }

    /// Resets the [`count`](Self::count).
    pub fn set_count(&self, count: usize) {
        self.0.data().count.store(count, Ordering::Relaxed)
    }

    /// Create a weak handle to the timer.
    pub fn downgrade(&self) -> WeakTimerHandle {
        WeakTimerHandle(self.0.downgrade())
    }
}
impl fmt::Debug for TimerHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TimerHandle")
            .field("interval", &self.interval())
            .field("count", &self.count())
            .field("timestamp", &self.timestamp())
            .field("handle", &self.0)
            .field(
                "state",
                &if self.is_stopped() {
                    "is_stopped"
                } else if self.is_paused() {
                    "is_paused"
                } else {
                    "playing"
                },
            )
            .finish()
    }
}

/// Weak [`TimerHandle`].
#[derive(Clone, PartialEq, Eq, Hash, Default, Debug)]
pub struct WeakTimerHandle(WeakHandle<TimerState>);
impl WeakTimerHandle {
    /// New weak handle that does not upgrade.
    pub fn new() -> Self {
        Self(WeakHandle::new())
    }

    /// Get the strong handle if the timer has not stopped.
    pub fn upgrade(&self) -> Option<TimerHandle> {
        self.0.upgrade().map(TimerHandle)
    }
}

/// An [`interval`](TIMERS::interval) timer.
///
/// This is a variable of type [`Timer`], it will update every time the timer elapses.
///
/// Drop all clones of this variable to stop the timer, you can also control the timer
/// with methods in the [`Timer`] value even though the variable is read-only.
///
/// ```
/// # use zng_app::timer::*;
/// # use zng_app::handler::*;
/// # use zng_app::var::*;
/// # use zng_txt::*;
/// # use zng_layout::unit::*;
/// # use std::time::Instant;
/// # fn foo() {
/// let timer: TimerVar = TIMERS.interval(1.secs(), false);
///
/// # let
/// text = timer.map(|d| match 20 - d.count() {
///     0 => {
///         d.stop();
///         formatx!("Done!")
///     }
///     1 => formatx!("1 second left"),
///     s => formatx!("{s} seconds left"),
/// });
/// # }
/// ```
///
/// In the example above the variable updates every second and stops after 20 seconds have elapsed. The variable
/// is mapped to a text and controls the timer from inside the mapping closure. See [`Var<T>`] for other things you
/// can do with variables, including `.await` for updates. Also see [`Timer`] for more timer control methods.
///
/// [`Var<T>`]: zng_var::Var
pub type TimerVar = Var<Timer>;

/// Represents a timer state in a [`TimerVar`] or interval handler.
///
/// This type uses interior mutability to communicate with the timer, the values provided by the methods
/// can be changed anytime by the [`TimerVar`] owners without the variable updating.
#[derive(Clone, PartialEq)]
pub struct Timer(TimerHandle);
impl fmt::Debug for Timer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Timer")
            .field("interval", &self.interval())
            .field("count", &self.count())
            .field("is_paused", &self.is_paused())
            .field("is_stopped", &self.is_stopped())
            .finish_non_exhaustive()
    }
}
impl Timer {
    /// Permanently stops the timer.
    pub fn stop(&self) {
        self.0.clone().stop();
    }

    /// If the timer was stopped.
    ///
    /// If `true` the timer var will not update again, this is permanent.
    pub fn is_stopped(&self) -> bool {
        self.0.is_stopped()
    }

    /// The timer interval. Enabled variables update every time this interval elapses.
    pub fn interval(&self) -> Duration {
        self.0.interval()
    }

    /// Sets the [`interval`](Self::interval).
    ///
    /// Note that this method does not awake the app, so if this is called from outside the app
    /// thread it will only apply on the next app update.
    pub fn set_interval(&self, new_interval: Duration) {
        self.0.set_interval(new_interval)
    }

    /// Last update time, or the start time if the timer has not updated yet.
    pub fn timestamp(&self) -> DInstant {
        self.0.timestamp()
    }

    /// The next deadline.
    ///
    /// This is the [`timestamp`](Self::timestamp) plus the [`interval`](Self::interval).
    pub fn deadline(&self) -> Deadline {
        self.0.deadline()
    }

    /// If the timer is not ticking, but can be started again.
    pub fn is_paused(&self) -> bool {
        self.0.is_paused()
    }

    /// If the timer is ticking.
    pub fn is_playing(&self) -> bool {
        self.0.is_playing()
    }

    /// Disable the timer, this causes the timer to stop ticking until [`play`] is called.
    ///
    /// [`play`]: Self::play
    pub fn pause(&self) {
        self.0.pause();
    }

    /// Enable the timer, this causes it to start ticking again.
    ///
    /// If `reset` is `true` the last [`timestamp`] is set to now.
    ///
    /// [`timestamp`]: Self::timestamp
    pub fn play(&self, reset: bool) {
        self.0.play(reset);
    }

    /// Count incremented by one every time the timer elapses.
    pub fn count(&self) -> usize {
        self.0.count()
    }

    /// Resets the [`count`](Self::count).
    pub fn set_count(&self, count: usize) {
        self.0.set_count(count)
    }
}

/// Arguments for an [`on_interval`](TIMERS::on_interval) handler.
///
/// Note the timer can be stopped using the handlers [`unsubscribe`](crate::handler::AppWeakHandle::unsubscribe),
/// and *once* handlers stop the timer automatically.
///
/// The field values are about the specific call to handler that received the args, the methods on the other hand
/// are **connected** with the timer by a weak reference and always show the up-to-date state of the timer.
/// For synchronous handlers this does not matter, but for async handlers this means that the values can be
/// different after each `.await`. This can be useful to for example, disable the timer until the async task finishes
/// but it can also be surprising.
#[derive(Clone)]
pub struct TimerArgs {
    /// When the handler was called.
    pub timestamp: DInstant,

    /// Expected deadline, is less-or-equal to the [`timestamp`](Self::timestamp).
    pub deadline: Deadline,

    wk_handle: WeakHandle<TimerState>,
}

impl TimerArgs {
    fn handle(&self) -> Option<TimerHandle> {
        self.wk_handle.upgrade().map(TimerHandle)
    }

    /// The timer interval. Enabled handlers are called every time this interval elapses.
    pub fn interval(&self) -> Duration {
        self.handle().map(|h| h.interval()).unwrap_or_default()
    }

    /// Set the [`interval`](Self::interval).
    ///
    /// Note that this method does not awake the app, so if this is called from outside the app
    /// thread it will only apply on the next app update.
    pub fn set_interval(&self, new_interval: Duration) {
        if let Some(h) = self.handle() {
            h.set_interval(new_interval)
        }
    }

    /// If the timer is not ticking, but can be started again.
    pub fn is_paused(&self) -> bool {
        self.handle().map(|h| h.is_paused()).unwrap_or(true)
    }

    /// If the timer is ticking.
    pub fn is_playing(&self) -> bool {
        self.handle().map(|h| h.is_playing()).unwrap_or(false)
    }

    /// Disable the timer, this causes the timer to stop ticking until [`play`] is called.
    ///
    /// [`play`]: Self::play
    pub fn pause(&self) {
        if let Some(h) = self.handle() {
            h.pause();
        }
    }

    /// Enable the timer, this causes it to start ticking again.
    ///
    /// If `reset` is `true` the last [`timestamp`] is set to now.
    ///
    /// [`timestamp`]: Self::timestamp
    pub fn play(&self, reset: bool) {
        if let Some(h) = self.handle() {
            h.play(reset);
        }
    }

    /// Count incremented by one every time the timer elapses.
    pub fn count(&self) -> usize {
        self.handle().map(|h| h.count()).unwrap_or(0)
    }

    /// Resets the [`count`](Self::count).
    pub fn set_count(&self, count: usize) {
        if let Some(h) = self.handle() {
            h.set_count(count)
        }
    }

    /// The timestamp of the last update. This can be different from [`timestamp`](Self::timestamp)
    /// after the first `.await` in async handlers of if called from a different thread.
    pub fn last_timestamp(&self) -> DInstant {
        self.handle().map(|h| h.timestamp()).unwrap_or(self.timestamp)
    }

    /// The next timer deadline.
    ///
    /// This is [`last_timestamp`](Self::last_timestamp) plus [`interval`](Self::interval).
    pub fn next_deadline(&self) -> Deadline {
        self.handle().map(|h| h.deadline()).unwrap_or(self.deadline)
    }

    /// If the timer was stopped while the handler was running after it started handling.
    ///
    /// Note the timer can be stopped from the inside of the handler using the handlers
    /// [`unsubscribe`], and once handlers stop the timer automatically.
    ///
    /// Outside of the handler the [`TimerHandle`] can be used to stop the timer at any time, even from another thread.
    ///
    /// [`unsubscribe`]: crate::handler::AppWeakHandle::unsubscribe
    pub fn is_stopped(&self) -> bool {
        self.handle().is_none()
    }
}

pub(crate) fn deadline_service(deadline: Deadline) -> Pin<Box<dyn Future<Output = ()> + Send + Sync>> {
    Box::pin(TIMERS.wait_deadline(deadline))
}
