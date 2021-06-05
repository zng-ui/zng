use super::*;
use crate::{
    app::{AppEvent, AppShutdown, EventLoopProxy, RecvFut, TimeoutOrAppShutdown},
    context::Updates,
};
use std::{
    any::type_name,
    fmt,
    ops::Deref,
    time::{Duration, Instant},
};

thread_singleton!(SingletonVars);

type SyncEntry = Box<dyn Fn(&Vars) -> Retain>;
type Retain = bool;

/// Read-only access to [`Vars`].
///
/// In some contexts variables can be set, so a full [`Vars`] reference if given, in other contexts
/// variables can only be read, so a [`VarsRead`] reference is given.
///
/// [`Vars`] auto-dereferences to to this type.
///
/// # Examples
///
/// You can [`get`](VarObj::get) a value using a [`VarsRead`] reference.
///
/// ```
/// # use zero_ui_core::var::{VarObj, VarsRead};
/// fn read_only(var: &impl VarObj<bool>, vars: &VarsRead) -> bool {
///     *var.get(vars)
/// }
/// ```
///
/// And because of auto-dereference you can can the same method using a full [`Vars`] reference.
///
/// ```
/// # use zero_ui_core::var::{VarObj, Vars};
/// fn read_write(var: &impl VarObj<bool>, vars: &Vars) -> bool {
///     *var.get(vars)
/// }
/// ```
pub struct VarsRead {
    _singleton: SingletonVars,
    update_id: u32,
    #[allow(clippy::type_complexity)]
    widget_clear: RefCell<Vec<Box<dyn Fn(bool)>>>,

    event_loop: EventLoopProxy,
    senders: RefCell<Vec<SyncEntry>>,
    receivers: RefCell<Vec<SyncEntry>>,
}
impl fmt::Debug for VarsRead {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "VarsRead {{ .. }}")
    }
}
impl VarsRead {
    pub(super) fn update_id(&self) -> u32 {
        self.update_id
    }

    /// Gets a var at the context level.
    pub(super) fn context_var<C: ContextVar>(&self) -> (&C::Type, bool, u32) {
        let (value, is_new, version) = C::thread_local_value().get();

        (
            // SAFETY: this is safe as long we are the only one to call `C::thread_local_value().get()` in
            // `Self::with_context_var`.
            //
            // The reference is held for as long as it is accessible in here, at least:
            //
            // * The initial reference is actually the `static` default value.
            // * Other references are held by `Self::with_context_var` for the duration
            //   they can appear here.
            unsafe { &*value },
            is_new,
            version,
        )
    }

    /// Calls `f` with the context var value.
    ///
    /// The value is visible for the duration of `f`, unless `f` recursive overwrites it again.
    #[inline(always)]
    pub fn with_context_var<C, R, F>(&self, context_var: C, value: &C::Type, version: u32, f: F) -> R
    where
        C: ContextVar,
        F: FnOnce() -> R,
    {
        self.with_context_var_impl(context_var, value, false, version, f)
    }
    #[inline(always)]
    fn with_context_var_impl<C, R, F>(&self, context_var: C, value: &C::Type, is_new: bool, version: u32, f: F) -> R
    where
        C: ContextVar,
        F: FnOnce() -> R,
    {
        // SAFETY: `Self::context_var` makes safety assumptions about this code
        // don't change before studying it.

        let _ = context_var;
        let prev = C::thread_local_value().replace((value as _, is_new, version));
        let _restore = RunOnDrop::new(move || {
            C::thread_local_value().set(prev);
        });

        f()

        // _prev restores the parent reference here on drop
    }

    /// Calls `f` with the context var value.
    ///
    /// The value is visible for the duration of `f` and only for the parts of it that are inside the current widget context.
    ///
    /// The value can be overwritten by a recursive call to [`with_context_var`](Vars::with_context_var) or
    /// this method, subsequent values from this same widget context are not visible in inner widget contexts.
    #[inline(always)]
    pub fn with_context_var_wgt_only<C, R, F>(&self, context_var: C, value: &C::Type, version: u32, f: F) -> R
    where
        C: ContextVar,
        F: FnOnce() -> R,
    {
        self.with_context_var_wgt_only_impl(context_var, value, false, version, f)
    }
    #[inline(always)]
    fn with_context_var_wgt_only_impl<C, R, F>(&self, context_var: C, value: &C::Type, is_new: bool, version: u32, f: F) -> R
    where
        C: ContextVar,
        F: FnOnce() -> R,
    {
        // SAFETY: `Self::context_var` makes safety assumptions about this code
        // don't change before studying it.

        let _ = context_var;

        let new = (value as _, is_new, version);
        let prev = C::thread_local_value().replace(new);

        self.widget_clear.borrow_mut().push(Box::new(move |undo| {
            if undo {
                C::thread_local_value().set(prev);
            } else {
                C::thread_local_value().set(new);
            }
        }));

        let _restore = RunOnDrop::new(move || {
            C::thread_local_value().set(prev);
        });

        f()
    }

    /// Calls [`with_context_var`](Vars::with_context_var) with values from `other_var`.
    #[inline(always)]
    pub fn with_context_bind<C, R, F, V>(&self, context_var: C, other_var: &V, f: F) -> R
    where
        C: ContextVar,
        F: FnOnce() -> R,
        V: VarObj<C::Type>,
    {
        self.with_context_var_impl(context_var, other_var.get(self), false, other_var.version(self), f)
    }

    /// Calls [`with_context_var_wgt_only`](Vars::with_context_var_wgt_only) with values from `other_var`.
    #[inline(always)]
    pub fn with_context_bind_wgt_only<C: ContextVar, R, F: FnOnce() -> R, V: VarObj<C::Type>>(
        &self,
        context_var: C,
        other_var: &V,
        f: F,
    ) -> R {
        self.with_context_var_wgt_only_impl(context_var, other_var.get(self), false, other_var.version(self), f)
    }

    /// Clears widget only context var values, calls `f` and restores widget only context var values.
    #[inline(always)]
    pub(crate) fn with_widget_clear<R, F: FnOnce() -> R>(&self, f: F) -> R {
        let wgt_clear = std::mem::take(&mut *self.widget_clear.borrow_mut());
        for clear in &wgt_clear {
            clear(true);
        }

        let _restore = RunOnDrop::new(move || {
            for clear in &wgt_clear {
                clear(false);
            }
            *self.widget_clear.borrow_mut() = wgt_clear;
        });

        f()
    }

    /// Creates a channel that can receive `var` updates from another thread.
    ///
    /// Every time the variable updates a clone of the value is sent to the receiver. The current value is sent immediately.
    ///
    /// Drop the receiver to release one reference to `var`.
    pub fn receiver<T, V>(&self, var: &V) -> VarReceiver<T>
    where
        T: VarValue + Send,
        V: Var<T>,
    {
        let (sender, receiver) = flume::unbounded();
        let _ = sender.send(var.get(self).clone());

        if var.always_read_only() {
            self.senders.borrow_mut().push(Box::new(move |_| {
                // retain if not disconnected.
                !sender.is_disconnected()
            }));
        } else {
            let var = var.clone();
            self.senders.borrow_mut().push(Box::new(move |vars| {
                if let Some(new) = var.get_new(vars) {
                    sender.send(new.clone()).is_ok()
                } else {
                    !sender.is_disconnected()
                }
            }));
        }

        VarReceiver { receiver }
    }
}

/// Access to application variables.
///
/// Only a single instance of this type exists at a time.
pub struct Vars {
    read: VarsRead,
    #[allow(clippy::type_complexity)]
    pending: RefCell<Vec<Box<dyn FnOnce(u32)>>>,
}
impl fmt::Debug for Vars {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Vars {{ .. }}")
    }
}
impl Vars {
    /// If an instance of `Vars` already exists in the  current thread.
    #[inline]
    pub fn instantiated() -> bool {
        SingletonVars::in_use()
    }

    /// Produces the instance of `Vars`. Only a single
    /// instance can exist in a thread at a time, panics if called
    /// again before dropping the previous instance.
    #[inline]
    pub fn instance(event_loop: EventLoopProxy) -> Self {
        Vars {
            read: VarsRead {
                _singleton: SingletonVars::assert_new("Vars"),
                update_id: 0,
                event_loop,
                widget_clear: Default::default(),
                senders: RefCell::default(),
                receivers: RefCell::default(),
            },
            pending: Default::default(),
        }
    }

    /// Calls `f` with the context var value.
    ///
    /// The value is visible for the duration of `f`, unless `f` recursive overwrites it again.
    #[inline(always)]
    pub fn with_context_var<C: ContextVar, F: FnOnce()>(&self, context_var: C, value: &C::Type, is_new: bool, version: u32, f: F) {
        self.with_context_var_impl(context_var, value, is_new, version, f)
    }

    /// Calls `f` with the context var value.
    ///
    /// The value is visible for the duration of `f` and only for the parts of it that are inside the current widget context.
    ///
    /// The value can be overwritten by a recursive call to [`with_context_var`](Vars::with_context_var) or
    /// this method, subsequent values from this same widget context are not visible in inner widget contexts.
    #[inline(always)]
    pub fn with_context_var_wgt_only<C: ContextVar, F: FnOnce()>(&self, context_var: C, value: &C::Type, is_new: bool, version: u32, f: F) {
        self.with_context_var_wgt_only_impl(context_var, value, is_new, version, f)
    }

    /// Calls [`with_context_var`](Vars::with_context_var) with values from `other_var`.
    #[inline(always)]
    pub fn with_context_bind<C: ContextVar, F: FnOnce(), V: VarObj<C::Type>>(&self, context_var: C, other_var: &V, f: F) {
        self.with_context_var_impl(context_var, other_var.get(self), other_var.is_new(self), other_var.version(self), f)
    }

    /// Calls [`with_context_var_wgt_only`](Vars::with_context_var_wgt_only) with values from `other_var`.
    #[inline(always)]
    pub fn with_context_bind_wgt_only<C: ContextVar, F: FnOnce(), V: VarObj<C::Type>>(&self, context_var: C, other_var: &V, f: F) {
        self.with_context_var_wgt_only(context_var, other_var.get(self), other_var.is_new(self), other_var.version(self), f)
    }

    pub(super) fn push_change(&self, change: Box<dyn FnOnce(u32)>) {
        self.pending.borrow_mut().push(change);
    }

    pub(crate) fn apply(&mut self, updates: &mut Updates) {
        self.read.update_id = self.update_id.wrapping_add(1);

        let pending = self.pending.get_mut();
        if !pending.is_empty() {
            for f in pending.drain(..) {
                f(self.read.update_id);
            }
            self.senders.borrow_mut().retain(|f| f(self));
            updates.update();
        }
    }

    pub(crate) fn sync(&self) {
        self.receivers.borrow_mut().retain(|f| f(self));
    }

    /// Creates a sender that can set `var` from other threads and without access to [`Vars`].
    ///
    /// If the variable is read-only when a value is received it is silently dropped.
    ///
    /// Drop the sender to release one reference to `var`.
    pub fn sender<T, V>(&self, var: &V) -> VarSender<T>
    where
        T: VarValue + Send,
        V: Var<T>,
    {
        let (sender, receiver) = flume::unbounded();

        if var.always_read_only() {
            self.receivers.borrow_mut().push(Box::new(move |_| {
                receiver.drain();
                !receiver.is_disconnected()
            }));
        } else {
            let var = var.clone();
            self.receivers.borrow_mut().push(Box::new(move |vars| {
                if let Some(new_value) = receiver.try_iter().last() {
                    let _ = var.set(vars, new_value);
                }
                !receiver.is_disconnected()
            }));
        };

        VarSender {
            wake: self.event_loop.clone(),
            sender,
        }
    }

    /// Creates a sender that modify `var` from other threads and without access to [`Vars`].
    ///
    /// If the variable is read-only when a modification is received it is silently dropped.
    ///
    /// Drop the sender to release one reference to `var`.
    pub fn modify_sender<T, V>(&self, var: &V) -> VarModifySender<T>
    where
        T: VarValue,
        V: Var<T>,
    {
        let (sender, receiver) = flume::unbounded::<Box<dyn FnOnce(&mut T) + Send>>();

        if var.always_read_only() {
            self.receivers.borrow_mut().push(Box::new(move |_| {
                receiver.drain();
                !receiver.is_disconnected()
            }));
        } else {
            let var = var.clone();
            self.receivers.borrow_mut().push(Box::new(move |vars| {
                for modify in receiver.try_iter() {
                    let _ = var.modify_boxed(vars, modify);
                }
                !receiver.is_disconnected()
            }));
        }

        VarModifySender {
            wake: self.event_loop.clone(),
            sender,
        }
    }
}
impl Deref for Vars {
    type Target = VarsRead;

    fn deref(&self) -> &Self::Target {
        &self.read
    }
}

struct RunOnDrop<F: FnOnce()>(Option<F>);
impl<F: FnOnce()> RunOnDrop<F> {
    fn new(clean: F) -> Self {
        RunOnDrop(Some(clean))
    }
}
impl<F: FnOnce()> Drop for RunOnDrop<F> {
    fn drop(&mut self) {
        if let Some(clean) = self.0.take() {
            clean();
        }
    }
}

/// A variable update receiver that can be used from any thread and without access to [`Vars`].
///
/// Use [`VarsRead::receiver`] to create a receiver, drop to stop listening.
pub struct VarReceiver<T: VarValue + Send> {
    receiver: flume::Receiver<T>,
}
impl<T: VarValue + Send> Clone for VarReceiver<T> {
    fn clone(&self) -> Self {
        VarReceiver {
            receiver: self.receiver.clone(),
        }
    }
}
impl<T: VarValue + Send> Debug for VarReceiver<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "VarReceiver<{}>", type_name::<T>())
    }
}
impl<T: VarValue + Send> VarReceiver<T> {
    /// Receives the oldest sent update not received, blocks until the variable updates.
    #[inline]
    pub fn recv(&self) -> Result<T, AppShutdown<()>> {
        self.receiver.recv().map_err(|_| AppShutdown(()))
    }

    /// Tries to receive the oldest sent update, returns `Ok(args)` if there was at least
    /// one update, or returns `Err(None)` if there was no update or returns `Err(AppHasShutdown)` if the connected
    /// app has shutdown.
    #[inline]
    pub fn try_recv(&self) -> Result<T, Option<AppShutdown<()>>> {
        self.receiver.try_recv().map_err(|e| match e {
            flume::TryRecvError::Empty => None,
            flume::TryRecvError::Disconnected => Some(AppShutdown(())),
        })
    }

    /// Receives the oldest sent update, blocks until the event updates or until the `deadline` is reached.
    #[inline]
    pub fn recv_deadline(&self, deadline: Instant) -> Result<T, TimeoutOrAppShutdown> {
        self.receiver.recv_deadline(deadline).map_err(TimeoutOrAppShutdown::from)
    }

    /// Receives the oldest sent update, blocks until the event updates or until timeout.
    #[inline]
    pub fn recv_timeout(&self, dur: Duration) -> Result<T, TimeoutOrAppShutdown> {
        self.receiver.recv_timeout(dur).map_err(TimeoutOrAppShutdown::from)
    }

    /// Returns a future that receives the oldest sent update, awaits until an event update occurs.
    #[inline]
    pub fn recv_async(&self) -> RecvFut<T> {
        self.receiver.recv_async().into()
    }

    /// Turns into a future that receives the oldest sent update, awaits until an event update occurs.
    #[inline]
    pub fn into_recv_async(self) -> RecvFut<'static, T> {
        self.receiver.into_recv_async().into()
    }

    /// Creates a blocking iterator over event updates, if there are no updates in the buffer the iterator blocks,
    /// the iterator only finishes when the app shuts-down.
    #[inline]
    pub fn iter(&self) -> flume::Iter<T> {
        self.receiver.iter()
    }

    /// Create a non-blocking iterator over event updates, the iterator finishes if
    /// there are no more updates in the buffer.
    #[inline]
    pub fn try_iter(&self) -> flume::TryIter<T> {
        self.receiver.try_iter()
    }
}
impl<T: VarValue + Send> From<VarReceiver<T>> for flume::Receiver<T> {
    fn from(e: VarReceiver<T>) -> Self {
        e.receiver
    }
}
impl<'a, T: VarValue + Send> IntoIterator for &'a VarReceiver<T> {
    type Item = T;

    type IntoIter = flume::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.receiver.iter()
    }
}
impl<T: VarValue + Send> IntoIterator for VarReceiver<T> {
    type Item = T;

    type IntoIter = flume::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.receiver.into_iter()
    }
}

/// A variable update sender that can set a variable from any thread and without access to [`Vars`].
///
/// Use [`Vars::sender`] to create a sender, drop to stop holding the paired variable in the UI thread.
pub struct VarSender<T>
where
    T: VarValue + Send,
{
    wake: EventLoopProxy,
    sender: flume::Sender<T>,
}
impl<T: VarValue + Send> Clone for VarSender<T> {
    fn clone(&self) -> Self {
        VarSender {
            wake: self.wake.clone(),
            sender: self.sender.clone(),
        }
    }
}
impl<T: VarValue + Send> Debug for VarSender<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "VarSender<{}>", type_name::<T>())
    }
}
impl<T> VarSender<T>
where
    T: VarValue + Send,
{
    /// Sends a new value for the variable, unless the connected app has shutdown.
    ///
    /// If the variable is read-only when the `new_value` is received it is silently dropped, if more then one
    /// value is sent before the app can process then, only the last value shows as an update in the UI thread.
    pub fn send(&self, new_value: T) -> Result<(), AppShutdown<T>> {
        self.sender.send(new_value).map_err(AppShutdown::from)?;
        let _ = self.wake.send_event(AppEvent::Var);
        Ok(())
    }
}

/// A variable modification sender that can be used to modify a variable from any thread and without access to [`Vars`].
///
/// Use [`Vars::modify_sender`] to create a sender, drop to stop holding the paired variable in the UI thread.
pub struct VarModifySender<T>
where
    T: VarValue,
{
    wake: EventLoopProxy,
    sender: flume::Sender<Box<dyn FnOnce(&mut T) + Send>>,
}
impl<T: VarValue> Clone for VarModifySender<T> {
    fn clone(&self) -> Self {
        VarModifySender {
            wake: self.wake.clone(),
            sender: self.sender.clone(),
        }
    }
}
impl<T: VarValue> Debug for VarModifySender<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "VarModifySender<{}>", type_name::<T>())
    }
}
impl<T> VarModifySender<T>
where
    T: VarValue,
{
    /// Sends a modification for the variable, unless the connected app has shutdown.
    ///
    /// If the variable is read-only when the `modify` is received it is silently dropped, if more then one
    /// modification is sent before the app can process then, they all are applied in order sent.
    pub fn send<F: FnOnce(&mut T) + Send + 'static>(&self, modify: F) -> Result<(), AppShutdown<()>> {
        self.sender.send(Box::new(modify)).map_err(|_| AppShutdown(()))?;
        let _ = self.wake.send_event(AppEvent::Var);
        Ok(())
    }
}

/// Variable sender used to notify the completion of an operation from any thread.
///
/// Use [`response_channel`] to init.
pub type ResponseSender<T> = VarSender<Response<T>>;

impl<T: VarValue + Send> ResponseSender<T> {
    /// Send the one time response.
    pub fn send_response(&self, response: T) -> Result<(), AppShutdown<T>> {
        self.send(Response::Done(response)).map_err(|e| {
            if let Response::Done(r) = e.0 {
                AppShutdown(r)
            } else {
                unreachable!()
            }
        })
    }
}

/// New paired [`ResponseSender`] and [`ResponseVar`] in the waiting state.
pub fn response_channel<T: VarValue + Send>(vars: &Vars) -> (ResponseSender<T>, ResponseVar<T>) {
    let (responder, response) = response_var();
    (vars.sender(&responder), response)
}
