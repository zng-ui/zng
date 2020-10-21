//! Asynchronous tasks and communication.

use super::{
    context::{AppSyncContext, UpdateNotifier},
    event::{EventEmitter, EventListener},
    var::{Var, VarValue},
};
use flume::{self, Receiver, Sender, TryRecvError};
use std::{future::Future, time::Duration};

/// Asynchronous tasks controller.
pub struct Tasks {
    notifier: UpdateNotifier,
    channels: Vec<Box<dyn SyncChannel>>,
}
impl Tasks {
    pub(super) fn new(notifier: UpdateNotifier) -> Self {
        Tasks {
            notifier,
            channels: vec![],
        }
    }

    pub(super) fn update(&mut self, ctx: &mut AppSyncContext) {
        self.channels.retain(|t| t.update(ctx));
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

    /// Creates an event emitter that can be used from other threads.
    pub fn event_sender<T: Send + 'static>(&mut self, event: EventEmitter<T>) -> EventSender<T> {
        let (sync, sender) = EventSenderSync::new(event, self.notifier.clone());
        self.channels.push(Box::new(sync));
        sender
    }

    fn response<T: Send + 'static>(&mut self) -> (EventSender<T>, EventListener<T>) {
        let event = EventEmitter::response();
        let listener = event.listener();
        let event = self.event_sender(event);
        (event, listener)
    }

    /// Run a CPU bound task.
    ///
    /// The task runs in a [`rayon`] thread-pool, this function is not blocking.
    ///
    /// # Example
    ///
    /// ```
    /// # use zero_ui::core::{context::WidgetContext, event::EventListener};
    /// # struct SomeStruct { sum_listener: EventListener<usize> }
    /// # impl SomeStruct {
    /// fn on_event(&mut self, ctx: &mut WidgetContext) {
    ///     self.sum_listener = ctx.tasks.run(||{
    ///         (0..1000).sum()
    ///     });
    /// }
    ///
    /// fn on_update(&mut self, ctx: &mut WidgetContext) {
    ///     if let Some(result) = self.sum_listener.updates(ctx.events).last() {
    ///         println!("sum of 0..1000: {}", result);   
    ///     }
    /// }
    /// # }
    /// ```
    pub fn run<R: Send + 'static, T: FnOnce() -> R + Send + 'static>(&mut self, task: T) -> EventListener<R> {
        let (event, listener) = self.response();
        rayon::spawn(move || {
            let r = task();
            event.notify(r);
        });
        listener
    }

    /// Run an IO bound task.
    ///
    /// The task runs in an [`async-global-executor`] thread-pool, this function is not blocking.
    ///
    /// # Example
    ///
    /// ```
    /// # use zero_ui::core::{context::WidgetContext, event::EventListener};
    /// # struct SomeStruct { file_listener: EventListener<Vec<u8>> }
    /// # impl SomeStruct {
    /// fn on_event(&mut self, ctx: &mut WidgetContext) {
    ///     self.file_listener = ctx.tasks.run_async(async {
    ///         todo!("use async_std to read a file")     
    ///     });
    /// }
    ///
    /// fn on_update(&mut self, ctx: &mut WidgetContext) {
    ///     if let Some(result) = self.file_listener.updates(ctx.events).last() {
    ///         println!("file loaded: {} bytes", result.len());   
    ///     }
    /// }
    /// # }
    /// ```
    pub fn run_async<R: Send + 'static, T: Future<Output = R> + Send + 'static>(&mut self, task: T) -> EventListener<R> {
        let (event, listener) = self.response();
        // TODO run block-on?
        async_global_executor::spawn(async move {
            let r = task.await;
            event.notify(r);
        })
        .detach();
        listener
    }
}

type Retain = bool;

trait SyncChannel {
    /// Sync updates.
    ///
    /// Returns if this object should be retained.
    fn update(&self, ctx: &mut AppSyncContext) -> Retain;
}

/// Represents an [`EventEmitter`] that can be updated from other threads.
///
/// See [`Tasks::event`] for more details.
#[derive(Clone)]
pub struct EventSender<T: Send + 'static> {
    notifier: UpdateNotifier,
    sender: Sender<T>,
}
impl<T: Send + 'static> EventSender<T> {
    /// Pushes an update notification.
    ///
    /// This will generate an event update.
    pub fn notify(&self, args: T) {
        self.sender.send(args).expect("TODO can this fail?");
        self.notifier.push_update(); // TODO high-pressure?
    }
}
struct EventSenderSync<T: Send + 'static> {
    event: EventEmitter<T>,
    receiver: Receiver<T>,
}
impl<T: Send + 'static> EventSenderSync<T> {
    fn new(event: EventEmitter<T>, notifier: UpdateNotifier) -> (Self, EventSender<T>) {
        let (sender, receiver) = flume::unbounded();
        (EventSenderSync { event, receiver }, EventSender { notifier, sender })
    }
}
impl<T: Send + 'static> SyncChannel for EventSenderSync<T> {
    fn update(&self, ctx: &mut AppSyncContext) -> Retain {
        for args in self.receiver.try_iter() {
            ctx.updates.push_notify(self.event.clone(), args);
        }
        !self.receiver.is_disconnected()
    }
}

#[derive(Clone)]
pub struct VarSender<T: VarValue + Send> {
    notifier: UpdateNotifier,
    sender: Sender<T>,
}
impl<T: VarValue + Send> VarSender<T> {
    /// Send the variable a new value.
    #[inline]
    pub fn set(&self, new_value: T) {
        self.sender.send(new_value).expect("TODO");
        self.notifier.push_update();
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
            let _ = self.var.push_set(new_value, ctx.vars, ctx.updates);
        }
        !self.receiver.is_disconnected()
    }
}

#[derive(Clone)]
pub struct VarModifySender<T: VarValue> {
    notifier: UpdateNotifier,
    sender: Sender<Box<dyn FnOnce(&mut T) + Send>>,
}
impl<T: VarValue> VarModifySender<T> {
    /// Send the variable an update.
    pub fn modify<U>(&self, update: U)
    where
        U: FnOnce(&mut T) + Send + 'static,
    {
        self.sender.send(Box::new(update)).expect("TODO");
        self.notifier.push_update();
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
            let _ = self.var.push_modify_boxed(change, ctx.vars, ctx.updates);
        }
        !self.receiver.is_disconnected()
    }
}

///
#[derive(Clone)]
pub struct VarReceiver<T: VarValue + Send> {
    receiver: Receiver<T>,
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
        if let Some(update) = self.var.update(ctx.vars) {
            let _ = self.sender.send(update.clone());
        }
        !self.sender.is_disconnected()
    }
}

/// Represents a [`Var`](crate::core::var::Var) that can be read and updated from other threads.
///
/// See [`Tasks::var_channel`] for more details.
///
/// ### Initial Value
///
/// The first value in the channel is the variable value when the channel was created, so this
/// method returns immediately on the first call.
#[derive(Clone)]
pub struct VarChannel<T: VarValue + Send> {
    notifier: UpdateNotifier,
    sender: Sender<T>,
    receiver: Receiver<T>,
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
        if let Some(new_value) = self.var.update(ctx.vars) {
            let _ = self.out_sender.send(new_value.clone());
        }
        if let Some(new_value) = self.in_receiver.try_iter().last() {
            let _ = self.var.push_set(new_value, ctx.vars, ctx.updates);
        }
        !self.out_sender.is_disconnected()
    }
}
