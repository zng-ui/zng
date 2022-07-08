//! App event API.

use unsafe_any::UnsafeAny;

use crate::app::{AppDisconnected, AppEventSender, RecvFut, TimeoutOrAppDisconnected};
use crate::command::AnyCommand;
use crate::context::{AppContext, InfoContext, UpdatesTrace, WidgetContext, WindowContext};
use crate::crate_util::{Handle, HandleOwner, WeakHandle};
use crate::handler::{AppHandler, AppHandlerArgs, AppWeakHandle, WidgetHandler};
use crate::var::Vars;
use crate::widget_info::{EventSlot, WidgetInfoBuilder, WidgetSubscriptions};
use crate::window::WindowId;
use crate::{impl_ui_node, UiNode, WidgetId, WidgetPath};
use std::cell::{Cell, RefCell};
use std::fmt::{self, Debug};
use std::marker::PhantomData;
use std::mem;
use std::ops::Deref;
use std::rc::Rc;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::{any::*, collections::VecDeque};

/// [`Event`] arguments.
pub trait EventArgs: Debug + Clone + 'static {
    /// Gets the instant this event happen.
    fn timestamp(&self) -> Instant;

    /// Generate an [`EventDeliveryList`] that defines all targets of the event.
    fn delivery_list(&self) -> EventDeliveryList;

    /// Propagation handle associated with this event instance.
    ///
    /// Cloned arguments share the same handle, some arguments may also share the handle
    /// of another event if they share the same cause.
    fn propagation(&self) -> &EventPropagationHandle;

    /// Calls `handler` and stops propagation if propagation is still allowed.
    ///
    /// Returns the `handler` result if it was called.
    fn handle<F, R>(&self, handler: F) -> Option<R>
    where
        F: FnOnce(&Self) -> R,
    {
        if self.propagation().is_stopped() {
            None
        } else {
            let r = handler(self);
            self.propagation().stop();
            Some(r)
        }
    }
}

/// Event propagation handle associated with one or multiple [`EventArgs`].
///
/// Event handlers can use this handle to signal subsequent handlers that they should skip handling the event.
///
/// You can get the propagation handle of any event argument by using the [`EventArgs::propagation`] method.
#[derive(Debug, Clone)]
pub struct EventPropagationHandle(Arc<AtomicBool>);
impl EventPropagationHandle {
    /// New in the not stopped default state.
    pub fn new() -> Self {
        EventPropagationHandle(Arc::new(AtomicBool::new(false)))
    }

    /// Signal subsequent handlers that the event is already handled.
    pub fn stop(&self) {
        // Is `Arc` to make `EventArgs` send, but stop handle is only useful in the UI thread, so
        // we don't need any ordering.
        self.0.store(true, std::sync::atomic::Ordering::Relaxed);
    }

    /// If the handler must skip this event instance.
    ///
    /// Note that property level handlers don't need to check this, as those handlers are
    /// not called when this is `true`. Direct event listeners in [`UiNode`] and [`AppExtension`]
    /// must check if this is `true`.
    ///
    /// [`UiNode`]: crate::UiNode
    /// [`AppExtension`]: crate::app::AppExtension
    pub fn is_stopped(&self) -> bool {
        self.0.load(std::sync::atomic::Ordering::Relaxed)
    }
}
impl Default for EventPropagationHandle {
    fn default() -> Self {
        EventPropagationHandle::new()
    }
}
impl PartialEq for EventPropagationHandle {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for EventPropagationHandle {}
impl std::hash::Hash for EventPropagationHandle {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let ptr = Arc::as_ptr(&self.0) as usize;
        std::hash::Hash::hash(&ptr, state);
    }
}

/// Identifies an event type.
///
/// Use [`event!`](macro@event) to declare.
#[cfg_attr(doc_nightly, doc(notable_trait))]
pub trait Event: Debug + Clone + Copy + 'static {
    /// Event arguments type.
    type Args: EventArgs;

    /// Schedule an event update.
    fn notify<Evs: WithEvents>(self, events: &mut Evs, args: Self::Args) {
        events.with_events(|events| events.notify::<Self>(self, args));
    }

    /// Gets the event arguments if the update is for `Self`.
    fn update<U: EventUpdateArgs>(self, args: &U) -> Option<&EventUpdate<Self>> {
        args.args_for::<Self>()
    }

    /// Gets the [`EventSlot`] assigned to this event type.
    fn slot(self) -> EventSlot;
}

/// [`EventUpdateArgs`] for event `E`, dereferences to the argument.
pub struct EventUpdate<E: Event> {
    args: E::Args,
    slot: EventSlot,
    delivery_list: EventDeliveryList,
}
impl<E: Event> EventUpdate<E> {
    /// New event update.
    pub fn new(event: E, args: E::Args) -> Self {
        EventUpdate {
            delivery_list: args.delivery_list(),
            args,
            slot: event.slot(),
        }
    }

    /// Clone the arguments.
    #[allow(clippy::should_implement_trait)] // that is what we want.
    pub fn clone(&self) -> E::Args {
        self.args.clone()
    }

    pub(crate) fn boxed(self) -> BoxedEventUpdate {
        BoxedEventUpdate {
            event_type: TypeId::of::<E>(),
            slot: self.slot,
            event_update_args: Box::new(self),
            delivery_list_deref: delivery_list_deref::<E>,
            debug_fmt: debug_fmt::<E>,
        }
    }

    fn boxed_send(self) -> BoxedSendEventUpdate
    where
        E::Args: Send,
    {
        BoxedSendEventUpdate {
            event_type: TypeId::of::<E>(),
            slot: self.slot,
            event_update_args: Box::new(self),
            delivery_list_deref: delivery_list_deref::<E>,
            debug_fmt: debug_fmt::<E>,
        }
    }

    /// Change the event type if the event args type is the same
    pub(crate) fn transmute_event<E2: Event<Args = E::Args>>(&self) -> &EventUpdate<E2> {
        // SAFETY: this is a change on the type system only, the data layout is the same.
        unsafe { mem::transmute(self) }
    }
}
impl<E: Event> crate::private::Sealed for EventUpdate<E> {}
impl<E: Event> fmt::Debug for EventUpdate<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "EventUpdate<{}>", type_name::<E>())?;
        f.debug_struct("")
            .field("args", &self.args)
            .field("delivery_list", &self.delivery_list)
            .finish_non_exhaustive()
    }
}
impl<E: Event> Deref for EventUpdate<E> {
    type Target = E::Args;

    fn deref(&self) -> &Self::Target {
        &self.args
    }
}

/// Construct with [`debug_fmt`].
type DebugFmtFn = unsafe fn(&dyn UnsafeAny, &mut fmt::Formatter) -> fmt::Result;
unsafe fn debug_fmt<E: Event>(args: &dyn UnsafeAny, f: &mut fmt::Formatter) -> fmt::Result {
    let args = args.downcast_ref_unchecked::<EventUpdate<E>>();
    fmt::Debug::fmt(args, f)
}

/// Construct with [`delivery_list_deref`].
type DeliveryListDerefFn = unsafe fn(&dyn UnsafeAny) -> &EventDeliveryList;
unsafe fn delivery_list_deref<E: Event>(args: &dyn UnsafeAny) -> &EventDeliveryList {
    let args = args.downcast_ref_unchecked::<EventUpdate<E>>();
    &args.delivery_list
}

#[derive(Debug)]
struct WindowDelivery {
    id: WindowId,
    widgets: Vec<WidgetPath>,
    all: bool,
}

/// Delivery list for an [`EventArgs`].
///
/// Windows and widgets use this list to find all targets of the event.
#[derive(Debug)]
pub struct EventDeliveryList {
    windows: RefCell<Vec<WindowDelivery>>,
    all: bool,
    window: Cell<usize>,
    depth: Cell<usize>,

    search: RefCell<Vec<WidgetId>>,
}
impl Default for EventDeliveryList {
    /// None.
    fn default() -> Self {
        Self::none()
    }
}
impl EventDeliveryList {
    /// Target no widgets or windows.
    ///
    /// Only app extensions receive the event.
    pub fn none() -> Self {
        Self {
            windows: RefCell::new(vec![]),
            all: false,
            window: Cell::new(0),
            depth: Cell::new(0),

            search: RefCell::new(vec![]),
        }
    }

    /// Target all widgets and windows.
    ///
    /// The event is broadcast to everyone.
    pub fn all() -> Self {
        let mut s = Self::none();
        s.all = true;
        s
    }

    /// All widgets inside the window.
    pub fn window(window_id: WindowId) -> Self {
        Self::none().with_window(window_id)
    }

    /// All widgets inside the window.
    pub fn window_opt(window_id: Option<WindowId>) -> Self {
        Self::none().with_window_opt(window_id)
    }

    /// All widgets in the path.
    pub fn widgets(widget_path: &WidgetPath) -> Self {
        Self::none().with_widgets(widget_path)
    }

    /// All widgets in each path on the list.
    pub fn widgets_list<'a>(list: impl IntoIterator<Item = &'a WidgetPath>) -> Self {
        Self::none().with_widgets_list(list)
    }

    /// All widgets in the path.
    pub fn widgets_opt(widget_path: Option<&WidgetPath>) -> Self {
        Self::none().with_widgets_opt(widget_path)
    }

    /// A widget ID to be searched before send.
    ///
    /// The windows info trees are searched before the event is send for delivery.
    pub fn find_widget(widget_id: WidgetId) -> Self {
        Self::none().with_find_widget(widget_id)
    }

    /// Add all widgets inside the window for delivery.
    ///
    /// The event is broadcast inside the window.
    pub fn with_window(mut self, window_id: WindowId) -> Self {
        if self.all {
            return self;
        }

        if let Some(w) = self.windows.get_mut().iter_mut().find(|w| w.id == window_id) {
            w.widgets.clear();
            w.all = true;
        } else {
            self.windows.get_mut().push(WindowDelivery {
                id: window_id,
                widgets: vec![],
                all: true,
            });
        }
        self
    }

    /// All the widgets in the window if it is some.
    pub fn with_window_opt(self, window_id: Option<WindowId>) -> Self {
        if let Some(window_id) = window_id {
            self.with_window(window_id)
        } else {
            self
        }
    }

    /// Add the widgets in the path to the delivery.
    pub fn with_widgets(mut self, widget_path: &WidgetPath) -> Self {
        if self.all {
            return self;
        }

        if let Some(w) = self.windows.get_mut().iter_mut().find(|w| w.id == widget_path.window_id()) {
            if !w.all {
                w.widgets.push(widget_path.clone());
            }
        } else {
            self.windows.get_mut().push(WindowDelivery {
                id: widget_path.window_id(),
                widgets: vec![widget_path.clone()],
                all: false,
            })
        }
        self
    }

    /// All the widgets in each path on the list.
    pub fn with_widgets_list<'a>(mut self, list: impl IntoIterator<Item = &'a WidgetPath>) -> Self {
        for path in list {
            self = self.with_widgets(path);
        }
        self
    }

    /// Add the widgets in the path if it is some.
    pub fn with_widgets_opt(self, widget_path: Option<&WidgetPath>) -> Self {
        if let Some(path) = widget_path {
            self.with_widgets(path)
        } else {
            self
        }
    }

    /// A widget ID to be searched before send.
    ///
    /// The windows info trees are searched before the event is send for delivery.
    pub fn with_find_widget(mut self, widget_id: WidgetId) -> Self {
        self.search.get_mut().push(widget_id);
        self
    }

    /// Returns `true` if the event has target in the window.
    pub fn enter_window(&self, ctx: &mut WindowContext) -> bool {
        if self.all {
            return true;
        }

        self.find_widgets(ctx);

        let window_id = *ctx.window_id;

        if let Some(i) = self.windows.borrow().iter().position(|w| w.id == window_id) {
            self.window.set(i);
            self.depth.set(0);
            true
        } else {
            false
        }
    }

    /// Returns `true` if the event has targets in the widget or targets the widget.
    pub fn enter_widget(&self, widget_id: WidgetId) -> bool {
        if self.all {
            self.depth.set(self.depth.get() + 1);
            return true;
        }

        let windows = self.windows.borrow();

        if windows.is_empty() {
            self.depth.set(self.depth.get() + 1);
            return false;
        }

        let window = &windows[self.window.get()];
        if window.all {
            self.depth.set(self.depth.get() + 1);
            true
        } else {
            for path in &window.widgets {
                let path = path.widgets_path();
                if path.len() > self.depth.get() && path[self.depth.get()] == widget_id {
                    self.depth.set(self.depth.get() + 1);
                    return true;
                }
            }
            false
        }
    }

    /// Must be called if [`enter_widget`] returned `true`.
    ///
    /// [`enter_widget`]: Self::enter_widget
    pub fn exit_widget(&self) {
        self.depth.set(self.depth.get() - 1);
    }

    /// Must be called if [`exit_window`] returned `true`.
    ///
    /// [`exit_window`]: Self::exit_window
    pub fn exit_window(&self) {
        self.depth.set(0);
    }

    /// Resolve `find_widget` pending queries.
    fn find_widgets(&self, ctx: &mut WindowContext) {
        let search = mem::take(&mut *self.search.borrow_mut());

        if self.all || search.is_empty() {
            return;
        }

        if let Some(windows) = ctx.services.get::<crate::window::Windows>() {
            let mut self_windows = self.windows.borrow_mut();
            'search: for wgt in search {
                for win in windows.widget_trees() {
                    if let Some(info) = win.get(wgt) {
                        if let Some(w) = self_windows.iter_mut().find(|w| w.id == win.window_id()) {
                            if !w.all {
                                w.widgets.push(info.path());
                            }
                        } else {
                            self_windows.push(WindowDelivery {
                                id: win.window_id(),
                                widgets: vec![info.path()],
                                all: false,
                            });
                        }

                        continue 'search;
                    }
                }
            }
        }
    }
}

/// Boxed [`EventUpdateArgs`].
pub struct BoxedEventUpdate {
    event_type: TypeId,
    slot: EventSlot,
    event_update_args: Box<dyn UnsafeAny>,
    delivery_list_deref: DeliveryListDerefFn,
    debug_fmt: DebugFmtFn,
}
impl BoxedEventUpdate {
    /// Unbox the arguments for `Q` if the update is for `Q`.
    pub fn unbox_for<Q: Event>(self) -> Result<EventUpdate<Q>, Self> {
        if self.event_type == TypeId::of::<Q>() {
            Ok(unsafe {
                // SAFETY: its the same type
                *self.event_update_args.downcast_unchecked()
            })
        } else {
            Err(self)
        }
    }

    fn delivery_list(&self) -> &EventDeliveryList {
        unsafe {
            // SAFETY: only `EventUpdate<E>` can build and it is strongly typed.
            (self.delivery_list_deref)(&*self.event_update_args)
        }
    }
}
impl crate::private::Sealed for BoxedEventUpdate {}
impl fmt::Debug for BoxedEventUpdate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "boxed ")?;
        // SAFETY: only `EventUpdate<E>` can build and it is strongly typed.
        unsafe { (self.debug_fmt)(&*self.event_update_args, f) }
    }
}
impl EventUpdateArgs for BoxedEventUpdate {
    fn args_for<Q: Event>(&self) -> Option<&EventUpdate<Q>> {
        if self.event_type == TypeId::of::<Q>() {
            Some(unsafe {
                // SAFETY: its the same type
                self.event_update_args.downcast_ref_unchecked()
            })
        } else {
            None
        }
    }

    fn as_any(&self) -> AnyEventUpdate {
        AnyEventUpdate {
            event_type_id: self.event_type,
            delivery_list_deref: self.delivery_list_deref,
            debug_fmt: self.debug_fmt,
            slot: self.slot,
            event_update_args: &*self.event_update_args,
        }
    }

    fn slot(&self) -> EventSlot {
        self.slot
    }

    fn with_window<H: FnOnce(&mut WindowContext) -> R, R>(&self, ctx: &mut WindowContext, handle: H) -> Option<R> {
        if self.delivery_list().enter_window(ctx) {
            let r = Some(handle(ctx));
            self.delivery_list().exit_window();
            r
        } else {
            None
        }
    }

    fn with_widget<H: FnOnce(&mut WidgetContext) -> R, R>(&self, ctx: &mut WidgetContext, handle: H) -> Option<R> {
        if self.delivery_list().enter_widget(ctx.path.widget_id()) {
            let r = Some(handle(ctx));
            self.delivery_list().exit_widget();
            r
        } else {
            None
        }
    }
}

/// A [`BoxedEventUpdate`] that is [`Send`].
pub struct BoxedSendEventUpdate {
    event_type: TypeId,
    slot: EventSlot,
    event_update_args: Box<dyn UnsafeAny + Send>,
    delivery_list_deref: DeliveryListDerefFn,
    debug_fmt: DebugFmtFn,
}
#[cfg(debug_assertions)]
fn _assert_is_send(args: BoxedSendEventUpdate) -> impl Send {
    args
}
impl BoxedSendEventUpdate {
    /// Unbox the arguments for `Q` if the update is for `Q`.
    pub fn unbox_for<Q: Event>(self) -> Result<EventUpdate<Q>, Self>
    where
        Q::Args: Send,
    {
        if self.event_type == TypeId::of::<Q>() {
            Ok(unsafe {
                // SAFETY: its the same type
                *<dyn UnsafeAny>::downcast_unchecked(self.event_update_args)
            })
        } else {
            Err(self)
        }
    }

    /// Convert to [`BoxedEventUpdate`].
    pub fn forget_send(self) -> BoxedEventUpdate {
        BoxedEventUpdate {
            event_type: self.event_type,
            slot: self.slot,
            event_update_args: self.event_update_args,
            delivery_list_deref: self.delivery_list_deref,
            debug_fmt: self.debug_fmt,
        }
    }
}
impl crate::private::Sealed for BoxedSendEventUpdate {}
impl fmt::Debug for BoxedSendEventUpdate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "boxed send ")?;
        // SAFETY: only `EventUpdate<E>` can build and it is strongly typed.
        unsafe { (self.debug_fmt)(&*self.event_update_args, f) }
    }
}

/// Type erased [`EventUpdateArgs`].
pub struct AnyEventUpdate<'a> {
    event_type_id: TypeId,
    slot: EventSlot,
    // this is a reference to a `EventUpdate<Q>`.
    event_update_args: &'a dyn UnsafeAny,
    delivery_list_deref: DeliveryListDerefFn,
    debug_fmt: DebugFmtFn,
}
impl<'a> AnyEventUpdate<'a> {
    /// Gets the [`TypeId`] of the event type represented by `self`.
    pub fn event_type_id(&self) -> TypeId {
        self.event_type_id
    }

    fn delivery_list(&self) -> &EventDeliveryList {
        unsafe {
            // SAFETY: only `EventUpdate<E>` can build and it is strongly typed.
            (self.delivery_list_deref)(&*self.event_update_args)
        }
    }
}
impl<'a> fmt::Debug for AnyEventUpdate<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "any ")?;
        // SAFETY: only `EventUpdate<E>` can build and it is strongly typed.
        unsafe { (self.debug_fmt)(self.event_update_args, f) }
    }
}
impl<'a> crate::private::Sealed for AnyEventUpdate<'a> {}
impl<'a> EventUpdateArgs for AnyEventUpdate<'a> {
    fn args_for<Q: Event>(&self) -> Option<&EventUpdate<Q>> {
        if self.event_type_id == TypeId::of::<Q>() {
            Some(unsafe {
                // SAFETY: its the same type.
                #[allow(clippy::transmute_ptr_to_ptr)]
                self.event_update_args.downcast_ref_unchecked()
            })
        } else {
            None
        }
    }

    fn as_any(&self) -> AnyEventUpdate {
        AnyEventUpdate {
            event_type_id: self.event_type_id,
            slot: self.slot,
            event_update_args: self.event_update_args,
            delivery_list_deref: self.delivery_list_deref,
            debug_fmt: self.debug_fmt,
        }
    }

    fn slot(&self) -> EventSlot {
        self.slot
    }

    fn with_window<H: FnOnce(&mut WindowContext) -> R, R>(&self, ctx: &mut WindowContext, handle: H) -> Option<R> {
        if self.delivery_list().enter_window(ctx) {
            let r = Some(handle(ctx));
            self.delivery_list().exit_window();
            r
        } else {
            None
        }
    }

    fn with_widget<H: FnOnce(&mut WidgetContext) -> R, R>(&self, ctx: &mut WidgetContext, handle: H) -> Option<R> {
        if self.delivery_list().enter_widget(ctx.path.widget_id()) {
            let r = Some(handle(ctx));
            self.delivery_list().exit_widget();
            r
        } else {
            None
        }
    }
}

/// Represents an event update.
pub trait EventUpdateArgs: fmt::Debug + crate::private::Sealed {
    /// Gets the the update arguments if the event updating is `Q`.
    fn args_for<Q: Event>(&self) -> Option<&EventUpdate<Q>>;

    /// Type erased event update.
    fn as_any(&self) -> AnyEventUpdate;

    /// Returns the [`EventSlot`] that represents the event type.
    fn slot(&self) -> EventSlot;

    /// Calls `handle` if the event targets the window.
    ///
    /// Good window implementations should check if the window event subscriptions includes the event slot too before calling this.
    fn with_window<H: FnOnce(&mut WindowContext) -> R, R>(&self, ctx: &mut WindowContext, handle: H) -> Option<R>;

    /// Calls `handle` if the event targets the widget.
    ///
    /// Good widget implementations should check if the widget event subscriptions includes the event slot too before calling this.
    fn with_widget<H: FnOnce(&mut WidgetContext) -> R, R>(&self, ctx: &mut WidgetContext, handle: H) -> Option<R>;
}
impl<E: Event> EventUpdateArgs for EventUpdate<E> {
    fn args_for<Q: Event>(&self) -> Option<&EventUpdate<Q>> {
        if TypeId::of::<E>() == TypeId::of::<Q>() {
            Some(unsafe {
                // SAFETY: its the same type.
                #[allow(clippy::transmute_ptr_to_ptr)]
                std::mem::transmute(self)
            })
        } else {
            None
        }
    }

    fn as_any(&self) -> AnyEventUpdate {
        AnyEventUpdate {
            event_type_id: TypeId::of::<E>(),
            debug_fmt: debug_fmt::<E>,
            delivery_list_deref: delivery_list_deref::<E>,
            slot: self.slot,
            event_update_args: self,
        }
    }

    fn slot(&self) -> EventSlot {
        self.slot
    }

    fn with_window<H: FnOnce(&mut WindowContext) -> R, R>(&self, ctx: &mut WindowContext, handle: H) -> Option<R> {
        if self.delivery_list().enter_window(ctx) {
            let r = Some(handle(ctx));
            self.delivery_list().exit_window();
            r
        } else {
            None
        }
    }

    fn with_widget<H: FnOnce(&mut WidgetContext) -> R, R>(&self, ctx: &mut WidgetContext, handle: H) -> Option<R> {
        if self.delivery_list().enter_widget(ctx.path.widget_id()) {
            let r = Some(handle(ctx));
            self.delivery_list().exit_widget();
            r
        } else {
            None
        }
    }
}

/// A buffered event listener.
///
/// This `struct` is a refence to the buffer, clones of it point to the same buffer. This `struct`
/// is not `Send`, you can use an [`Events::receiver`] for that.
#[derive(Clone)]
pub struct EventBuffer<E: Event> {
    queue: Rc<RefCell<VecDeque<E::Args>>>,
}
impl<E: Event> EventBuffer<E> {
    /// If there are any updates in the buffer.
    pub fn has_updates(&self) -> bool {
        !RefCell::borrow(&self.queue).is_empty()
    }

    /// Take the oldest event in the buffer.
    pub fn pop_oldest(&self) -> Option<E::Args> {
        self.queue.borrow_mut().pop_front()
    }

    /// Take the oldest `n` events from the buffer.
    ///
    /// The result is sorted from oldest to newer.
    pub fn pop_oldest_n(&self, n: usize) -> Vec<E::Args> {
        self.queue.borrow_mut().drain(..n).collect()
    }

    /// Take all the events from the buffer.
    ///
    /// The result is sorted from oldest to newest.
    pub fn pop_all(&self) -> Vec<E::Args> {
        self.queue.borrow_mut().drain(..).collect()
    }

    /// Create an empty buffer that will always stay empty.
    pub fn never() -> Self {
        EventBuffer { queue: Default::default() }
    }
}

/// An event update sender that can be used from any thread and without access to [`Events`].
///
/// Use [`Events::sender`] to create a sender.
pub struct EventSender<E>
where
    E: Event,
    E::Args: Send,
{
    sender: AppEventSender,
    slot: EventSlot,
    _event: PhantomData<E>,
}
impl<E> Clone for EventSender<E>
where
    E: Event,
    E::Args: Send,
{
    fn clone(&self) -> Self {
        EventSender {
            sender: self.sender.clone(),
            slot: self.slot,
            _event: PhantomData,
        }
    }
}
impl<E> fmt::Debug for EventSender<E>
where
    E: Event,
    E::Args: Send,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "EventSender<{}>", type_name::<E>())
    }
}
impl<E> EventSender<E>
where
    E: Event,
    E::Args: Send,
{
    /// Send an event update.
    pub fn send(&self, args: E::Args) -> Result<(), AppDisconnected<E::Args>> {
        UpdatesTrace::log_event::<E>();

        let update = EventUpdate::<E> {
            delivery_list: args.delivery_list(),
            args,
            slot: self.slot,
        }
        .boxed_send();
        self.sender.send_event(update).map_err(|e| {
            if let Ok(e) = e.0.unbox_for::<E>() {
                AppDisconnected(e.args)
            } else {
                unreachable!()
            }
        })
    }
}

/// An event update receiver that can be used from any thread and without access to [`Events`].
///
/// Use [`Events::receiver`] to create a receiver, drop to stop listening.
pub struct EventReceiver<E>
where
    E: Event,
    E::Args: Send,
{
    receiver: flume::Receiver<E::Args>,
}
impl<E> Clone for EventReceiver<E>
where
    E: Event,
    E::Args: Send,
{
    fn clone(&self) -> Self {
        EventReceiver {
            receiver: self.receiver.clone(),
        }
    }
}
impl<E> Debug for EventReceiver<E>
where
    E: Event,
    E::Args: Send,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "EventReceiver<{}>", type_name::<E>())
    }
}
impl<E> EventReceiver<E>
where
    E: Event,
    E::Args: Send,
{
    /// Receives the oldest send update, blocks until the event updates.
    pub fn recv(&self) -> Result<E::Args, AppDisconnected<()>> {
        self.receiver.recv().map_err(|_| AppDisconnected(()))
    }

    /// Tries to receive the oldest sent update not received, returns `Ok(args)` if there was at least
    /// one update, or returns `Err(None)` if there was no update or returns `Err(AppDisconnected)` if the connected
    /// app has exited.
    pub fn try_recv(&self) -> Result<E::Args, Option<AppDisconnected<()>>> {
        self.receiver.try_recv().map_err(|e| match e {
            flume::TryRecvError::Empty => None,
            flume::TryRecvError::Disconnected => Some(AppDisconnected(())),
        })
    }

    /// Receives the oldest send update, blocks until the event updates or until the `deadline` is reached.
    pub fn recv_deadline(&self, deadline: Instant) -> Result<E::Args, TimeoutOrAppDisconnected> {
        self.receiver.recv_deadline(deadline).map_err(TimeoutOrAppDisconnected::from)
    }

    /// Receives the oldest send update, blocks until the event updates or until timeout.
    pub fn recv_timeout(&self, dur: Duration) -> Result<E::Args, TimeoutOrAppDisconnected> {
        self.receiver.recv_timeout(dur).map_err(TimeoutOrAppDisconnected::from)
    }

    /// Returns a future that receives the oldest send update, awaits until an event update occurs.
    pub fn recv_async(&self) -> RecvFut<E::Args> {
        self.receiver.recv_async().into()
    }

    /// Turns into a future that receives the oldest send update, awaits until an event update occurs.
    pub fn into_recv_async(self) -> RecvFut<'static, E::Args> {
        self.receiver.into_recv_async().into()
    }

    /// Creates a blocking iterator over event updates, if there are no updates sent the iterator blocks,
    /// the iterator only finishes when the app shuts-down.
    pub fn iter(&self) -> flume::Iter<E::Args> {
        self.receiver.iter()
    }

    /// Create a non-blocking iterator over event updates, the iterator finishes if
    /// there are no more updates sent.
    pub fn try_iter(&self) -> flume::TryIter<E::Args> {
        self.receiver.try_iter()
    }
}
impl<E> From<EventReceiver<E>> for flume::Receiver<E::Args>
where
    E: Event,
    E::Args: Send,
{
    fn from(e: EventReceiver<E>) -> Self {
        e.receiver
    }
}
impl<'a, E> IntoIterator for &'a EventReceiver<E>
where
    E: Event,
    E::Args: Send,
{
    type Item = E::Args;

    type IntoIter = flume::Iter<'a, E::Args>;

    fn into_iter(self) -> Self::IntoIter {
        self.receiver.iter()
    }
}
impl<E> IntoIterator for EventReceiver<E>
where
    E: Event,
    E::Args: Send,
{
    type Item = E::Args;

    type IntoIter = flume::IntoIter<E::Args>;

    fn into_iter(self) -> Self::IntoIter {
        self.receiver.into_iter()
    }
}

struct OnEventHandler {
    handle: HandleOwner<()>,
    handler: Box<dyn FnMut(&mut AppContext, &BoxedEventUpdate, &dyn AppWeakHandle)>,
}

/// Represents an app context event handler created by [`Events::on_event`] or [`Events::on_pre_event`].
///
/// Drop all clones of this handle to drop the handler, or call [`unsubscribe`](Self::unsubscribe) to drop the handle
/// without dropping the handler.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
#[repr(transparent)]
#[must_use = "the event handler unsubscribes if the handle is dropped"]
pub struct OnEventHandle(Handle<()>);
impl OnEventHandle {
    fn new() -> (HandleOwner<()>, OnEventHandle) {
        let (owner, handle) = Handle::new(());
        (owner, OnEventHandle(handle))
    }

    /// Create a handle to nothing, the handle always in the *unsubscribed* state.
    ///
    /// Note that `Option<OnEventHandle>` takes up the same space as `OnEventHandle` and avoids an allocation.
    pub fn dummy() -> Self {
        assert_non_null!(OnEventHandle);
        OnEventHandle(Handle::dummy(()))
    }

    /// Drop the handle but does **not** unsubscribe.
    ///
    /// The handler stays in memory for the duration of the app or until another handle calls [`unsubscribe`](Self::unsubscribe.)
    pub fn perm(self) {
        self.0.perm();
    }

    /// If another handle has called [`perm`](Self::perm).
    /// If `true` the var binding will stay active until the app exits, unless [`unsubscribe`](Self::unsubscribe) is called.
    pub fn is_permanent(&self) -> bool {
        self.0.is_permanent()
    }

    /// Drops the handle and forces the handler to drop.
    pub fn unsubscribe(self) {
        self.0.force_drop()
    }

    /// If another handle has called [`unsubscribe`](Self::unsubscribe).
    ///
    /// The handler is already dropped or will be dropped in the next app update, this is irreversible.
    pub fn is_unsubscribed(&self) -> bool {
        self.0.is_dropped()
    }

    /// Create a weak handle.
    pub fn downgrade(&self) -> WeakOnEventHandle {
        WeakOnEventHandle(self.0.downgrade())
    }
}

/// Weak [`OnEventHandle`].
#[derive(Clone, PartialEq, Eq, Hash, Default, Debug)]
pub struct WeakOnEventHandle(WeakHandle<()>);
impl WeakOnEventHandle {
    /// New weak handle that does not upgrade.
    pub fn new() -> Self {
        Self(WeakHandle::new())
    }

    /// Gets the strong handle if it is still subscribed.
    pub fn upgrade(&self) -> Option<OnEventHandle> {
        self.0.upgrade().map(OnEventHandle)
    }
}

thread_singleton!(SingletonEvents);

type BufferEntry = Box<dyn Fn(&BoxedEventUpdate) -> Retain>;

/// Access to application events.
///
/// An instance of this struct is available in [`AppContext`] and derived contexts.
pub struct Events {
    app_event_sender: AppEventSender,

    updates: Vec<BoxedEventUpdate>,

    pre_buffers: Vec<BufferEntry>,
    buffers: Vec<BufferEntry>,
    pre_handlers: Vec<OnEventHandler>,
    pos_handlers: Vec<OnEventHandler>,

    commands: Vec<AnyCommand>,

    _singleton: SingletonEvents,
}
impl Events {
    /// If an instance of `Events` already exists in the  current thread.
    pub(crate) fn instantiated() -> bool {
        SingletonEvents::in_use()
    }

    /// Produces the instance of `Events`. Only a single
    /// instance can exist in a thread at a time, panics if called
    /// again before dropping the previous instance.
    pub(crate) fn instance(app_event_sender: AppEventSender) -> Self {
        Events {
            app_event_sender,
            updates: vec![],
            pre_buffers: vec![],
            buffers: vec![],
            pre_handlers: vec![],
            pos_handlers: vec![],
            commands: vec![],
            _singleton: SingletonEvents::assert_new("Events"),
        }
    }

    /// Called by [`Event::notify`] to schedule a notification.
    pub fn notify<E: Event>(&mut self, event: E, args: E::Args) {
        UpdatesTrace::log_event::<E>();
        let update = EventUpdate::<E>::new(event, args);
        self.updates.push(update.boxed());
    }

    pub(crate) fn notify_app_event(&mut self, update: BoxedSendEventUpdate) {
        self.updates.push(update.forget_send());
    }

    pub(crate) fn register_command(&mut self, command: AnyCommand) {
        if self
            .commands
            .iter()
            .any(|c| c.command_type_id() == command.command_type_id() && c.scope() == command.scope())
        {
            panic!("command `{command:?}` is already registered")
        }
        self.commands.push(command);
    }

    /// Creates an event buffer that listens to `E`. The event updates are pushed as soon as possible, before
    /// the UI and [`on_event`](Self::on_event) are notified.
    ///
    /// Drop the buffer to stop listening.
    pub fn pre_buffer<E: Event>(&mut self, event: E) -> EventBuffer<E> {
        Self::push_buffer::<E>(&mut self.pre_buffers, event)
    }

    /// Creates an event buffer that listens to `E`. The event updates are pushed only after
    /// the UI and [`on_event`](Self::on_event) are notified.
    ///
    /// Drop the buffer to stop listening.
    pub fn buffer<E: Event>(&mut self, event: E) -> EventBuffer<E> {
        Self::push_buffer::<E>(&mut self.buffers, event)
    }

    fn push_buffer<E: Event>(buffers: &mut Vec<BufferEntry>, event: E) -> EventBuffer<E> {
        let buf = EventBuffer::never();
        let weak = Rc::downgrade(&buf.queue);
        buffers.push(Box::new(move |args| {
            let mut retain = false;
            if let Some(rc) = weak.upgrade() {
                if let Some(args) = event.update(args) {
                    rc.borrow_mut().push_back(args.clone());
                }
                retain = true;
            }
            retain
        }));
        buf
    }

    /// Creates a sender that can raise an event from other threads and without access to [`Events`].
    pub fn sender<E>(&mut self, event: E) -> EventSender<E>
    where
        E: Event,
        E::Args: Send,
    {
        EventSender {
            sender: self.app_event_sender.clone(),
            slot: event.slot(),
            _event: PhantomData,
        }
    }

    /// Creates a channel that can listen to event from another thread. The event updates are sent as soon as possible, before
    /// the UI and [`on_event`](Self::on_event) are notified.
    ///
    /// Drop the receiver to stop listening.
    pub fn pre_receiver<E>(&mut self, event: E) -> EventReceiver<E>
    where
        E: Event,
        E::Args: Send,
    {
        Self::push_receiver::<E>(&mut self.pre_buffers, event)
    }

    /// Creates a channel that can listen to event from another thread. The event updates are sent only after the
    /// UI and [`on_event`](Self::on_event) are notified.
    ///
    /// Drop the receiver to stop listening.
    pub fn receiver<E>(&mut self, event: E) -> EventReceiver<E>
    where
        E: Event,
        E::Args: Send,
    {
        Self::push_receiver::<E>(&mut self.buffers, event)
    }

    fn push_receiver<E>(buffers: &mut Vec<BufferEntry>, event: E) -> EventReceiver<E>
    where
        E: Event,
        E::Args: Send,
    {
        let (sender, receiver) = flume::unbounded();

        buffers.push(Box::new(move |e| {
            let mut retain = true;
            if let Some(args) = event.update(e) {
                retain = sender.send(args.clone()).is_ok();
            }
            retain
        }));

        EventReceiver { receiver }
    }

    /// Creates a preview event handler.
    ///
    /// The event `handler` is called for every update of `E` that has not stopped [`propagation`](EventArgs::propagation).
    /// The handler is called before UI handlers and [`on_event`](Self::on_event) handlers, it is called after all previous registered
    /// preview handlers.
    ///
    /// Returns a [`OnEventHandle`] that can be used to unsubscribe, you can also unsubscribe from inside the handler by calling
    /// [`unsubscribe`](crate::handler::AppWeakHandle::unsubscribe) in the third parameter of [`app_hn!`] or [`async_app_hn!`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use zero_ui_core::event::*;
    /// # use zero_ui_core::handler::app_hn;
    /// # use zero_ui_core::focus::{FocusChangedEvent, FocusChangedArgs};
    /// # fn example(ctx: &mut zero_ui_core::context::AppContext) {
    /// let handle = ctx.events.on_pre_event(FocusChangedEvent, app_hn!(|_ctx, args: &FocusChangedArgs, _| {
    ///     println!("focused: {:?}", args.new_focus);
    /// }));
    /// # }
    /// ```
    /// The example listens to all `FocusChangedEvent` events, independent of widget context and before all UI handlers.
    ///
    /// # Handlers
    ///
    /// the event handler can be any type that implements [`AppHandler`], there are multiple flavors of handlers, including
    /// async handlers that allow calling `.await`. The handler closures can be declared using [`app_hn!`], [`async_app_hn!`],
    /// [`app_hn_once!`] and [`async_app_hn_once!`].
    ///
    /// ## Async
    ///
    /// Note that for async handlers only the code before the first `.await` is called in the *preview* moment, code after runs in
    /// subsequent event updates, after the event has already propagated, so stopping [`propagation`](EventArgs::propagation)
    /// only causes the desired effect before the first `.await`.
    ///
    /// [`app_hn!`]: crate::handler::app_hn!
    /// [`async_app_hn!`]: crate::handler::async_app_hn!
    /// [`app_hn_once!`]: crate::handler::app_hn_once!
    /// [`async_app_hn_once!`]: crate::handler::async_app_hn_once!
    pub fn on_pre_event<E, H>(&mut self, event: E, handler: H) -> OnEventHandle
    where
        E: Event,
        H: AppHandler<E::Args>,
    {
        Self::push_event_handler(&mut self.pre_handlers, event, true, handler)
    }

    /// Creates an event handler.
    ///
    /// The event `handler` is called for every update of `E` that has not stopped [`propagation`](EventArgs::propagation).
    /// The handler is called after all [`on_pre_event`],(Self::on_pre_event) all UI handlers and all [`on_event`](Self::on_event) handlers
    /// registered before this one.
    ///
    /// Returns a [`OnEventHandle`] that can be used to unsubscribe, you can also unsubscribe from inside the handler by calling
    /// [`unsubscribe`](crate::handler::AppWeakHandle::unsubscribe) in the third parameter of [`app_hn!`] or [`async_app_hn!`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use zero_ui_core::event::*;
    /// # use zero_ui_core::handler::app_hn;
    /// # use zero_ui_core::focus::{FocusChangedEvent, FocusChangedArgs};
    /// # fn example(ctx: &mut zero_ui_core::context::AppContext) {
    /// let handle = ctx.events.on_event(FocusChangedEvent, app_hn!(|_ctx, args: &FocusChangedArgs, _| {
    ///     println!("focused: {:?}", args.new_focus);
    /// }));
    /// # }
    /// ```
    /// The example listens to all `FocusChangedEvent` events, independent of widget context, after the UI was notified.
    ///
    /// # Handlers
    ///
    /// the event handler can be any type that implements [`AppHandler`], there are multiple flavors of handlers, including
    /// async handlers that allow calling `.await`. The handler closures can be declared using [`app_hn!`], [`async_app_hn!`],
    /// [`app_hn_once!`] and [`async_app_hn_once!`].
    ///
    /// ## Async
    ///
    /// Note that for async handlers only the code before the first `.await` is called in the *preview* moment, code after runs in
    /// subsequent event updates, after the event has already propagated, so stopping [`propagation`](EventArgs::propagation)
    /// only causes the desired effect before the first `.await`.
    ///
    /// [`app_hn!`]: crate::handler::app_hn!
    /// [`async_app_hn!`]: crate::handler::async_app_hn!
    /// [`app_hn_once!`]: crate::handler::app_hn_once!
    /// [`async_app_hn_once!`]: crate::handler::async_app_hn_once!
    pub fn on_event<E, H>(&mut self, event: E, handler: H) -> OnEventHandle
    where
        E: Event,
        H: AppHandler<E::Args>,
    {
        Self::push_event_handler(&mut self.pos_handlers, event, false, handler)
    }

    fn push_event_handler<E, H>(handlers: &mut Vec<OnEventHandler>, event: E, is_preview: bool, mut handler: H) -> OnEventHandle
    where
        E: Event,
        H: AppHandler<E::Args>,
    {
        let (handle_owner, handle) = OnEventHandle::new();
        let handler = move |ctx: &mut AppContext, args: &BoxedEventUpdate, handle: &dyn AppWeakHandle| {
            if let Some(args) = event.update(args) {
                if !args.propagation().is_stopped() {
                    handler.event(ctx, args, &AppHandlerArgs { handle, is_preview });
                }
            }
        };
        handlers.push(OnEventHandler {
            handle: handle_owner,
            handler: Box::new(handler),
        });
        handle
    }

    pub(crate) fn has_pending_updates(&mut self) -> bool {
        !self.updates.is_empty()
    }

    #[must_use]
    pub(super) fn apply_updates(&mut self, vars: &Vars) -> Vec<BoxedEventUpdate> {
        for command in &self.commands {
            command.update_state(vars);
        }
        self.updates.drain(..).collect()
    }

    pub(super) fn on_pre_events(ctx: &mut AppContext, args: &BoxedEventUpdate) {
        ctx.events.pre_buffers.retain(|buf| buf(args));

        let mut handlers = mem::take(&mut ctx.events.pre_handlers);
        Self::notify_retain(&mut handlers, ctx, args);
        handlers.extend(mem::take(&mut ctx.events.pre_handlers));
        ctx.events.pre_handlers = handlers;
    }

    pub(super) fn on_events(ctx: &mut AppContext, args: &BoxedEventUpdate) {
        let mut handlers = mem::take(&mut ctx.events.pos_handlers);
        Self::notify_retain(&mut handlers, ctx, args);
        handlers.extend(mem::take(&mut ctx.events.pos_handlers));
        ctx.events.pos_handlers = handlers;

        ctx.events.buffers.retain(|buf| buf(args));
    }

    fn notify_retain(handlers: &mut Vec<OnEventHandler>, ctx: &mut AppContext, args: &BoxedEventUpdate) {
        handlers.retain_mut(|e| {
            !e.handle.is_dropped() && {
                (e.handler)(ctx, args, &e.handle.weak_handle());
                !e.handle.is_dropped()
            }
        });
    }

    /// Commands that had handles generated in this app.
    ///
    /// When [`Command::new_handle`] is called for the first time in an app, the command gets registered here.
    ///
    /// [`Command::new_handle`]: crate::command::Command::new_handle
    pub fn commands(&self) -> impl Iterator<Item = AnyCommand> + '_ {
        self.commands.iter().copied()
    }
}
impl Drop for Events {
    fn drop(&mut self) {
        for cmd in &self.commands {
            cmd.on_exit();
        }
    }
}

/// Represents a type that can provide access to [`Events`] inside the window of function call.
///
/// This is used to make event notification less cumbersome to use, it is implemented to all sync and async context types
/// and [`Events`] it-self.
///
/// # Examples
///
/// The example demonstrate how this `trait` simplifies calls to [`Event::notify`].
///
/// ```
/// # use zero_ui_core::{var::*, event::*, context::*};
/// # event_args! { pub struct BarArgs { pub msg: &'static str, .. fn delivery_list(&self) -> EventDeliveryList { EventDeliveryList::all() } } }
/// # event! { pub BarEvent: BarArgs; }
/// # struct Foo { } impl Foo {
/// fn update(&mut self, ctx: &mut WidgetContext) {
///     BarEvent.notify(ctx, BarArgs::now("we are not borrowing `ctx` so can use it directly"));
///
///    // ..
///    let services = &mut ctx.services;
///    BarEvent.notify(ctx, BarArgs::now("we are partially borrowing `ctx` but not `ctx.vars` so we use that"));
/// }
///
/// async fn handler(&mut self, mut ctx: WidgetContextMut) {
///     BarEvent.notify(&mut ctx, BarArgs::now("async contexts can also be used"));
/// }
/// # }
/// ```
pub trait WithEvents {
    /// Calls `action` with the [`Events`] reference.
    fn with_events<R, A: FnOnce(&mut Events) -> R>(&mut self, action: A) -> R;
}
impl WithEvents for Events {
    fn with_events<R, A: FnOnce(&mut Events) -> R>(&mut self, action: A) -> R {
        action(self)
    }
}
impl<'a> WithEvents for crate::context::AppContext<'a> {
    fn with_events<R, A: FnOnce(&mut Events) -> R>(&mut self, action: A) -> R {
        action(self.events)
    }
}
impl<'a> WithEvents for crate::context::WindowContext<'a> {
    fn with_events<R, A: FnOnce(&mut Events) -> R>(&mut self, action: A) -> R {
        action(self.events)
    }
}
impl<'a> WithEvents for crate::context::WidgetContext<'a> {
    fn with_events<R, A: FnOnce(&mut Events) -> R>(&mut self, action: A) -> R {
        action(self.events)
    }
}
impl WithEvents for crate::context::AppContextMut {
    fn with_events<R, A: FnOnce(&mut Events) -> R>(&mut self, action: A) -> R {
        self.with(move |ctx| action(ctx.events))
    }
}
impl WithEvents for crate::context::WidgetContextMut {
    fn with_events<R, A: FnOnce(&mut Events) -> R>(&mut self, action: A) -> R {
        self.with(move |ctx| action(ctx.events))
    }
}
#[cfg(any(test, doc, feature = "test_util"))]
impl WithEvents for crate::context::TestWidgetContext {
    fn with_events<R, A: FnOnce(&mut Events) -> R>(&mut self, action: A) -> R {
        action(&mut self.events)
    }
}
impl WithEvents for crate::app::HeadlessApp {
    fn with_events<R, A: FnOnce(&mut Events) -> R>(&mut self, action: A) -> R {
        action(self.ctx().events)
    }
}

type Retain = bool;

///<span data-del-macro-root></span> Declares new [`EventArgs`] types.
///
/// # Examples
///
/// ```
/// # use zero_ui_core::{event::event_args, WidgetPath, text::{Text, formatx}};
///
/// event_args! {
///     /// My event arguments.
///     pub struct MyEventArgs {
///         /// My argument.
///         pub arg: String,
///         /// My event target.
///         pub target: WidgetPath,
///
///         ..
///
///         /// If `target` starts with the current path.
///         fn delivery_list(&self) -> EventDeliveryList {
///             EventDeliveryList::widgets(&self.target)
///         }
///
///         /// Optional validation, if defined the generated `new` and `now` functions call it and unwrap the result.
///         ///
///         /// The error type can be any type that implement `Debug`.
///         fn validate(&self) -> Result<(), Text> {
///             if self.arg.contains("error") {
///                 return Err(formatx!("invalid arg `{}`", self.arg));
///             }
///             Ok(())
///         }
///     }
///
///     // multiple structs can be declared in the same call.
///     // pub struct MyOtherEventArgs { /**/ }
/// }
/// ```
///
/// [`EventArgs`]: crate::event::EventArgs
#[macro_export]
macro_rules! event_args {
    ($(
        $(#[$outer:meta])*
        $vis:vis struct $Args:ident {
            $($(#[$arg_outer:meta])* $arg_vis:vis $arg:ident : $arg_ty:ty,)*
            ..
            $(#[$delivery_list_outer:meta])*
            fn delivery_list(&$self:ident) -> EventDeliveryList { $($delivery_list:tt)+ }

            $(
                $(#[$validate_outer:meta])*
                fn validate(&$self_v:ident) -> Result<(), $ValidationError:path> { $($validate:tt)+ }
            )?
        }
    )+) => {$(
        $crate::__event_args! {
            $(#[$outer])*
            $vis struct $Args {
                $($(#[$arg_outer])* $arg_vis $arg: $arg_ty,)*

                ..

                $(#[$delivery_list_outer])*
                fn delivery_list(&$self) -> EventDeliveryList { $($delivery_list)+ }

                $(
                    $(#[$validate_outer])*
                    fn validate(&$self_v) -> Result<(), $ValidationError> { $($validate)+ }
                )?
            }
        }
    )+};
}
#[doc(hidden)]
#[macro_export]
macro_rules! __event_args {
    // match validate
    (
        $(#[$outer:meta])*
        $vis:vis struct $Args:ident {
            $($(#[$arg_outer:meta])* $arg_vis:vis $arg:ident : $arg_ty:ty,)*
            ..
            $(#[$delivery_list_outer:meta])*
            fn delivery_list(&$self:ident) -> EventDeliveryList { $($delivery_list:tt)+ }

            $(#[$validate_outer:meta])*
            fn validate(&$self_v:ident) -> Result<(), $ValidationError:path> { $($validate:tt)+ }
        }
    ) => {
        $crate::__event_args! {common=>

            $(#[$outer])*
            $vis struct $Args {
                $($(#[$arg_outer])* $arg_vis $arg: $arg_ty,)*
                ..
                $(#[$delivery_list_outer])*
                fn delivery_list(&$self) -> EventDeliveryList { $($delivery_list)+ }
            }
        }
        impl $Args {
            /// New args from values that convert [into](Into) the argument types.
            ///
            /// # Panics
            ///
            /// Panics if the arguments are invalid.
            #[track_caller]
            #[allow(clippy::too_many_arguments)]
            pub fn new(
                timestamp: impl Into<std::time::Instant>,
                propagation_handle: $crate::event::EventPropagationHandle,
                $($arg : impl Into<$arg_ty>),*
            ) -> Self {
                let args = $Args {
                    timestamp: timestamp.into(),
                    $($arg: $arg.into(),)*
                    propagation_handle,
                };
                args.assert_valid();
                args
            }

            /// New args from values that convert [into](Into) the argument types.
            ///
            /// Returns an error if the constructed arguments are invalid.
            #[allow(clippy::too_many_arguments)]
            pub fn try_new(
                timestamp: impl Into<std::time::Instant>,
                propagation_handle: $crate::event::EventPropagationHandle,
                $($arg : impl Into<$arg_ty>),*
            ) -> Result<Self, $ValidationError> {
                let args = $Args {
                    timestamp: timestamp.into(),
                    $($arg: $arg.into(),)*
                    propagation_handle,
                };
                args.validate()?;
                Ok(args)
            }

            /// Arguments for event that happened now (`Instant::now`).
            ///
            /// # Panics
            ///
            /// Panics if the arguments are invalid.
            #[track_caller]
            #[allow(clippy::too_many_arguments)]
            pub fn now($($arg : impl Into<$arg_ty>),*) -> Self {
                Self::new(std::time::Instant::now(), $crate::event::EventPropagationHandle::new(), $($arg),*)
            }

            /// Arguments for event that happened now (`Instant::now`).
            ///
            /// Returns an error if the constructed arguments are invalid.
            #[allow(clippy::too_many_arguments)]
            pub fn try_now($($arg : impl Into<$arg_ty>),*) -> Result<Self, $ValidationError> {
                Self::try_new(std::time::Instant::now(), $crate::event::EventPropagationHandle::new(), $($arg),*)
            }

            $(#[$validate_outer])*
            pub fn validate(&$self_v) -> Result<(), $ValidationError> {
                $($validate)+
            }

            /// Panics if the arguments are invalid.
            #[track_caller]
            pub fn assert_valid(&self) {
                if let Err(e) = self.validate() {
                    panic!("invalid `{}`, {e:?}", stringify!($Args));
                }
            }
        }
    };

    // match no validate
    (
        $(#[$outer:meta])*
        $vis:vis struct $Args:ident {
            $($(#[$arg_outer:meta])* $arg_vis:vis $arg:ident : $arg_ty:ty,)*
            ..
            $(#[$delivery_list_outer:meta])*
            fn delivery_list(&$self:ident) -> EventDeliveryList { $($delivery_list:tt)+ }
        }
    ) => {
        $crate::__event_args! {common=>

            $(#[$outer])*
            $vis struct $Args {
                $($(#[$arg_outer])* $arg_vis $arg: $arg_ty,)*
                ..
                $(#[$delivery_list_outer])*
                fn delivery_list(&$self) -> EventDeliveryList { $($delivery_list)+  }
            }
        }

        impl $Args {
            /// New args from values that convert [into](Into) the argument types.
            #[allow(clippy::too_many_arguments)]
            pub fn new(
                timestamp: impl Into<std::time::Instant>,
                propagation_handle: $crate::event::EventPropagationHandle,
                $($arg : impl Into<$arg_ty>),*
            ) -> Self {
                $Args {
                    timestamp: timestamp.into(),
                    $($arg: $arg.into(),)*
                    propagation_handle,
                }
            }

            /// Arguments for event that happened now (`Instant::now`).
            #[allow(clippy::too_many_arguments)]
            pub fn now($($arg : impl Into<$arg_ty>),*) -> Self {
                Self::new(std::time::Instant::now(), $crate::event::EventPropagationHandle::new(), $($arg),*)
            }
        }
    };

    // common code between validating and not.
    (common=>

        $(#[$outer:meta])*
        $vis:vis struct $Args:ident {
            $($(#[$arg_outer:meta])* $arg_vis:vis $arg:ident : $arg_ty:ty,)*
            ..
            $(#[$delivery_list_outer:meta])*
            fn delivery_list(&$self:ident) -> EventDeliveryList { $($delivery_list:tt)+ }
        }
    ) => {
        $(#[$outer])*
        #[derive(Debug, Clone)]
        $vis struct $Args {
            /// When the event happened.
            pub timestamp: std::time::Instant,
            $($(#[$arg_outer])* $arg_vis $arg : $arg_ty,)*

            propagation_handle: $crate::event::EventPropagationHandle,
        }
        impl $Args {
            /// Propagation handle associated with this event instance.
            #[allow(unused)]
            pub fn propagation(&self) -> &$crate::event::EventPropagationHandle {
                <Self as $crate::event::EventArgs>::propagation(self)
            }

            $(#[$delivery_list_outer])*
            #[allow(unused)]
            pub fn delivery_list(&self) -> $crate::event::EventDeliveryList {
                <Self as $crate::event::EventArgs>::delivery_list(self)
            }

            /// Calls `handler` and stops propagation if propagation is still allowed.
            ///
            /// Returns the `handler` result if it was called.
            #[allow(unused)]
            pub fn handle<F, R>(&self, handler: F) -> Option<R>
            where
                F: FnOnce(&Self) -> R,
            {
                <Self as $crate::event::EventArgs>::handle(self, handler)
            }
        }
        impl $crate::event::EventArgs for $Args {

            fn timestamp(&self) -> std::time::Instant {
                self.timestamp
            }


            $(#[$delivery_list_outer])*
            fn delivery_list(&$self) -> $crate::event::EventDeliveryList {
                use $crate::event::EventDeliveryList;

                $($delivery_list)+
            }


            fn propagation(&self) -> &$crate::event::EventPropagationHandle {
                &self.propagation_handle
            }
        }
    };
}
#[doc(inline)]
pub use crate::event_args;

///<span data-del-macro-root></span> Declares new [`Event`](crate::event::Event) types.
///
/// # Examples
///
/// ```
/// # use zero_ui_core::event::event;
/// # use zero_ui_core::gesture::ClickArgs;
/// event! {
///     /// Event docs.
///     pub ClickEvent: ClickArgs;
///
///     /// Other event docs.
///     pub DoubleClickEvent: ClickArgs;
/// }
/// ```
#[macro_export]
macro_rules! event {
    ($(
        $(#[$outer:meta])*
        $vis:vis $Event:ident : $Args:path;
    )+) => {$(
        $(#[$outer])*
        #[derive(Clone, Copy, Debug)]
        $vis struct $Event;
        impl $Event {
            /// Gets the event arguments if the update is for this event.
            pub fn update<U: $crate::event::EventUpdateArgs>(self, args: &U) -> Option<&$crate::event::EventUpdate<$Event>> {
                <Self as $crate::event::Event>::update(self, args)
            }

            /// Schedule an event update.
            #[cfg_attr(test, allow(unused))]
            pub fn notify<Evs: $crate::event::WithEvents>(self, events: &mut Evs, args: $Args) {
                <Self as $crate::event::Event>::notify(self, events, args);
            }
        }
        impl $crate::event::Event for $Event {
            type Args = $Args;

            fn slot(self) -> $crate::widget_info::EventSlot {
                std::thread_local! {
                    static SLOT: $crate::widget_info::EventSlot = $crate::widget_info::EventSlot::next();
                }
                SLOT.with(|s| *s)
            }
        }
    )+};
}
#[doc(inline)]
pub use crate::event;

/* Event Property */

#[doc(hidden)]
#[macro_export]
macro_rules! __event_property {
    (
        $(#[$on_event_attrs:meta])*
        $vis:vis fn $event:ident {
            event: $Event:path,
            args: $Args:path,
            filter: $filter:expr,
        }
    ) => { $crate::paste! {
        $(#[$on_event_attrs])*
        ///
        /// # Preview
        ///
        #[doc = "You can preview this event using [`on_pre_"$event "`](fn.on_pre_"$event ".html)."]
        /// Otherwise the handler is only called after the widget content has a chance of handling the event by stopping propagation.
        ///
        /// # Async
        ///
        /// You can use async event handlers with this property.
        #[$crate::property(event, default( $crate::handler::hn!(|_, _|{}) ))]
        $vis fn [<on_ $event>](
            child: impl $crate::UiNode,
            handler: impl $crate::handler::WidgetHandler<$Args>,
        ) -> impl $crate::UiNode {
            $crate::event::on_event(child, $Event, $filter, handler)
        }

        #[doc = "Preview [`on_"$event "`](fn.on_"$event ".html) event."]
        ///
        /// # Preview
        ///
        /// Preview event properties call the handler before the main event property and before the widget content, if you stop
        /// the propagation of a preview event the main event handler is not called.
        ///
        /// # Async
        ///
        /// You can use async event handlers with this property, note that only the code before the fist `.await` is *preview*,
        /// subsequent code runs in widget updates.
        #[$crate::property(event, default( $crate::handler::hn!(|_, _|{}) ))]
        $vis fn [<on_pre_ $event>](
            child: impl $crate::UiNode,
            handler: impl $crate::handler::WidgetHandler<$Args>,
        ) -> impl $crate::UiNode {
            $crate::event::on_pre_event(child, $Event, $filter, handler)
        }
    } };

    (
        $(#[$on_event_attrs:meta])*
        $vis:vis fn $event:ident {
            event: $Event:path,
            args: $Args:path,
        }
    ) => {
        $crate::__event_property! {
            $(#[$on_event_attrs])*
            $vis fn $event {
                event: $Event,
                args: $Args,
                filter: |ctx, args| true,
            }
        }
    };
}
///<span data-del-macro-root></span> Declare one or more event properties.
///
/// Each declaration expands to two properties `on_$event`, `on_pre_$event`.
/// The preview properties call [`on_pre_event`], the main event properties call [`on_event`].
///
/// # Examples
///
/// ```
/// # fn main() { }
/// # use zero_ui_core::event::{event_property, EventArgs};
/// # use zero_ui_core::keyboard::*;
/// event_property! {
///     /// on_key_input docs.
///     pub fn key_input {
///         event: KeyInputEvent,
///         args: KeyInputArgs,
///         // default filter is |ctx, args| true,
///     }
///
///     pub(crate) fn key_down {
///         event: KeyInputEvent,
///         args: KeyInputArgs,
///         // optional filter:
///         filter: |ctx, args| args.state == KeyState::Pressed,
///     }
/// }
/// ```
///
/// # Filter
///
/// App events are delivered to all `UiNode` inside all widgets in the [`EventDeliveryList`], some event properties can
/// also specialize further on top of a more general app event. To implement this you can use a filter predicate.
///
/// The `filter` predicate is called if [`propagation`] is not stopped. It must return `true` if the event arguments
/// are relevant in the context of the widget and event property. If it returns `true` the `handler` closure is called.
/// See [`on_event`] and [`on_pre_event`] for more information.
///
/// If you don't provide a filter predicate the default always allows, so all app events targeting the widget and not already handled
/// are allowed by default.  Note that events that represent an *interaction* with the widget are send for both [`ENABLED`] and [`DISABLED`]
/// targets, event properties should probably distinguish if they fire on normal interactions vs on *disabled* interactions.
///
/// # Async
///
/// Async event handlers are supported by properties generated by this macro, but only the code before the first `.await` executes
/// in the event track, subsequent code runs in widget updates.
///
/// [`on_pre_event`]: crate::event::on_pre_event
/// [`on_event`]: crate::event::on_event
/// [`propagation`]: EventArgs::propagation
/// [`ENABLED`]: crate::widget_info::Interactivity::ENABLED
/// [`DISABLED`]: crate::widget_info::Interactivity::DISABLED
#[macro_export]
macro_rules! event_property {
    ($(
        $(#[$on_event_attrs:meta])*
        $vis:vis fn $event:ident {
            event: $Event:path,
            args: $Args:path $(,
            filter: $filter:expr)? $(,)?
        }
    )+) => {$(
        $crate::__event_property! {
            $(#[$on_event_attrs])*
            $vis fn $event {
                event: $Event,
                args: $Args,
                $(filter: $filter,)?
            }
        }
    )+};
}
#[doc(inline)]
pub use crate::event_property;

/// Helper for declaring event properties.
///
/// This function is used by the [`event_property!`] macro.
///
/// # Filter
///
/// The `filter` predicate is called if [`propagation`] was not stopped. It must return `true` if the event arguments are
/// relevant in the context of the widget. If it returns `true` the `handler` closure is called. Note that events that represent
/// an *interaction* with the widget are send for both [`ENABLED`] and [`DISABLED`] targets, event properties should probably distinguish
/// if they fire on normal interactions vs on *disabled* interactions.
///
/// # Route
///
/// The event `handler` is called after the [`on_pre_event`] equivalent at the same context level. If the event
/// `filter` allows more then one widget and one widget contains the other, the `handler` is called on the inner widget first.
///
/// # Async
///
/// Async event handlers are called like normal, but code after the first `.await` only runs in subsequent updates. This means
/// that [`propagation`] must be stopped before the first `.await`, otherwise you are only signaling
/// other async tasks handling the same event, if they are monitoring the propagation handle.
///
/// [`propagation`]: EventArgs::propagation
/// [`ENABLED`]: crate::widget_info::Interactivity::ENABLED
/// [`DISABLED`]: crate::widget_info::Interactivity::DISABLED
pub fn on_event<C, E, F, H>(child: C, event: E, filter: F, handler: H) -> impl UiNode
where
    C: UiNode,
    E: Event,
    F: FnMut(&mut WidgetContext, &E::Args) -> bool + 'static,
    H: WidgetHandler<E::Args>,
{
    struct OnEventNode<C, E, F, H> {
        child: C,
        event: E,
        filter: F,
        handler: H,
    }
    #[impl_ui_node(child)]
    impl<C, E, F, H> UiNode for OnEventNode<C, E, F, H>
    where
        C: UiNode,
        E: Event,
        F: FnMut(&mut WidgetContext, &E::Args) -> bool + 'static,
        H: WidgetHandler<E::Args>,
    {
        fn info(&self, ctx: &mut InfoContext, widget_info: &mut WidgetInfoBuilder) {
            self.child.info(ctx, widget_info);
        }

        fn subscriptions(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions) {
            subs.event(self.event).handler(&self.handler);
            self.child.subscriptions(ctx, subs);
        }

        fn event<EU: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &EU) {
            if let Some(args) = self.event.update(args) {
                self.child.event(ctx, args);

                if !args.propagation().is_stopped() && (self.filter)(ctx, args) {
                    self.handler.event(ctx, args);
                }
            } else {
                self.child.event(ctx, args);
            }
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            self.child.update(ctx);
            self.handler.update(ctx);
        }
    }

    #[cfg(dyn_closure)]
    let filter: Box<dyn FnMut(&mut WidgetContext, &E::Args) -> bool> = Box::new(filter);

    OnEventNode {
        child: child.cfg_boxed(),
        event,
        filter,
        handler: handler.cfg_boxed(),
    }
    .cfg_boxed()
}

/// Helper for declaring preview event properties.
///
/// This function is used by the [`event_property!`] macro.
///
/// # Filter
///
/// The `filter` predicate is called if [`propagation`] was not stopped. It must return `true` if the event arguments are
/// relevant in the context of the widget. If it returns `true` the `handler` closure is called. Note that events that represent
/// an *interaction* with the widget are send for both [`ENABLED`] and [`DISABLED`] targets, event properties should probably distinguish
/// if they fire on normal interactions vs on *disabled* interactions.
///
/// # Route
///
/// The event `handler` is called before the [`on_event`] equivalent at the same context level. If the event
/// `filter` allows more then one widget and one widget contains the other, the `handler` is called on the inner widget first.
///
/// # Async
///
/// Async event handlers are called like normal, but code after the first `.await` only runs in subsequent event updates. This means
/// that [`propagation`] must be stopped before the first `.await`, otherwise you are only signaling
/// other async tasks handling the same event, if they are monitoring the propagation handle.
///
/// [`propagation`]: EventArgs::propagation
/// [`ENABLED`]: crate::widget_info::Interactivity::ENABLED
/// [`DISABLED`]: crate::widget_info::Interactivity::DISABLED
pub fn on_pre_event<C, E, F, H>(child: C, event: E, filter: F, handler: H) -> impl UiNode
where
    C: UiNode,
    E: Event,
    F: FnMut(&mut WidgetContext, &E::Args) -> bool + 'static,
    H: WidgetHandler<E::Args>,
{
    struct OnPreviewEventNode<C, E, F, H> {
        child: C,
        event: E,
        filter: F,
        handler: H,
    }
    #[impl_ui_node(child)]
    impl<C, E, F, H> UiNode for OnPreviewEventNode<C, E, F, H>
    where
        C: UiNode,
        E: Event,
        F: FnMut(&mut WidgetContext, &E::Args) -> bool + 'static,
        H: WidgetHandler<E::Args>,
    {
        fn info(&self, ctx: &mut InfoContext, widget_info: &mut WidgetInfoBuilder) {
            self.child.info(ctx, widget_info);
        }

        fn subscriptions(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions) {
            subs.event(self.event).handler(&self.handler);
            self.child.subscriptions(ctx, subs);
        }

        fn event<EU: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &EU) {
            if let Some(args) = self.event.update(args) {
                if !args.propagation().is_stopped() && (self.filter)(ctx, args) {
                    self.handler.event(ctx, args);
                }
                self.child.event(ctx, args);
            } else {
                self.child.event(ctx, args);
            }
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            self.handler.update(ctx);
            self.child.update(ctx);
        }
    }

    #[cfg(dyn_closure)]
    let filter: Box<dyn FnMut(&mut WidgetContext, &E::Args) -> bool> = Box::new(filter);

    OnPreviewEventNode {
        child: child.cfg_boxed(),
        event,
        filter,
        handler: handler.cfg_boxed(),
    }
    .cfg_boxed()
}
