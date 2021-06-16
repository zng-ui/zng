use retain_mut::RetainMut;

use super::*;
use crate::{
    app::{AppEventSender, AppShutdown, RecvFut, TimeoutOrAppShutdown},
    context::Updates,
    crate_util::RunOnDrop,
};
use std::{
    any::type_name,
    cell::{Cell, RefCell},
    fmt,
    ops::Deref,
    rc::Rc,
    time::{Duration, Instant},
};

thread_singleton!(SingletonVars);

type SyncEntry = Box<dyn Fn(&Vars) -> Retain>;
type Retain = bool;

type VarBinding = Box<dyn FnMut(&Vars) -> Retain>;

/// Read-only access to [`Vars`].
///
/// In some contexts variables can be set, so a full [`Vars`] reference if given, in other contexts
/// variables can only be read, so a [`VarsRead`] reference is given.
///
/// [`Vars`] auto-dereferences to to this type.
///
/// # Examples
///
/// You can [`get`](Var::get) a value using a [`VarsRead`] reference.
///
/// ```
/// # use zero_ui_core::var::{Var, VarsRead};
/// fn read_only(var: &impl Var<bool>, vars: &VarsRead) -> bool {
///     *var.get(vars)
/// }
/// ```
///
/// And because of auto-dereference you can can the same method using a full [`Vars`] reference.
///
/// ```
/// # use zero_ui_core::var::{Var, Vars};
/// fn read_write(var: &impl Var<bool>, vars: &Vars) -> bool {
///     *var.get(vars)
/// }
/// ```
pub struct VarsRead {
    _singleton: SingletonVars,
    update_id: u32,
    #[allow(clippy::type_complexity)]
    widget_clear: RefCell<Vec<Box<dyn Fn(bool)>>>,

    app_event_sender: AppEventSender,
    senders: RefCell<Vec<SyncEntry>>,
    receivers: RefCell<Vec<SyncEntry>>,
}
impl fmt::Debug for VarsRead {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "VarsRead {{ .. }}")
    }
}
impl VarsRead {
    /// Id of the current update cycle, can be used to determinate if a variable value is new.
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
        V: Var<C::Type>,
    {
        self.with_context_var_impl(context_var, other_var.get(self), false, other_var.version(self), f)
    }

    /// Calls [`with_context_var_wgt_only`](Vars::with_context_var_wgt_only) with values from `other_var`.
    #[inline(always)]
    pub fn with_context_bind_wgt_only<C: ContextVar, R, F: FnOnce() -> R, V: Var<C::Type>>(
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

type PendingUpdate = Box<dyn FnOnce(u32) -> bool>;

/// Access to application variables.
///
/// An instance of this struct in [`AppContext`](crate::context::AppContext) and derived contexts.
pub struct Vars {
    read: VarsRead,

    binding_update_id: u32,
    bindings: RefCell<Vec<VarBinding>>,

    #[allow(clippy::type_complexity)]
    pending: RefCell<Vec<PendingUpdate>>,
}
impl fmt::Debug for Vars {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Vars {{ .. }}")
    }
}
impl Vars {
    /// If an instance of `Vars` already exists in the  current thread.
    #[inline]
    pub(crate) fn instantiated() -> bool {
        SingletonVars::in_use()
    }

    /// Produces the instance of `Vars`. Only a single
    /// instance can exist in a thread at a time, panics if called
    /// again before dropping the previous instance.
    #[inline]
    pub(crate) fn instance(app_event_sender: AppEventSender) -> Self {
        Vars {
            read: VarsRead {
                _singleton: SingletonVars::assert_new("Vars"),
                update_id: 0u32.wrapping_sub(13),
                app_event_sender,
                widget_clear: Default::default(),
                senders: RefCell::default(),
                receivers: RefCell::default(),
            },
            binding_update_id: 0u32.wrapping_sub(13),
            bindings: RefCell::default(),
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
    pub fn with_context_bind<C: ContextVar, F: FnOnce(), V: Var<C::Type>>(&self, context_var: C, other_var: &V, f: F) {
        self.with_context_var_impl(context_var, other_var.get(self), other_var.is_new(self), other_var.version(self), f)
    }

    /// Calls [`with_context_var_wgt_only`](Vars::with_context_var_wgt_only) with values from `other_var`.
    #[inline(always)]
    pub fn with_context_bind_wgt_only<C: ContextVar, F: FnOnce(), V: Var<C::Type>>(&self, context_var: C, other_var: &V, f: F) {
        self.with_context_var_wgt_only(context_var, other_var.get(self), other_var.is_new(self), other_var.version(self), f)
    }

    /// Schedule set/modify.
    pub(super) fn push_change(&self, change: PendingUpdate) {
        self.pending.borrow_mut().push(change);
    }

    /// Apply scheduled set/modify.
    pub(crate) fn apply_updates(&mut self, updates: &mut Updates) {
        self.read.update_id = self.update_id.wrapping_add(1);

        let pending = self.pending.get_mut();
        if !pending.is_empty() {
            let mut modified = false;
            for f in pending.drain(..) {
                modified |= f(self.read.update_id);
            }

            if modified {
                // update bindings
                if !self.bindings.get_mut().is_empty() {
                    self.binding_update_id = self.binding_update_id.wrapping_add(1);

                    loop {
                        self.bindings.borrow_mut().retain_mut(|f| f(self));

                        let pending = self.pending.get_mut();
                        if pending.is_empty() {
                            break;
                        }
                        for f in pending.drain(..) {
                            f(self.read.update_id);
                        }
                    }
                }

                // send values.
                self.senders.borrow_mut().retain(|f| f(self));

                // does an app update because some vars have new values.
                updates.update();
            }
        }
    }

    /// Receive and apply set/modify from [`VarSender`] and [`VarModifySender`] instances.
    pub(crate) fn receive_sended_modify(&self) {
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
            wake: self.app_event_sender.clone(),
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
        let (sender, receiver) = flume::unbounded::<Box<dyn FnOnce(&mut VarModify<T>) + Send>>();

        if var.always_read_only() {
            self.receivers.borrow_mut().push(Box::new(move |_| {
                receiver.drain();
                !receiver.is_disconnected()
            }));
        } else {
            let var = var.clone();
            self.receivers.borrow_mut().push(Box::new(move |vars| {
                for modify in receiver.try_iter() {
                    let _ = var.modify(vars, modify);
                }
                !receiver.is_disconnected()
            }));
        }

        VarModifySender {
            wake: self.app_event_sender.clone(),
            sender,
        }
    }

    /// Create a [`map`](Var::map) like binding between two existing variables.
    pub fn bind<A, B, AV, BV, M>(&self, from_var: AV, to_var: BV, mut map: M) -> VarBindingHandle
    where
        A: VarValue,
        B: VarValue,
        AV: Var<A>,
        BV: Var<B>,
        M: FnMut(&VarBindingInfo, &A) -> B + 'static,
    {
        let handle = VarBindingHandle::new();
        let u_handle = handle.clone();

        let mut last_update_id = self.binding_update_id;

        self.bindings.borrow_mut().push(Box::new(move |vars| {
            let mut retain = handle.retain();

            if vars.binding_update_id == last_update_id {
                return retain;
            }

            if retain {
                if let Some(new_value) = from_var.get_new(vars) {
                    last_update_id = vars.binding_update_id;

                    let info = VarBindingInfo::new();
                    let new_value = map(&info, new_value);
                    let _ = to_var.set(vars, new_value);
                    if info.unbind.get() {
                        retain = false;
                    }
                }
            }
            retain
        }));
        u_handle
    }

    /// Create a [`map_bidi`](Var::map_bidi) like binding between two existing variables.
    pub fn bind_bidi<A, B, AV, BV, M, N>(&self, from_var: AV, to_var: BV, mut map: M, mut map_back: N) -> VarBindingHandle
    where
        A: VarValue,
        B: VarValue,
        AV: Var<A>,
        BV: Var<B>,
        M: FnMut(&VarBindingInfo, &A) -> B + 'static,
        N: FnMut(&VarBindingInfo, &B) -> A + 'static,
    {
        let handle = VarBindingHandle::new();
        let u_handle = handle.clone();

        let mut last_update_id = self.binding_update_id;

        self.bindings.borrow_mut().push(Box::new(move |vars| {
            let mut retain = handle.retain();

            if vars.binding_update_id == last_update_id {
                return retain;
            }

            if retain {
                if let Some(new_value) = from_var.get_new(vars) {
                    last_update_id = vars.binding_update_id;

                    let info = VarBindingInfo::new();
                    let new_value = map(&info, new_value);
                    let _ = to_var.set(vars, new_value);
                    if info.unbind.get() {
                        retain = false;
                    }
                }
                if let Some(new_value) = to_var.get_new(vars) {
                    last_update_id = vars.binding_update_id;

                    let info = VarBindingInfo::new();
                    let new_value = map_back(&info, new_value);
                    let _ = from_var.set(vars, new_value);
                    if info.unbind.get() {
                        retain = false;
                    }
                }
            }
            retain
        }));
        u_handle
    }

    /// Create a [`filter_map`](Var::filter_map) like binding between two existing variables.
    pub fn bind_filtered<A, B, AV, BV, M>(&self, from_var: AV, to_var: BV, mut map: M) -> VarBindingHandle
    where
        A: VarValue,
        B: VarValue,
        AV: Var<A>,
        BV: Var<B>,
        M: FnMut(&VarBindingInfo, &A) -> Option<B> + 'static,
    {
        let handle = VarBindingHandle::new();
        let u_handle = handle.clone();

        let mut last_update_id = self.binding_update_id;

        self.bindings.borrow_mut().push(Box::new(move |vars| {
            let mut retain = handle.retain();

            if vars.binding_update_id == last_update_id {
                return retain;
            }

            if retain {
                if let Some(new_value) = from_var.get_new(vars) {
                    last_update_id = vars.binding_update_id;

                    let info = VarBindingInfo::new();
                    if let Some(new_value) = map(&info, new_value) {
                        let _ = to_var.set(vars, new_value);
                    }
                    if info.unbind.get() {
                        retain = false;
                    }
                }
            }
            retain
        }));
        u_handle
    }

    /// Create a [`filter_map_bidi`](Var::filter_map_bidi) like binding between two existing variables.
    pub fn bind_bidi_filtered<A, B, AV, BV, M, N>(&self, from_var: AV, to_var: BV, mut map: M, mut map_back: N) -> VarBindingHandle
    where
        A: VarValue,
        B: VarValue,
        AV: Var<A>,
        BV: Var<B>,
        M: FnMut(&VarBindingInfo, &A) -> Option<B> + 'static,
        N: FnMut(&VarBindingInfo, &B) -> Option<A> + 'static,
    {
        let handle = VarBindingHandle::new();
        let u_handle = handle.clone();

        let mut last_update_id = self.binding_update_id;

        self.bindings.borrow_mut().push(Box::new(move |vars| {
            let mut retain = handle.retain();

            if vars.binding_update_id == last_update_id {
                return retain;
            }

            if retain {
                if let Some(new_value) = from_var.get_new(vars) {
                    last_update_id = vars.binding_update_id;

                    let info = VarBindingInfo::new();
                    if let Some(new_value) = map(&info, new_value) {
                        let _ = to_var.set(vars, new_value);
                    }
                    if info.unbind.get() {
                        retain = false;
                    }
                }
                if let Some(new_value) = to_var.get_new(vars) {
                    last_update_id = vars.binding_update_id;

                    let info = VarBindingInfo::new();
                    if let Some(new_value) = map_back(&info, new_value) {
                        let _ = from_var.set(vars, new_value);
                    }
                    if info.unbind.get() {
                        retain = false;
                    }
                }
            }
            retain
        }));
        u_handle
    }
}
impl Deref for Vars {
    type Target = VarsRead;

    fn deref(&self) -> &Self::Target {
        &self.read
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
impl<T: VarValue + Send> fmt::Debug for VarReceiver<T> {
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
    wake: AppEventSender,
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
impl<T: VarValue + Send> fmt::Debug for VarSender<T> {
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
        let _ = self.wake.send_var();
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
    wake: AppEventSender,
    sender: flume::Sender<Box<dyn FnOnce(&mut VarModify<T>) + Send>>,
}
impl<T: VarValue> Clone for VarModifySender<T> {
    fn clone(&self) -> Self {
        VarModifySender {
            wake: self.wake.clone(),
            sender: self.sender.clone(),
        }
    }
}
impl<T: VarValue> fmt::Debug for VarModifySender<T> {
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
    pub fn send<F>(&self, modify: F) -> Result<(), AppShutdown<()>>
    where
        F: FnOnce(&mut VarModify<T>) + Send + 'static,
    {
        self.sender.send(Box::new(modify)).map_err(|_| AppShutdown(()))?;
        let _ = self.wake.send_var();
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

/// Represents a variable binding created one of the `bind` methods of [`Vars`].
///
/// Drop all clones of this handle to drop the binding, or call [`forget`](Self::forget) to drop the handle
/// without dropping the binding.
#[derive(Clone)]
#[must_use = "the var binding is removed if the handle is dropped"]
pub struct VarBindingHandle(Rc<VarBindingData>);
struct VarBindingData {
    forget: Cell<bool>,
    unbind: Cell<bool>,
}
impl VarBindingHandle {
    fn new() -> Self {
        VarBindingHandle(Rc::new(VarBindingData {
            forget: Cell::new(false),
            unbind: Cell::new(false),
        }))
    }

    /// Drop the handle but does **not** drop the binding.
    ///
    ///
    /// This method does not work like [`std::mem::forget`], **no memory is leaked**, the handle
    /// memory is released immediately and the binding, memory is released when application shuts-down.
    #[inline]
    pub fn forget(self) {
        self.0.forget.set(true);
    }

    /// Drops the handle and forces the binding to drop.
    #[inline]
    pub fn unbind(self) {
        self.0.unbind.set(true);
    }

    fn retain(&self) -> bool {
        !self.0.unbind.get() && (Rc::strong_count(&self.0) > 1 || self.0.forget.get())
    }
}

/// Represents a variable binding in the binding closure.
///
/// All of the `bind` methods of [`Vars`] take a closure that take a reference to this info
/// as input, they can use it to drop the variable binding from the inside.
pub struct VarBindingInfo {
    unbind: Cell<bool>,
}
impl VarBindingInfo {
    fn new() -> Self {
        VarBindingInfo { unbind: Cell::new(false) }
    }

    /// Drop the binding after applying the returned update.
    #[inline]
    pub fn unbind(&self) {
        self.unbind.set(true);
    }
}

#[cfg(test)]
mod tests {
    use crate::app::App;
    use crate::text::ToText;
    use crate::var::var;

    #[test]
    fn one_way_binding() {
        let a = var(10);
        let b = var("".to_text());

        let mut app = App::blank().run_headless();

        app.ctx().vars.bind(a.clone(), b.clone(), |_, a| a.to_text()).forget();

        let mut update_count = 0;
        app.update_observe(
            |_| {
                update_count += 1;
            },
            false,
        );
        assert_eq!(0, update_count);

        a.set(app.ctx().vars, 20);

        update_count = 0;
        app.update_observe(
            |ctx| {
                update_count += 1;
                assert_eq!(Some(&20i32), a.get_new(ctx.vars));
                assert_eq!(Some(&"20".to_text()), b.get_new(ctx.vars));
            },
            false,
        );
        assert_eq!(1, update_count);

        a.set(app.ctx().vars, 13);

        update_count = 0;
        app.update_observe(
            |ctx| {
                update_count += 1;
                assert_eq!(Some(&13i32), a.get_new(ctx.vars));
                assert_eq!(Some(&"13".to_text()), b.get_new(ctx.vars));
            },
            false,
        );
        assert_eq!(1, update_count);
    }

    #[test]
    fn two_way_binding() {
        let a = var(10);
        let b = var("".to_text());

        let mut app = App::blank().run_headless();

        app.ctx()
            .vars
            .bind_bidi(a.clone(), b.clone(), |_, a| a.to_text(), |_, b| b.parse().unwrap())
            .forget();

        let mut update_count = 0;
        app.update_observe(
            |_| {
                update_count += 1;
            },
            false,
        );
        assert_eq!(0, update_count);

        a.set(app.ctx().vars, 20);

        update_count = 0;
        app.update_observe(
            |ctx| {
                update_count += 1;
                assert_eq!(Some(&20i32), a.get_new(ctx.vars));
                assert_eq!(Some(&"20".to_text()), b.get_new(ctx.vars));
            },
            false,
        );
        assert_eq!(1, update_count);

        b.set(app.ctx().vars, "55");

        update_count = 0;
        app.update_observe(
            |ctx| {
                update_count += 1;
                assert_eq!(Some(&"55".to_text()), b.get_new(ctx.vars));
                assert_eq!(Some(&55i32), a.get_new(ctx.vars));
            },
            false,
        );
        assert_eq!(1, update_count);
    }

    #[test]
    fn one_way_filtered_binding() {
        let a = var(10);
        let b = var("".to_text());

        let mut app = App::blank().run_headless();

        app.ctx()
            .vars
            .bind_filtered(a.clone(), b.clone(), |_, a| if *a == 13 { None } else { Some(a.to_text()) })
            .forget();

        let mut update_count = 0;
        app.update_observe(
            |_| {
                update_count += 1;
            },
            false,
        );
        assert_eq!(0, update_count);

        a.set(app.ctx().vars, 20);

        update_count = 0;
        app.update_observe(
            |ctx| {
                update_count += 1;
                assert_eq!(Some(&20i32), a.get_new(ctx.vars));
                assert_eq!(Some(&"20".to_text()), b.get_new(ctx.vars));
            },
            false,
        );
        assert_eq!(1, update_count);

        a.set(app.ctx().vars, 13);

        update_count = 0;
        app.update_observe(
            |ctx| {
                update_count += 1;
                assert_eq!(Some(&13i32), a.get_new(ctx.vars));
                assert_eq!(&"20".to_text(), b.get(ctx.vars));
                assert!(!b.is_new(ctx.vars));
            },
            false,
        );
        assert_eq!(1, update_count);
    }

    #[test]
    fn two_way_filtered_binding() {
        let a = var(10);
        let b = var("".to_text());

        let mut app = App::blank().run_headless();

        app.ctx()
            .vars
            .bind_bidi_filtered(a.clone(), b.clone(), |_, a| Some(a.to_text()), |_, b| b.parse().ok())
            .forget();

        let mut update_count = 0;
        app.update_observe(
            |_| {
                update_count += 1;
            },
            false,
        );
        assert_eq!(0, update_count);

        a.set(app.ctx().vars, 20);

        update_count = 0;
        app.update_observe(
            |ctx| {
                update_count += 1;
                assert_eq!(Some(&20i32), a.get_new(ctx.vars));
                assert_eq!(Some(&"20".to_text()), b.get_new(ctx.vars));
            },
            false,
        );
        assert_eq!(1, update_count);

        b.set(app.ctx().vars, "55");

        update_count = 0;
        app.update_observe(
            |ctx| {
                update_count += 1;
                assert_eq!(Some(&"55".to_text()), b.get_new(ctx.vars));
                assert_eq!(Some(&55i32), a.get_new(ctx.vars));
            },
            false,
        );
        assert_eq!(1, update_count);

        b.set(app.ctx().vars, "not a i32");

        update_count = 0;
        app.update_observe(
            |ctx| {
                update_count += 1;
                assert_eq!(Some(&"not a i32".to_text()), b.get_new(ctx.vars));
                assert_eq!(&55i32, a.get(ctx.vars));
                assert!(!a.is_new(ctx.vars));
            },
            false,
        );
        assert_eq!(1, update_count);
    }

    #[test]
    fn binding_chain() {
        let a = var(0);
        let b = var(0);
        let c = var(0);
        let d = var(0);

        let mut app = App::blank().run_headless();

        app.ctx().vars.bind(a.clone(), b.clone(), |_, a| *a + 1).forget();
        app.ctx().vars.bind(b.clone(), c.clone(), |_, b| *b + 1).forget();
        app.ctx().vars.bind(c.clone(), d.clone(), |_, c| *c + 1).forget();

        let mut update_count = 0;
        app.update_observe(
            |_| {
                update_count += 1;
            },
            false,
        );
        assert_eq!(0, update_count);

        a.set(app.ctx().vars, 20);

        let mut update_count = 0;
        app.update_observe(
            |ctx| {
                update_count += 1;

                assert_eq!(Some(&20), a.get_new(ctx.vars));
                assert_eq!(Some(&21), b.get_new(ctx.vars));
                assert_eq!(Some(&22), c.get_new(ctx.vars));
                assert_eq!(Some(&23), d.get_new(ctx.vars));
            },
            false,
        );
        assert_eq!(1, update_count);

        a.set(app.ctx().vars, 30);

        let mut update_count = 0;
        app.update_observe(
            |ctx| {
                update_count += 1;

                assert_eq!(Some(&30), a.get_new(ctx.vars));
                assert_eq!(Some(&31), b.get_new(ctx.vars));
                assert_eq!(Some(&32), c.get_new(ctx.vars));
                assert_eq!(Some(&33), d.get_new(ctx.vars));
            },
            false,
        );
        assert_eq!(1, update_count);
    }

    #[test]
    fn binding_bidi_chain() {
        let a = var(0);
        let b = var(0);
        let c = var(0);
        let d = var(0);

        let mut app = App::blank().run_headless();

        app.ctx().vars.bind_bidi(a.clone(), b.clone(), |_, a| *a, |_, b| *b).forget();
        app.ctx().vars.bind_bidi(b.clone(), c.clone(), |_, b| *b, |_, c| *c).forget();
        app.ctx().vars.bind_bidi(c.clone(), d.clone(), |_, c| *c, |_, d| *d).forget();

        let mut update_count = 0;
        app.update_observe(
            |_| {
                update_count += 1;
            },
            false,
        );
        assert_eq!(0, update_count);

        a.set(app.ctx().vars, 20);

        let mut update_count = 0;
        app.update_observe(
            |ctx| {
                update_count += 1;

                assert_eq!(Some(&20), a.get_new(ctx.vars));
                assert_eq!(Some(&20), b.get_new(ctx.vars));
                assert_eq!(Some(&20), c.get_new(ctx.vars));
                assert_eq!(Some(&20), d.get_new(ctx.vars));
            },
            false,
        );
        assert_eq!(1, update_count);

        d.set(app.ctx().vars, 30);

        let mut update_count = 0;
        app.update_observe(
            |ctx| {
                update_count += 1;

                assert_eq!(Some(&30), a.get_new(ctx.vars));
                assert_eq!(Some(&30), b.get_new(ctx.vars));
                assert_eq!(Some(&30), c.get_new(ctx.vars));
                assert_eq!(Some(&30), d.get_new(ctx.vars));
            },
            false,
        );
        assert_eq!(1, update_count);
    }

    #[test]
    fn binding_drop_from_inside() {
        let a = var(1);
        let b = var(1);

        let mut app = App::blank().run_headless();

        let _handle = app.ctx().vars.bind(a.clone(), b.clone(), |info, i| {
            info.unbind();
            *i + 1
        });

        a.set(app.ctx().vars, 10);

        let mut update_count = 0;
        app.update_observe(
            |ctx| {
                update_count += 1;

                assert_eq!(Some(&10), a.get_new(ctx.vars));
                assert_eq!(Some(&11), b.get_new(ctx.vars));
            },
            false,
        );
        assert_eq!(1, update_count);

        assert_eq!(1, a.strong_count());
        assert_eq!(1, b.strong_count());

        a.set(app.ctx().vars, 100);

        update_count = 0;
        app.update_observe(
            |ctx| {
                update_count += 1;

                assert_eq!(Some(&100), a.get_new(ctx.vars));
                assert!(!b.is_new(ctx.vars));
                assert_eq!(&11, b.get(ctx.vars));
            },
            false,
        );
        assert_eq!(1, update_count);
    }

    #[test]
    fn binding_drop_from_outside() {
        let a = var(1);
        let b = var(1);

        let mut app = App::blank().run_headless();

        let handle = app.ctx().vars.bind(a.clone(), b.clone(), |_, i| *i + 1);

        a.set(app.ctx().vars, 10);

        let mut update_count = 0;
        app.update_observe(
            |ctx| {
                update_count += 1;

                assert_eq!(Some(&10), a.get_new(ctx.vars));
                assert_eq!(Some(&11), b.get_new(ctx.vars));
            },
            false,
        );
        assert_eq!(1, update_count);

        drop(handle);

        a.set(app.ctx().vars, 100);

        update_count = 0;
        app.update_observe(
            |ctx| {
                update_count += 1;

                assert_eq!(Some(&100), a.get_new(ctx.vars));
                assert!(!b.is_new(ctx.vars));
                assert_eq!(&11, b.get(ctx.vars));
            },
            false,
        );
        assert_eq!(1, update_count);

        assert_eq!(1, a.strong_count());
        assert_eq!(1, b.strong_count());
    }
}
