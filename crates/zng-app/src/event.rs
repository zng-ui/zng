//! App event and commands API.

use std::{fmt, marker::PhantomData, ops, sync::Arc};

use crate::{handler::HandlerResult, widget::OnVarArgs};
use parking_lot::MappedRwLockReadGuard;
use zng_app_context::AppLocal;
use zng_task::channel;
use zng_var::{AnyVar, VARS, Var, VarHandle, VarUpdateId, VarValue};

use crate::{
    handler::Handler,
    update::UpdateOp,
    widget::{VarSubscribe, WIDGET, WidgetId},
};

mod args;
pub use args::*;

mod events;
pub use events::*;

mod command;
pub use command::*;

/// Event notifications from the last update cycle that notified.
#[derive(Clone, PartialEq, Debug)]
pub struct EventUpdates<A: EventArgs> {
    generation: VarUpdateId,
    updates: Vec<A>,
}
impl<A: EventArgs> ops::Deref for EventUpdates<A> {
    type Target = [A];

    fn deref(&self) -> &Self::Target {
        &self.updates
    }
}
impl<A: EventArgs> EventUpdates<A> {
    /// New empty.
    pub const fn none() -> Self {
        Self {
            generation: VarUpdateId::never(),
            updates: vec![],
        }
    }

    /// Last args in the list.
    pub fn latest(&self) -> Option<&A> {
        self.updates.last()
    }

    /// Call `handler` for arguments that target the `id` or a descendant of it.
    ///
    /// If `ignore_propagation` is `false` only calls the handler for args with [`propagation`] is not stopped.
    ///
    /// [`propagation`]: EventArgs::propagation
    pub fn each_relevant(&self, id: WidgetId, ignore_propagation: bool, mut handler: impl FnMut(&A)) {
        for args in &self.updates {
            if args.is_in_target(id) && (ignore_propagation || !args.propagation().is_stopped()) {
                handler(args);
            }
        }
    }

    /// Referent the latest args that target the `id` or a descendant of it.
    ///
    /// If `ignore_propagation` is `false` only calls the handler if the [`propagation`] is not stopped.
    ///
    /// [`propagation`]: EventArgs::propagation
    pub fn latest_relevant(&self, id: WidgetId, ignore_propagation: bool) -> Option<&A> {
        for args in self.updates.iter().rev() {
            if args.is_in_target(id) {
                if !ignore_propagation && args.propagation().is_stopped() {
                    break;
                }
                return Some(args);
            }
        }
        None
    }

    fn notify(&mut self, args: A) {
        if self.updates.is_empty() {
            self.updates.push(args);
        } else {
            let t = args.timestamp();
            if let Some(i) = self.updates.iter().position(|a| a.timestamp() > t) {
                self.updates.insert(i, args);
            } else {
                self.updates.push(args);
            }
        }
    }
}

#[doc(hidden)]
pub struct EventData {
    var: AnyVar,
    subscribe: fn(&AnyVar, UpdateOp, WidgetId) -> VarHandle,
}
impl EventData {
    pub fn new<A: EventArgs>() -> Self {
        Self {
            var: zng_var::var(EventUpdates::<A>::none()).into(),
            subscribe: Self::subscribe::<A>,
        }
    }

    fn subscribe<A: EventArgs>(var: &AnyVar, op: UpdateOp, widget_id: WidgetId) -> VarHandle {
        var.clone()
            .downcast::<EventUpdates<A>>()
            .unwrap()
            .subscribe_when(op, widget_id, move |v| v.value().latest_relevant(widget_id, true).is_some())
    }
}

/// Represents a type erased event variable.
pub struct AnyEvent(&'static AppLocal<EventData>);
impl Clone for AnyEvent {
    fn clone(&self) -> Self {
        *self
    }
}
impl Copy for AnyEvent {}
impl PartialEq for AnyEvent {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}
impl Eq for AnyEvent {}
impl std::hash::Hash for AnyEvent {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::ptr::from_ref(self.0).hash(state);
    }
}
impl fmt::Debug for AnyEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("AnyEvent").finish_non_exhaustive()
    }
}
impl AnyEvent {
    fn read_var(&self) -> MappedRwLockReadGuard<'_, AnyVar> {
        self.0.read_map(|v| &v.var)
    }

    /// Subscribe the widget to receive updates when events are relevant to it.
    pub fn subscribe(&self, op: UpdateOp, widget_id: WidgetId) -> VarHandle {
        let s = self.0.read();
        (s.subscribe)(&s.var, op, widget_id)
    }
}

/// Represents an event variable.
pub struct Event<A: EventArgs>(AnyEvent, PhantomData<fn() -> A>);
impl<A: EventArgs> fmt::Debug for Event<A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Event").finish_non_exhaustive()
    }
}
impl<A: EventArgs> ops::Deref for Event<A> {
    type Target = AnyEvent;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<A: EventArgs> Clone for Event<A> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<A: EventArgs> Copy for Event<A> {}
impl<A: EventArgs> PartialEq for Event<A> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}
impl<A: EventArgs> std::hash::Hash for Event<A> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}
impl<A: EventArgs> Eq for Event<A> {}
impl<A: EventArgs> Event<A> {
    #[doc(hidden)]
    pub const fn new(local: &'static AppLocal<EventData>) -> Self {
        Self(AnyEvent(local), PhantomData)
    }

    fn get_var(&self) -> Var<EventUpdates<A>> {
        self.0.0.read().var.clone().downcast::<EventUpdates<A>>().unwrap()
    }

    /// Variable that tracks all the args notified in the last update cycle.
    ///
    /// Note that the event variable is only cleared when new notifications are requested.
    pub fn var(&self) -> Var<EventUpdates<A>> {
        self.get_var().read_only()
    }

    /// Variable that tracks the latest update.
    ///
    /// Is only `None` if this event has never notified yet.
    pub fn var_latest(&self) -> Var<Option<A>> {
        self.get_var().map(|l| l.latest().cloned())
    }

    /// Filter map the latest args.
    ///
    /// The variable tracks the latest args that passes the `filter_map`. Every event update calls the closure for each
    /// pending args, latest first, and stops on the first args that produces a new value.
    pub fn var_map<O: VarValue>(
        &self,
        mut filter_map: impl FnMut(&A) -> Option<O> + Send + 'static,
        fallback_init: impl Fn() -> O + Send + 'static,
    ) -> Var<O> {
        self.read_var().filter_map(
            move |a| {
                for args in a.downcast_ref::<EventUpdates<A>>().unwrap().iter().rev() {
                    let r = filter_map(args);
                    if r.is_some() {
                        return r;
                    }
                }
                None
            },
            fallback_init,
        )
    }

    /// Bind filter the latest args to the variable.
    ///
    /// The `other` variable will be updated with the latest args that passes the `filter_map`.  Every event update calls the closure for each
    /// pending args, latest first, and stops on the first args that produces a new value.
    pub fn var_bind<O: VarValue>(&self, other: &Var<O>, mut filter_map: impl FnMut(&A) -> Option<O> + Send + 'static) -> VarHandle {
        self.read_var().bind_filter_map(other, move |a| {
            for args in a.downcast_ref::<EventUpdates<A>>().unwrap().iter().rev() {
                let r = filter_map(args);
                if r.is_some() {
                    return r;
                }
            }
            None
        })
    }

    /// Modify the event variable to include the `args` in the next update.
    pub fn notify(&self, args: A) {
        self.read_var()
            .modify(move |a| a.downcast_mut::<EventUpdates<A>>().unwrap().notify(args));
    }

    /// Visit each new update, oldest first, that target the context widget.
    ///
    /// If not called inside an widget visits all updates.
    ///
    /// If `ignore_propagation` is `false` only calls the handler if the [`propagation`] is not stopped.
    ///
    /// [`propagation`]: EventArgs::propagation
    pub fn each_update(&self, ignore_propagation: bool, mut handler: impl FnMut(&A)) {
        self.read_var().with_new(|u| {
            let u = u.downcast_ref::<EventUpdates<A>>().unwrap();
            if let Some(id) = WIDGET.try_id() {
                u.each_relevant(id, ignore_propagation, handler);
            } else {
                for args in u.iter() {
                    if ignore_propagation || !args.propagation().is_stopped() {
                        handler(args);
                    }
                }
            }
        });
    }

    /// Visit the latest update that targets the context widget.
    ///
    /// If not called inside an widget visits the latest in general.
    ///
    /// If `ignore_propagation` is `false` only calls the handler if the [`propagation`] is not stopped.
    ///
    /// [`propagation`]: EventArgs::propagation
    pub fn latest_update<O>(&self, ignore_propagation: bool, handler: impl FnOnce(&A) -> O) -> Option<O> {
        self.read_var()
            .with_new(|u| {
                let u = u.downcast_ref::<EventUpdates<A>>().unwrap();
                if let Some(id) = WIDGET.try_id() {
                    if let Some(args) = u.latest_relevant(id, ignore_propagation) {
                        return Some(handler(args));
                    }
                    None
                } else if let Some(args) = u.latest()
                    && (ignore_propagation || !args.propagation().is_stopped())
                {
                    Some(handler(args))
                } else {
                    None
                }
            })
            .flatten()
    }

    /// Subscribe the widget to receive updates when events are relevant to it and the latest args passes the `predicate`.
    pub fn subscribe_when(&self, op: UpdateOp, widget_id: WidgetId, predicate: impl Fn(&A) -> bool + Send + Sync + 'static) -> VarHandle {
        self.get_var().subscribe_when(op, widget_id, move |v| {
            v.value().latest_relevant(widget_id, true).map(&predicate).unwrap_or(false)
        })
    }

    /// Creates a preview event handler.
    ///
    /// The event `handler` is called for every update that has not stopped [`propagation`](AnyEventArgs::propagation).
    /// The handler is called before widget handlers and [`on_event`](Self::on_event) handlers. The handler is called
    /// after all previous registered preview handlers.
    ///
    /// If `ignore_propagation` is set also call handlers for args with stopped propagation.
    ///
    /// Returns an [`EventHandle`] that can be dropped to unsubscribe, you can also unsubscribe from inside the handler by calling
    /// [`unsubscribe`](crate::handler::APP_HANDLER::unsubscribe).
    ///
    /// # Examples
    ///
    /// ```
    /// # use zng_app::event::*;
    /// # use zng_app::APP;
    /// # use zng_app::handler::hn;
    /// # event_args! { pub struct FocusChangedArgs { pub new_focus: bool, .. fn delivery_list(&self, _l: &mut UpdateDeliveryList) {} } }
    /// # event! { pub static FOCUS_CHANGED_EVENT: FocusChangedArgs; }
    /// # let _scope = APP.minimal();
    /// let handle = FOCUS_CHANGED_EVENT.on_pre_event(hn!(|args| {
    ///     println!("focused: {:?}", args.new_focus);
    /// }));
    /// ```
    /// The example listens to all `FOCUS_CHANGED_EVENT` events, independent of widget context and before all UI handlers.
    ///
    /// # Handlers
    ///
    /// the event handler can be any [`Handler<A>`], there are multiple flavors of handlers, including
    /// async handlers that allow calling `.await`. The handler closures can be declared using [`hn!`], [`async_hn!`],
    /// [`hn_once!`] and [`async_hn_once!`].
    ///
    /// ## Async
    ///
    /// Note that for async handlers only the code before the first `.await` is called in the *preview* moment, code after runs in
    /// subsequent event updates, after the event has already propagated, so stopping [`propagation`](AnyEventArgs::propagation)
    /// only causes the desired effect before the first `.await`.
    ///
    /// [`hn!`]: crate::handler::hn!
    /// [`async_hn!`]: crate::handler::async_hn!
    /// [`hn_once!`]: crate::handler::hn_once!
    /// [`async_hn_once!`]: crate::handler::async_hn_once!
    pub fn on_pre_event(&self, ignore_propagation: bool, handler: Handler<A>) -> VarHandle {
        self.get_var().on_pre_new(Self::event_handler(ignore_propagation, handler))
    }

    /// Creates an event handler.
    ///
    /// The event `handler` is called for every update that has not stopped [`propagation`](AnyEventArgs::propagation).
    /// The handler is called after all [`on_pre_event`](Self::on_pre_event) all widget handlers and all [`on_event`](Self::on_event)
    /// handlers registered before this one.
    /// 
    /// If `ignore_propagation` is set also call handlers for args with stopped propagation.
    ///
    /// Returns an [`EventHandle`] that can be dropped to unsubscribe, you can also unsubscribe from inside the handler by calling
    /// [`unsubscribe`](crate::handler::APP_HANDLER::unsubscribe) in the third parameter of [`hn!`] or [`async_hn!`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use zng_app::event::*;
    /// # use zng_app::APP;
    /// # use zng_app::handler::hn;
    /// # event_args! { pub struct FocusChangedArgs { pub new_focus: bool, .. fn delivery_list(&self, _l: &mut UpdateDeliveryList) {} } }
    /// # event! { pub static FOCUS_CHANGED_EVENT: FocusChangedArgs; }
    /// # let _scope = APP.minimal();
    /// let handle = FOCUS_CHANGED_EVENT.on_event(hn!(|args| {
    ///     println!("focused: {:?}", args.new_focus);
    /// }));
    /// ```
    /// The example listens to all `FOCUS_CHANGED_EVENT` events, independent of widget context, after the UI was notified.
    ///
    /// # Handlers
    ///
    /// the event handler can be any [`Handler<A>`], there are multiple flavors of handlers, including
    /// async handlers that allow calling `.await`. The handler closures can be declared using [`hn!`], [`async_hn!`],
    /// [`hn_once!`] and [`async_hn_once!`].
    ///
    /// ## Async
    ///
    /// Note that for async handlers only the code before the first `.await` is called in the *preview* moment, code after runs in
    /// subsequent event updates, after the event has already propagated, so stopping [`propagation`](AnyEventArgs::propagation)
    /// only causes the desired effect before the first `.await`.
    ///
    /// [`hn!`]: crate::handler::hn!
    /// [`async_hn!`]: crate::handler::async_hn!
    /// [`hn_once!`]: crate::handler::hn_once!
    /// [`async_hn_once!`]: crate::handler::async_hn_once!
    pub fn on_event(&self, ignore_propagation: bool, handler: Handler<A>) -> VarHandle {
        self.get_var().on_new(Self::event_handler(ignore_propagation, handler))
    }

    fn event_handler(ignore_propagation: bool, mut handler: Handler<A>) -> Handler<OnVarArgs<EventUpdates<A>>> {
        Box::new(move |a| {
            let mut futs = vec![];
            for args in a.value.iter() {
                if !ignore_propagation && args.propagation().is_stopped() {
                    continue;
                }
                match handler(args) {
                    HandlerResult::Done => {}
                    HandlerResult::Continue(f) => futs.push(f),
                }
            }
            if futs.is_empty() {
                HandlerResult::Done
            } else if futs.len() == 1 {
                HandlerResult::Continue(futs.remove(0))
            } else {
                HandlerResult::Continue(Box::pin(async move {
                    for f in futs {
                        f.await;
                    }
                }))
            }
        })
    }

    /// Creates a receiver channel for the event. The event updates are send on hook, before even preview handlers.
    /// The receiver is unbounded, it will fill indefinitely if not drained. The receiver can be used in any thread,
    /// including non-app threads.
    ///
    /// Drop the receiver to stop listening.
    pub fn receiver(&self) -> channel::Receiver<A>
    where
        A: Send,
    {
        let (sender, receiver) = channel::unbounded();

        self.hook(move |args| sender.send_blocking(args.clone()).is_ok()).perm();

        receiver
    }

    /// Deref as [`AnyEvent`].
    pub fn as_any(&self) -> &AnyEvent {
        self
    }

    /// Setups a callback for just after the event notifications are listed,
    /// the closure runs in the root app context, just like var modify and hook closures.
    ///
    /// The closure must return true to be retained and false to be dropped.
    ///
    /// Any event notification or var modification done in the `handler` will apply on the same update that notifies this event.
    pub fn hook(&self, mut handler: impl FnMut(&A) -> bool + Send + 'static) -> VarHandle {
        // events can be modified multiple times in the same hooks resolution, every var hook update will list all *pending*
        // args for the next update, to avoid calling `handler` for the same args we track already called
        let mut last_call_id = VarUpdateId::never();
        let mut last_call_take = 0;
        self.read_var().hook(move |a| {
            let updates = a.downcast_value::<EventUpdates<A>>().unwrap();
            let id = VARS.update_id();
            let mut skip = 0;
            if last_call_id != id {
                last_call_id = id;
                last_call_take = 0;
            } else {
                skip = last_call_take;
                last_call_take = updates.len();
            }

            // notify
            for args in updates[skip..].iter() {
                if !handler(args) {
                    return false;
                }
            }
            true
        })
    }

    /// Wait until any args, current or new passes the `predicate`.
    pub async fn wait_match(&self, predicate: impl Fn(&A) -> bool + Send + Sync + 'static) {
        self.get_var().wait_match(move |a| a.iter().any(&predicate)).await
    }
}

#[doc(hidden)]
#[macro_export]
macro_rules! event_macro_impl {
    (
        $(#[$attr:meta])*
        $vis:vis static $EVENT:ident: $Args:path;
    ) => {
        $(#[$attr])*
        $vis static $EVENT: $crate::event::Event<$Args> = {
            $crate::event::app_local! {
                static LOCAL: $crate::event::EventData = $crate::event::EventData::new::<$Args>();
            }
            $crate::event::Event::new(&LOCAL)
        };
    };
    (
        $(#[$attr:meta])*
        $vis:vis static $EVENT:ident: $Args:path { $($init:tt)* };
    ) => {
        $(#[$attr])*
        $vis static $EVENT: $crate::event::Event<$Args> = {
            fn __init_event__() {
                $($init)*
            }
            $crate::event::app_local! {
                static LOCAL: $crate::event::EventData = {
                    $crate::event::EVENTS.notify("event init", __init_event__);
                    $crate::event::EventData::new::<$Args>()
                };
            }
            $crate::event::Event::new(&LOCAL)
        };
    };
}

///<span data-del-macro-root></span> Declares new [`Event<A>`] static items.
///
/// Event static items represent external, app or widget events. You can also use [`command!`]
/// to declare events specialized for commanding widgets and services.
///
/// # Conventions
///
/// Command events have the `_EVENT` suffix, for example an event representing a click is called `CLICK_EVENT`.
///
/// # Properties
///
/// If the event targets widgets you can use `event_property!` to declare properties that setup event handlers for the event.
///
/// # Examples
///
/// The example defines two events with the same arguments type.
///
/// ```
/// # use zng_app::event::*;
/// # event_args! { pub struct ClickArgs { .. fn is_in_target(&self, _id: WidgetId) -> bool { true } } }
/// event! {
///     /// Event docs.
///     pub static CLICK_EVENT: ClickArgs;
///
///     /// Other event docs.
///     pub static DOUBLE_CLICK_EVENT: ClickArgs;
/// }
/// ```
#[macro_export]
macro_rules! event_macro {
    ($(
        $(#[$attr:meta])*
        $vis:vis static $EVENT:ident: $Args:path $({ $($init:tt)* })?;
    )+) => {
        $(
            $crate::event_macro_impl! {
                $(#[$attr])*
                $vis static $EVENT: $Args $({ $($init)* })?;
            }
        )+
    }
}
#[doc(inline)]
pub use crate::event_macro as event;
