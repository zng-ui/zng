//! Asynchronous tasks.

use super::{
    context::{UpdateNotifier, Updates},
    event::{EventEmitter, EventListener},
};
use std::{
    future::Future,
    sync::mpsc::{channel, Receiver, Sender, TryRecvError},
};

/// Asynchronous tasks controller.
pub struct Tasks {
    notifier: UpdateNotifier,
    events: Vec<Box<dyn EventChannelAny>>,
}
impl Tasks {
    pub(super) fn new(notifier: UpdateNotifier) -> Self {
        Tasks { notifier, events: vec![] }
    }

    pub(super) fn update(&mut self, updates: &mut Updates) {
        self.events.retain(|t| t.try_recv(updates));
    }

    /// Creates an event emitter that can be used from other threads.
    pub fn event<T: Send + 'static>(&mut self, event: EventEmitter<T>) -> AsyncEventEmitter<T> {
        let (task, emitter) = EventChannel::new(event, self.notifier.clone());
        self.events.push(Box::new(task));
        emitter
    }

    fn response<T: Send + 'static>(&mut self) -> (AsyncEventEmitter<T>, EventListener<T>) {
        let event = EventEmitter::response();
        let listener = event.listener();
        let event = self.event(event);
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
            event.push_update(r);
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
            event.push_update(r);
        })
        .detach();
        listener
    }
}

type Retain = bool;

trait EventChannelAny {
    /// Receive and emit an event message.
    ///
    /// Returns if the channel should be retained.
    fn try_recv(&self, updates: &mut Updates) -> Retain;
}
struct EventChannel<T: Send + 'static> {
    event: EventEmitter<T>,
    receiver: Receiver<T>,
}
impl<T: Send + 'static> EventChannel<T> {
    fn new(event: EventEmitter<T>, notifier: UpdateNotifier) -> (Self, AsyncEventEmitter<T>) {
        let (sender, receiver) = channel();
        (EventChannel { event, receiver }, AsyncEventEmitter { notifier, sender })
    }
}
impl<T: Send + 'static> EventChannelAny for EventChannel<T> {
    fn try_recv(&self, updates: &mut Updates) -> Retain {
        for args in self.receiver.try_iter() {
            updates.push_notify(self.event.clone(), args);
        }
        match self.receiver.try_recv() {
            Err(TryRecvError::Empty) => true,
            Err(TryRecvError::Disconnected) => false,
            Ok(args) => {
                updates.push_notify(self.event.clone(), args);
                true
            }
        }
    }
}

/// Represents an [`EventEmitter`] that can be updated from other threads.
///
/// See [`Tasks::event`] for more details.
#[derive(Clone)]
pub struct AsyncEventEmitter<T: Send + 'static> {
    notifier: UpdateNotifier,
    sender: Sender<T>,
}
impl<T: Send + 'static> AsyncEventEmitter<T> {
    /// Pushes an update notification.
    ///
    /// This will generate an event update
    pub fn push_update(&self, args: T) {
        self.sender.send(args).expect("TODO can this fail?");
        self.notifier.push_update(); // TODO high-pressure?
    }
}
