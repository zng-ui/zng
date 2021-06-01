//! Asynchronous tasks and communication.

use crate::{
    event::{AnyEventUpdate, Event},
    var::{response_var, var, ForceReadOnlyVar, RcVar, ResponderVar, ResponseVar, Vars},
};

use super::{
    context::{AppSyncContext, UpdateNotifier},
    var::{Var, VarValue},
};
use flume::{self, Receiver, Sender, TryRecvError};
use retain_mut::*;
use std::{
    fmt,
    future::Future,
    sync::{atomic::AtomicBool, Arc},
    time::{Duration, Instant},
};

/// Asynchronous tasks controller.
pub struct Sync {
    notifier: UpdateNotifier,
    channels: Vec<Box<dyn SyncChannel>>,

    once_timers: Vec<OnceTimer>,
    interval_timers: Vec<IntervalTimer>,

    new_wake_time: Option<Instant>,
}
impl Sync {
    pub(super) fn new(notifier: UpdateNotifier) -> Self {
        Sync {
            notifier,
            channels: vec![],
            once_timers: vec![],
            interval_timers: vec![],
            new_wake_time: None,
        }
    }

    pub(super) fn update(&mut self, ctx: &mut AppSyncContext) -> Option<Instant> {
        self.channels.retain(|t| t.update(ctx));
        self.new_wake_time.take()
    }

    /// Update timers, gets next wakeup moment.
    pub fn update_timers(&mut self, vars: &Vars) -> Option<Instant> {
        let now = Instant::now();

        self.once_timers.retain(|t| t.retain(now, vars));
        self.interval_timers.retain_mut(|t| t.retain(now, vars));

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

    /// Create a variable update listener that can be used from other threads.
    ///
    /// The variable current value is send during this call.
    ///
    /// Context variables are evaluated in the app root context.
    pub fn var_receiver<T: VarValue + Send, V: Var<T>>(&mut self, var: V) -> VarReceiver<T> {
        let (sync, sender) = VarReceiverSync::new(var);
        self.channels.push(Box::new(sync));
        sender
    }

    /// Create a variable setter that can be used from other threads.
    ///
    /// Context variables are set in the app root context.
    pub fn var_sender<T: VarValue + Send, V: Var<T>>(&mut self, var: V) -> VarSender<T> {
        let (sync, sender) = VarSenderSync::new(var, self.notifier.clone());
        self.channels.push(Box::new(sync));
        sender
    }

    /// Create a variable setter that can be used from other threads.
    ///
    /// Instead of sending a new full value this sends a `impl FnOnce(&mut T) + Send + 'static`
    /// that is evaluated in the app thread.
    ///
    /// Context variables are modified in the app root context.
    pub fn var_modify_sender<T: VarValue, V: Var<T>>(&mut self, var: V) -> VarModifySender<T> {
        let (sync, sender) = VarModifySenderSync::new(var, self.notifier.clone());
        self.channels.push(Box::new(sync));
        sender
    }

    /// Variable sender/receiver dual-channel.
    pub fn var_channel<T: VarValue + Send, V: Var<T>>(&mut self, var: V) -> VarChannel<T> {
        let (sync, channel) = VarChannelSync::new(var, self.notifier.clone());
        self.channels.push(Box::new(sync));
        channel
    }

    /// Creates a channel that can raise an event from another thread.
    pub fn event_sender<A, E>(&mut self) -> EventSender<E>
    where
        E: Event,
        E::Args: Send,
    {
        let (sync, sender) = EventSenderSync::new(self.notifier.clone());
        self.channels.push(Box::new(sync));
        sender
    }

    /// Creates a channel that can listen to event from another thread.
    pub fn event_receiver<E>(&mut self) -> EventReceiver<E>
    where
        E: Event,
        E::Args: Send,
    {
        let (sync, receiver) = EventReceiverSync::new();
        self.channels.push(Box::new(sync));
        receiver
    }

    /// Run a CPU bound task.
    ///
    /// The task runs in a [`rayon`] thread-pool, this function is not blocking.
    ///
    /// # Example
    ///
    /// ```
    /// # use zero_ui_core::{context::WidgetContext, var::ResponseVar};
    /// # struct SomeStruct { sum_response: ResponseVar<usize> }
    /// # impl SomeStruct {
    /// fn on_event(&mut self, ctx: &mut WidgetContext) {
    ///     self.sum_response = ctx.sync.run(||{
    ///         (0..1000).sum()
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
    pub fn run<R: VarValue + Send, T: FnOnce() -> R + Send + 'static>(&mut self, task: T) -> ResponseVar<R> {
        let (responder, response) = response_var();
        let sender = self.var_sender(responder);
        rayon::spawn(move || {
            let r = task();
            sender.set(crate::var::Response::Done(r));
        });
        response
    }

    /// Run an IO bound task.
    ///
    /// The task runs in an [`async-global-executor`] thread-pool, this function is not blocking.
    ///
    /// # Example
    ///
    /// ```
    /// # use zero_ui_core::{context::WidgetContext, var::ResponseVar};
    /// # struct SomeStruct { file_response: ResponseVar<Vec<u8>> }
    /// # impl SomeStruct {
    /// fn on_event(&mut self, ctx: &mut WidgetContext) {
    ///     self.file_response = ctx.sync.run_async(async {
    ///         todo!("use async_std to read a file")     
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
    pub fn run_async<R: VarValue + Send, T: Future<Output = R> + Send + 'static>(&mut self, task: T) -> ResponseVar<R> {
        let (responder, response) = response_var();
        let sender = self.var_sender(responder);
        // TODO run block-on?
        async_global_executor::spawn(async move {
            let r = task.await;
            sender.set(crate::var::Response::Done(r));
        })
        .detach();
        response
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

type Retain = bool;

trait SyncChannel {
    /// Sync events.
    ///
    /// Returns if this object should be retained.
    fn on_event(&self, ctx: &mut AppSyncContext, args: &AnyEventUpdate) -> Retain;

    /// Sync updates.
    ///
    /// Returns if this object should be retained.
    fn update(&self, ctx: &mut AppSyncContext) -> Retain;
}

/// Represents an [`EventEmitter`] that can be updated from other threads.
///
/// See [`Sync::event_sender`] for more details.
pub struct EventSender<E>
where
    E: Event,
    E::Args: Send,
{
    notifier: UpdateNotifier,
    sender: Sender<E::Args>,
}
impl<E> Clone for EventSender<E>
where
    E: Event,
    E::Args: Send,
{
    fn clone(&self) -> Self {
        EventSender {
            notifier: self.notifier.clone(),
            sender: self.sender.clone(),
        }
    }
}
impl<E> EventSender<E>
where
    E: Event,
    E::Args: Send,
{
    /// Pushes an update notification.
    ///
    /// This will generate an event update in the UI thread.
    pub fn notify(&self, args: E::Args) {
        self.sender.send(args).expect("TODO can this fail?");
        self.notifier.update();
    }
}
struct EventSenderSync<E>
where
    E: Event,
    E::Args: Send,
{
    receiver: Receiver<E::Args>,
}
impl<E> EventSenderSync<E>
where
    E: Event,
    E::Args: Send,
{
    fn new(notifier: UpdateNotifier) -> (Self, EventSender<E>) {
        let (sender, receiver) = flume::unbounded();
        (EventSenderSync { receiver }, EventSender { notifier, sender })
    }
}
impl<E> SyncChannel for EventSenderSync<E>
where
    E: Event,
    E::Args: Send,
{
    fn on_event(&self, _: &mut AppSyncContext, _: &AnyEventUpdate) -> Retain {
        true
    }

    fn update(&self, ctx: &mut AppSyncContext) -> Retain {
        for args in self.receiver.try_iter() {
            E::notify(ctx.events, args);
        }
        !self.receiver.is_disconnected()
    }
}

/// Represents an [`EventListener`] that can receive updates from other threads.
///
/// See [`Sync::event_receiver`] for more details.
#[derive(Clone)]
pub struct EventReceiver<E>
where
    E: Event,
    E::Args: Send,
{
    receiver: Receiver<E::Args>,
}
impl<E> EventReceiver<E>
where
    E: Event,
    E::Args: Send,
{
    /// A blocking iterator over the updated received.
    pub fn updates(&self) -> flume::Iter<E::Args> {
        self.receiver.iter()
    }

    /// A non-blocking iterator over the updates received.
    #[inline]
    pub fn try_updates(&self) -> flume::TryIter<E::Args> {
        self.receiver.try_iter()
    }

    /// Reference the underlying update receiver.
    pub fn receiver(&self) -> &Receiver<E::Args> {
        &self.receiver
    }
}
struct EventReceiverSync<E>
where
    E: Event,
    E::Args: Send,
{
    sender: Sender<E::Args>,
}
impl<E> EventReceiverSync<E>
where
    E: Event,
    E::Args: Send,
{
    fn new() -> (Self, EventReceiver<E>) {
        let (sender, receiver) = flume::unbounded();
        (EventReceiverSync { sender }, EventReceiver { receiver })
    }
}
impl<E> SyncChannel for EventReceiverSync<E>
where
    E: Event,
    E::Args: Send,
{
    fn on_event(&self, _: &mut AppSyncContext, args: &AnyEventUpdate) -> Retain {
        if let Some(args) = E::update(args) {
            self.sender.send(args.clone()).expect("TODO");
        }
        !self.sender.is_disconnected()
    }

    fn update(&self, _: &mut AppSyncContext) -> Retain {
        true
    }
}

/// See [`Sync::var_sender`] for more details.
pub struct VarSender<T: VarValue + Send> {
    notifier: UpdateNotifier,
    sender: Sender<T>,
}
impl<T: VarValue + Send> Clone for VarSender<T> {
    fn clone(&self) -> Self {
        VarSender {
            notifier: self.notifier.clone(),
            sender: self.sender.clone(),
        }
    }
}
impl<T: VarValue + Send> VarSender<T> {
    /// Send the variable a new value.
    #[inline]
    pub fn set(&self, new_value: T) {
        self.sender.send(new_value).expect("TODO");
        self.notifier.update();
    }
}
struct VarSenderSync<T: VarValue + Send, V: Var<T>> {
    var: V,
    receiver: Receiver<T>,
}
impl<T: VarValue + Send, V: Var<T>> VarSenderSync<T, V> {
    fn new(var: V, notifier: UpdateNotifier) -> (Self, VarSender<T>) {
        let (sender, receiver) = flume::unbounded();
        (VarSenderSync { var, receiver }, VarSender { notifier, sender })
    }
}
impl<T: VarValue + Send, V: Var<T>> SyncChannel for VarSenderSync<T, V> {
    fn update(&self, ctx: &mut AppSyncContext) -> Retain {
        if let Some(new_value) = self.receiver.try_iter().last() {
            let _ = self.var.set(ctx.vars, new_value);
        }
        !self.receiver.is_disconnected()
    }

    fn on_event(&self, _: &mut AppSyncContext, _: &AnyEventUpdate) -> Retain {
        true
    }
}

/// See [`Sync::var_modify_sender`] for more details.
pub struct VarModifySender<T: VarValue> {
    notifier: UpdateNotifier,
    sender: Sender<Box<dyn FnOnce(&mut T) + Send>>,
}
impl<T: VarValue + Send> Clone for VarModifySender<T> {
    fn clone(&self) -> Self {
        VarModifySender {
            notifier: self.notifier.clone(),
            sender: self.sender.clone(),
        }
    }
}
impl<T: VarValue> VarModifySender<T> {
    /// Send the variable an update.
    pub fn modify<U>(&self, update: U)
    where
        U: FnOnce(&mut T) + Send + 'static,
    {
        self.sender.send(Box::new(update)).expect("TODO");
        self.notifier.update();
    }
}
struct VarModifySenderSync<T: VarValue, V: Var<T>> {
    var: V,
    receiver: Receiver<Box<dyn FnOnce(&mut T) + Send>>,
}
impl<T: VarValue, V: Var<T>> VarModifySenderSync<T, V> {
    fn new(var: V, notifier: UpdateNotifier) -> (Self, VarModifySender<T>) {
        let (sender, receiver) = flume::unbounded();
        (VarModifySenderSync { var, receiver }, VarModifySender { notifier, sender })
    }
}
impl<T: VarValue, V: Var<T>> SyncChannel for VarModifySenderSync<T, V> {
    fn update(&self, ctx: &mut AppSyncContext) -> Retain {
        for change in self.receiver.try_iter() {
            let _ = self.var.modify_boxed(ctx.vars, change);
        }
        !self.receiver.is_disconnected()
    }

    fn on_event(&self, _: &mut AppSyncContext, _: &AnyEventUpdate) -> Retain {
        true
    }
}

/// See [`Sync::var_receiver`] for more details.
pub struct VarReceiver<T: VarValue + Send> {
    receiver: Receiver<T>,
}
impl<T: VarValue + Send> Clone for VarReceiver<T> {
    fn clone(&self) -> Self {
        VarReceiver {
            receiver: self.receiver.clone(),
        }
    }
}
impl<T: VarValue + Send> VarReceiver<T> {
    /// Wait for a value update.
    #[inline]
    pub fn get(&self) -> Result<T, flume::RecvError> {
        self.receiver.recv()
    }

    /// Try to fetch a value update.
    #[inline]
    pub fn try_get(&self) -> Result<T, TryRecvError> {
        self.receiver.try_recv()
    }

    /// Wait for a value update until the timeout duration.
    #[inline]
    pub fn get_timeout(&self, dur: Duration) -> Result<T, flume::RecvTimeoutError> {
        self.receiver.recv_timeout(dur)
    }

    /// Reference the underlying update receiver.
    #[inline]
    pub fn receiver(&self) -> &Receiver<T> {
        &self.receiver
    }
}
struct VarReceiverSync<T: VarValue + Send, V: Var<T>> {
    var: V,
    sender: Sender<T>,
}
impl<T: VarValue + Send, V: Var<T>> VarReceiverSync<T, V> {
    fn new(var: V) -> (Self, VarReceiver<T>) {
        let (sender, receiver) = flume::unbounded();
        (VarReceiverSync { var, sender }, VarReceiver { receiver })
    }
}
impl<T: VarValue + Send, V: Var<T>> SyncChannel for VarReceiverSync<T, V> {
    fn update(&self, ctx: &mut AppSyncContext) -> Retain {
        if let Some(update) = self.var.get_new(ctx.vars) {
            let _ = self.sender.send(update.clone());
        }
        !self.sender.is_disconnected()
    }

    fn on_event(&self, _: &mut AppSyncContext, _: &AnyEventUpdate) -> Retain {
        true
    }
}

/// Represents a [`Var`](crate::var::Var) that can be read and updated from other threads.
///
/// See [`Sync::var_channel`] for more details.
///
/// ### Initial Value
///
/// The first value in the channel is the variable value when the channel was created, so this
/// method returns immediately on the first call.

pub struct VarChannel<T: VarValue + Send> {
    notifier: UpdateNotifier,
    sender: Sender<T>,
    receiver: Receiver<T>,
}
impl<T: VarValue + Send> Clone for VarChannel<T> {
    fn clone(&self) -> Self {
        VarChannel {
            notifier: self.notifier.clone(),
            sender: self.sender.clone(),
            receiver: self.receiver.clone(),
        }
    }
}
impl<T: VarValue + Send> VarChannel<T> {
    /// Send the variable a new value.
    #[inline]
    pub fn set(&self, new_value: T) {
        self.sender.send(new_value).expect("TODO")
    }

    /// Reference the underlying update receiver.
    #[inline]
    pub fn receiver(&self) -> &Receiver<T> {
        &self.receiver
    }

    /// Wait for a value update.
    #[inline]
    pub fn get(&self) -> Result<T, flume::RecvError> {
        self.receiver.recv()
    }

    /// Try to fetch a value update.
    #[inline]
    pub fn try_get(&self) -> Result<T, TryRecvError> {
        self.receiver.try_recv()
    }

    /// Wait for a value update until the timeout duration.
    #[inline]
    pub fn get_timeout(&self, dur: Duration) -> Result<T, flume::RecvTimeoutError> {
        self.receiver.recv_timeout(dur)
    }
}
struct VarChannelSync<T: VarValue + Send, V: Var<T>> {
    var: V,
    out_sender: Sender<T>,
    in_receiver: Receiver<T>,
}
impl<T: VarValue + Send, V: Var<T>> VarChannelSync<T, V> {
    fn new(var: V, notifier: UpdateNotifier) -> (Self, VarChannel<T>) {
        let (out_sender, out_receiver) = flume::unbounded();
        let (in_sender, in_receiver) = flume::unbounded();
        (
            VarChannelSync {
                var,
                out_sender,
                in_receiver,
            },
            VarChannel {
                notifier,
                sender: in_sender,
                receiver: out_receiver,
            },
        )
    }
}
impl<T: VarValue + Send, V: Var<T>> SyncChannel for VarChannelSync<T, V> {
    fn update(&self, ctx: &mut AppSyncContext) -> Retain {
        if let Some(new_value) = self.var.get_new(ctx.vars) {
            let _ = self.out_sender.send(new_value.clone());
        }
        if let Some(new_value) = self.in_receiver.try_iter().last() {
            let _ = self.var.set(ctx.vars, new_value);
        }
        !self.out_sender.is_disconnected()
    }

    fn on_event(&self, _: &mut AppSyncContext, _: &AnyEventUpdate) -> Retain {
        true
    }
}
