//! App startup and app extension API.

use crate::context::*;
use crate::event::{cancelable_event_args, AnyEventUpdate, EventUpdateArgs, Events};
use crate::profiler::*;
use crate::timer::Timers;
use crate::var::{response_var, ResponderVar, ResponseVar};
use crate::{
    focus::FocusManager,
    gesture::GestureManager,
    keyboard::KeyboardManager,
    mouse::MouseManager,
    service::Service,
    text::FontManager,
    window::{WindowEvent, WindowId, WindowManager},
};
use glutin::event::Event as GEvent;
use glutin::event_loop::{
    ControlFlow, EventLoop as GEventLoop, EventLoopProxy as GEventLoopProxy, EventLoopWindowTarget as GEventLoopWindowTarget,
};
use std::future::Future;
use std::sync::Mutex;
use std::{
    any::{type_name, TypeId},
    fmt,
    sync::atomic::AtomicBool,
    time::Instant,
};

pub use glutin::event::{DeviceEvent, DeviceId, ElementState};

/// Error when the app connected to a sender/receiver channel has shutdown.
///
/// Contains the value that could not be send or `()` for receiver errors.
pub struct AppShutdown<T>(pub T);
impl<T> From<glutin::event_loop::EventLoopClosed<T>> for AppShutdown<T> {
    fn from(e: glutin::event_loop::EventLoopClosed<T>) -> Self {
        AppShutdown(e.0)
    }
}
impl From<flume::RecvError> for AppShutdown<()> {
    fn from(_: flume::RecvError) -> Self {
        AppShutdown(())
    }
}
impl<T> From<flume::SendError<T>> for AppShutdown<T> {
    fn from(e: flume::SendError<T>) -> Self {
        AppShutdown(e.0)
    }
}
impl<T> fmt::Debug for AppShutdown<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AppHasShutdown")
    }
}
impl<T> fmt::Display for AppShutdown<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "cannot send/receive because the app has shutdown")
    }
}
impl<T> std::error::Error for AppShutdown<T> {}

/// Error when the app connected to a sender channel has shutdown or taken to long to respond.
pub enum TimeoutOrAppShutdown {
    /// Connected app has not responded.
    Timeout,
    /// Connected app has shutdown.
    AppShutdown,
}
impl From<flume::RecvTimeoutError> for TimeoutOrAppShutdown {
    fn from(e: flume::RecvTimeoutError) -> Self {
        match e {
            flume::RecvTimeoutError::Timeout => TimeoutOrAppShutdown::Timeout,
            flume::RecvTimeoutError::Disconnected => TimeoutOrAppShutdown::AppShutdown,
        }
    }
}
impl fmt::Debug for TimeoutOrAppShutdown {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "AppHasNotRespondedOrShutdown::")?;
        }
        match self {
            TimeoutOrAppShutdown::Timeout => write!(f, "Timeout"),
            TimeoutOrAppShutdown::AppShutdown => write!(f, "AppShutdown"),
        }
    }
}
impl fmt::Display for TimeoutOrAppShutdown {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TimeoutOrAppShutdown::Timeout => write!(f, "failed send, timeout"),
            TimeoutOrAppShutdown::AppShutdown => write!(f, "cannot send because the app has shutdown"),
        }
    }
}
impl std::error::Error for TimeoutOrAppShutdown {}

/// A future that receives a single message from a running [app](App).
pub struct RecvFut<'a, M>(flume::r#async::RecvFut<'a, M>);
impl<'a, M> From<flume::r#async::RecvFut<'a, M>> for RecvFut<'a, M> {
    fn from(f: flume::r#async::RecvFut<'a, M>) -> Self {
        Self(f)
    }
}
impl<'a, M> Future for RecvFut<'a, M> {
    type Output = Result<M, AppShutdown<()>>;

    fn poll(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        match std::pin::Pin::new(&mut self.0).poll(cx) {
            std::task::Poll::Ready(r) => std::task::Poll::Ready(r.map_err(|_| AppShutdown(()))),
            std::task::Poll::Pending => std::task::Poll::Pending,
        }
    }
}

/// An [`App`] extension.
pub trait AppExtension: 'static {
    /// Type id of this extension.
    #[inline]
    fn id(&self) -> TypeId {
        TypeId::of::<Self>()
    }

    /// If this extension is the `app_extension_id` or dispatches to it.
    #[inline]
    fn is_or_contain(&self, app_extension_id: TypeId) -> bool {
        self.id() == app_extension_id
    }

    /// Initializes this extension.
    #[inline]
    fn init(&mut self, ctx: &mut AppInitContext) {
        let _ = ctx;
    }

    /// If the application should listen to device events.
    ///
    /// This is called zero or one times after [`init`](Self::init).
    ///
    /// This is `false` by default.
    #[inline]
    fn enable_device_events(&self) -> bool {
        false
    }

    /// Called when the OS sends a global device event.
    ///
    /// This is only called is [`enable_device_events`](Self::enable_device_events) is `true`.
    #[inline]
    fn device_event(&mut self, ctx: &mut AppContext, device_id: DeviceId, event: &DeviceEvent) {
        let _ = (ctx, device_id, event);
    }

    /// Called when the OS sends an event to a window.
    #[inline]
    fn window_event(&mut self, ctx: &mut AppContext, window_id: WindowId, event: &WindowEvent) {
        let _ = (ctx, window_id, event);
    }

    /// Called just before [`update_ui`](Self::update_ui).
    ///
    /// Extensions can handle this method to interact with updates before the UI.
    ///
    /// Note that this is not related to the `on_event_preview` properties, all UI events
    /// happen in `update_ui`.
    #[inline]
    fn update_preview(&mut self, ctx: &mut AppContext) {
        let _ = ctx;
    }

    /// Called just before [`update`](Self::update).
    ///
    /// Only extensions that generate windows must handle this method. The [`UiNode::update`](super::UiNode::update)
    /// method is called here.
    #[inline]
    fn update_ui(&mut self, ctx: &mut AppContext) {
        let _ = ctx;
    }

    /// Called after every [`update_ui`](Self::update_ui).
    ///
    /// This is the general extensions update, it gives the chance for
    /// the UI to signal stop propagation.
    #[inline]
    fn update(&mut self, ctx: &mut AppContext) {
        let _ = ctx;
    }

    /// Called just before [`event_ui`](Self::event_ui).
    ///
    /// Extensions can handle this method to to intersect event updates before the UI.
    ///
    /// Note that this is not related to the `on_event_preview` properties, all UI events
    /// happen in `on_event_ui`.
    #[inline]
    fn event_preview<EV: EventUpdateArgs>(&mut self, ctx: &mut AppContext, args: &EV) {
        let _ = (ctx, args);
    }

    /// Called just before [`event`](Self::event).
    ///
    /// Only extensions that generate windows must handle this method. The [`UiNode::event`](super::UiNode::event)
    /// method is called here.
    #[inline]
    fn event_ui<EV: EventUpdateArgs>(&mut self, ctx: &mut AppContext, args: &EV) {
        let _ = (ctx, args);
    }

    /// Called after every [`event_ui`](Self::event_ui).
    ///
    /// This is the general extensions event handler, it gives the chance for the UI to signal stop propagation.
    #[inline]
    fn event<EV: EventUpdateArgs>(&mut self, ctx: &mut AppContext, args: &EV) {
        let _ = (ctx, args);
    }

    /// Called after every sequence of updates if display update was requested.
    #[inline]
    fn update_display(&mut self, ctx: &mut AppContext, update: UpdateDisplayRequest) {
        let _ = (ctx, update);
    }

    /// Called when a new frame is ready to be presented.
    #[inline]
    fn new_frame_ready(&mut self, ctx: &mut AppContext, window_id: WindowId) {
        let _ = (ctx, window_id);
    }

    /// Called when the OS sends a request for re-drawing the last frame.
    #[inline]
    fn redraw_requested(&mut self, ctx: &mut AppContext, window_id: WindowId) {
        let _ = (ctx, window_id);
    }

    /// Called when a shutdown was requested.
    #[inline]
    fn shutdown_requested(&mut self, ctx: &mut AppContext, args: &ShutdownRequestedArgs) {
        let _ = (ctx, args);
    }

    /// Called when the application is shutting down.
    ///
    /// Update requests and event notifications generated during this call are ignored.
    #[inline]
    fn deinit(&mut self, ctx: &mut AppContext) {
        let _ = ctx;
    }

    /// The extension in a box.
    fn boxed(self) -> Box<dyn AppExtensionBoxed>
    where
        Self: Sized,
    {
        Box::new(self)
    }
}

/// Boxed version of [`AppExtension`].
#[doc(hidden)]
pub trait AppExtensionBoxed: 'static {
    fn id_boxed(&self) -> TypeId;
    fn is_or_contain_boxed(&self, app_extension_id: TypeId) -> bool;
    fn init_boxed(&mut self, ctx: &mut AppInitContext);
    fn enable_device_events_boxed(&self) -> bool;
    fn device_event_boxed(&mut self, ctx: &mut AppContext, device_id: DeviceId, event: &DeviceEvent);
    fn window_event_boxed(&mut self, ctx: &mut AppContext, window_id: WindowId, event: &WindowEvent);
    fn update_preview_boxed(&mut self, ctx: &mut AppContext);
    fn update_ui_boxed(&mut self, ctx: &mut AppContext);
    fn update_boxed(&mut self, ctx: &mut AppContext);
    fn event_preview_boxed(&mut self, ctx: &mut AppContext, args: &AnyEventUpdate);
    fn event_ui_boxed(&mut self, ctx: &mut AppContext, args: &AnyEventUpdate);
    fn event_boxed(&mut self, ctx: &mut AppContext, args: &AnyEventUpdate);
    fn update_display_boxed(&mut self, ctx: &mut AppContext, update: UpdateDisplayRequest);
    fn new_frame_ready_boxed(&mut self, ctx: &mut AppContext, window_id: WindowId);
    fn redraw_requested_boxed(&mut self, ctx: &mut AppContext, window_id: WindowId);
    fn shutdown_requested_boxed(&mut self, ctx: &mut AppContext, args: &ShutdownRequestedArgs);
    fn deinit_boxed(&mut self, ctx: &mut AppContext);
}
impl<T: AppExtension> AppExtensionBoxed for T {
    fn id_boxed(&self) -> TypeId {
        self.id()
    }

    fn is_or_contain_boxed(&self, app_extension_id: TypeId) -> bool {
        self.is_or_contain(app_extension_id)
    }

    fn init_boxed(&mut self, ctx: &mut AppInitContext) {
        self.init(ctx);
    }

    fn enable_device_events_boxed(&self) -> bool {
        self.enable_device_events()
    }

    fn device_event_boxed(&mut self, ctx: &mut AppContext, device_id: DeviceId, event: &DeviceEvent) {
        self.device_event(ctx, device_id, event);
    }

    fn window_event_boxed(&mut self, ctx: &mut AppContext, window_id: WindowId, event: &WindowEvent) {
        self.window_event(ctx, window_id, event);
    }

    fn update_preview_boxed(&mut self, ctx: &mut AppContext) {
        self.update_preview(ctx);
    }

    fn update_ui_boxed(&mut self, ctx: &mut AppContext) {
        self.update_ui(ctx);
    }

    fn update_boxed(&mut self, ctx: &mut AppContext) {
        self.update(ctx);
    }

    fn event_preview_boxed(&mut self, ctx: &mut AppContext, args: &AnyEventUpdate) {
        self.event_preview(ctx, args);
    }

    fn event_ui_boxed(&mut self, ctx: &mut AppContext, args: &AnyEventUpdate) {
        self.event_ui(ctx, args);
    }

    fn event_boxed(&mut self, ctx: &mut AppContext, args: &AnyEventUpdate) {
        self.event(ctx, args);
    }

    fn update_display_boxed(&mut self, ctx: &mut AppContext, update: UpdateDisplayRequest) {
        self.update_display(ctx, update);
    }

    fn new_frame_ready_boxed(&mut self, ctx: &mut AppContext, window_id: WindowId) {
        self.new_frame_ready(ctx, window_id);
    }

    fn redraw_requested_boxed(&mut self, ctx: &mut AppContext, window_id: WindowId) {
        self.redraw_requested(ctx, window_id);
    }

    fn shutdown_requested_boxed(&mut self, ctx: &mut AppContext, args: &ShutdownRequestedArgs) {
        self.shutdown_requested(ctx, args);
    }

    fn deinit_boxed(&mut self, ctx: &mut AppContext) {
        self.deinit(ctx);
    }
}
impl AppExtension for Box<dyn AppExtensionBoxed> {
    fn id(&self) -> TypeId {
        self.as_ref().id_boxed()
    }

    fn is_or_contain(&self, app_extension_id: TypeId) -> bool {
        self.as_ref().is_or_contain_boxed(app_extension_id)
    }

    fn init(&mut self, ctx: &mut AppInitContext) {
        self.as_mut().init_boxed(ctx);
    }

    fn enable_device_events(&self) -> bool {
        self.as_ref().enable_device_events_boxed()
    }

    fn device_event(&mut self, ctx: &mut AppContext, device_id: DeviceId, event: &DeviceEvent) {
        self.as_mut().device_event_boxed(ctx, device_id, event);
    }

    fn window_event(&mut self, ctx: &mut AppContext, window_id: WindowId, event: &WindowEvent) {
        self.as_mut().window_event_boxed(ctx, window_id, event);
    }

    fn update_preview(&mut self, ctx: &mut AppContext) {
        self.as_mut().update_preview_boxed(ctx);
    }

    fn update_ui(&mut self, ctx: &mut AppContext) {
        self.as_mut().update_ui_boxed(ctx);
    }

    fn update(&mut self, ctx: &mut AppContext) {
        self.as_mut().update_boxed(ctx);
    }

    fn event_preview<EV: EventUpdateArgs>(&mut self, ctx: &mut AppContext, args: &EV) {
        let args = args.as_any();
        self.as_mut().event_preview_boxed(ctx, &args);
    }

    fn event_ui<EV: EventUpdateArgs>(&mut self, ctx: &mut AppContext, args: &EV) {
        let args = args.as_any();
        self.as_mut().event_ui_boxed(ctx, &args);
    }

    fn event<EV: EventUpdateArgs>(&mut self, ctx: &mut AppContext, args: &EV) {
        let args = args.as_any();
        self.as_mut().event_boxed(ctx, &args);
    }

    fn update_display(&mut self, ctx: &mut AppContext, update: UpdateDisplayRequest) {
        self.as_mut().update_display_boxed(ctx, update);
    }

    fn new_frame_ready(&mut self, ctx: &mut AppContext, window_id: WindowId) {
        self.as_mut().new_frame_ready_boxed(ctx, window_id);
    }

    fn redraw_requested(&mut self, ctx: &mut AppContext, window_id: WindowId) {
        self.as_mut().redraw_requested_boxed(ctx, window_id);
    }

    fn shutdown_requested(&mut self, ctx: &mut AppContext, args: &ShutdownRequestedArgs) {
        self.as_mut().shutdown_requested_boxed(ctx, args);
    }

    fn deinit(&mut self, ctx: &mut AppContext) {
        self.as_mut().deinit_boxed(ctx);
    }

    fn boxed(self) -> Box<dyn AppExtensionBoxed>
    where
        Self: Sized,
    {
        self
    }
}

cancelable_event_args! {
    /// Arguments for `on_shutdown_requested`.
    pub struct ShutdownRequestedArgs {
        ..
        /// Always true.
        fn concerns_widget(&self, _: &mut WidgetContext) -> bool {
            true
        }
    }
}

/// Defines and runs an application.
///
/// # Debug Log
///
/// In debug builds, `App` sets a [`logger`](log) that prints warnings and errors to `stderr`
/// if no logger was registered before the call to [`blank`](Self::blank) or [`default`](Self::default).
pub struct App;

impl App {
    /// If a headed app is running in the current process.
    ///
    /// Only a single headed app is allowed per-process.
    #[inline]
    pub fn is_headed_running() -> bool {
        HEADED_APP_RUNNING.load(std::sync::atomic::Ordering::Acquire)
    }

    /// If an app is already running in the current thread.
    ///
    /// Only a single app is allowed per-thread and only a single headed app is allowed per-process.
    #[inline]
    pub fn is_running() -> bool {
        crate::var::Vars::instantiated() || crate::event::Events::instantiated()
    }
}

// In release mode we use generics tricks to compile all app extensions with
// static dispatch optimized to a direct call to the extension handle.
#[cfg(not(debug_assertions))]
impl App {
    /// Application without any extension.
    #[inline]
    pub fn blank() -> AppExtended<()> {
        AppExtended { extensions: () }
    }

    /// Application with default extensions.
    ///
    /// # Extensions
    ///
    /// Extensions included.
    ///
    /// * [MouseManager]
    /// * [KeyboardManager]
    /// * [GestureManager]
    /// * [WindowManager]
    /// * [FontManager]
    /// * [FocusManager]
    #[inline]
    pub fn default() -> AppExtended<impl AppExtension> {
        App::blank()
            .extend(MouseManager::default())
            .extend(KeyboardManager::default())
            .extend(GestureManager::default())
            .extend(WindowManager::default())
            .extend(FontManager::default())
            .extend(FocusManager::default())
    }
}

// In debug mode we use dynamic dispatch to reduce the number of types
// in the stack-trace and compile more quickly.
#[cfg(debug_assertions)]
impl App {
    /// Application without any extension and without device events.
    pub fn blank() -> AppExtended<Vec<Box<dyn AppExtensionBoxed>>> {
        DebugLogger::init();
        AppExtended { extensions: vec![] }
    }

    /// Application with default extensions.
    ///
    /// # Extensions
    ///
    /// Extensions included.
    ///
    /// * [MouseManager]
    /// * [KeyboardManager]
    /// * [GestureManager]
    /// * [WindowManager]
    /// * [FontManager]
    /// * [FocusManager]
    pub fn default() -> AppExtended<Vec<Box<dyn AppExtensionBoxed>>> {
        App::blank()
            .extend(MouseManager::default())
            .extend(KeyboardManager::default())
            .extend(GestureManager::default())
            .extend(WindowManager::default())
            .extend(FontManager::default())
            .extend(FocusManager::default())
    }
}

/// Application with extensions.
pub struct AppExtended<E: AppExtension> {
    extensions: E,
}

/// Cancellation message of a [shutdown request](AppProcess::shutdown).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShutdownCancelled;
impl fmt::Display for ShutdownCancelled {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "shutdown cancelled")
    }
}

/// Service for managing the application process.
///
/// This is the only service that is registered without an application extension.
#[derive(Service)]
pub struct AppProcess {
    shutdown_requests: Option<ResponderVar<ShutdownCancelled>>,
    update_sender: UpdateSender,
}
impl AppProcess {
    /// New app process service
    pub fn new(update_sender: UpdateSender) -> Self {
        AppProcess {
            shutdown_requests: None,
            update_sender,
        }
    }

    /// Register a request for process shutdown in the next update.
    ///
    /// Returns an event listener that is updated once with the unit value [`ShutdownCancelled`]
    /// if the shutdown operation is cancelled.
    pub fn shutdown(&mut self) -> ResponseVar<ShutdownCancelled> {
        if let Some(r) = &self.shutdown_requests {
            r.response_var()
        } else {
            let (responder, response) = response_var();
            self.shutdown_requests = Some(responder);
            let _ = self.update_sender.send();
            response
        }
    }

    fn take_requests(&mut self) -> Option<ResponderVar<ShutdownCancelled>> {
        self.shutdown_requests.take()
    }
}

#[derive(Debug)]
enum EventLoopInner {
    Glutin(GEventLoop<AppEvent>),
    Headless((flume::Sender<AppEvent>, flume::Receiver<AppEvent>)),
}

/// Provides a way to retrieve events from the system and from the windows that were registered to the events loop.
/// Can be a fake headless event loop too.
#[derive(Debug)]
pub struct EventLoop(EventLoopInner);

impl EventLoop {
    /// Initializes a new event loop.
    pub fn new(headless: bool) -> Self {
        if headless {
            EventLoop(EventLoopInner::Headless(flume::unbounded()))
        } else {
            EventLoop(EventLoopInner::Glutin(GEventLoop::with_user_event()))
        }
    }

    /// If the event loop is a headless.
    pub fn is_headless(&self) -> bool {
        matches!(&self.0, EventLoopInner::Headless(_))
    }

    /// Takes the headless user events send since the last call.
    ///
    /// # Panics
    ///
    /// If the event loop is not headless panics with the message: `"cannot take user events from headed EventLoop`.
    pub fn take_headless_app_events(&self, wait: bool) -> Vec<AppEvent> {
        match &self.0 {
            EventLoopInner::Headless((_, rcv)) => {
                if wait && rcv.is_empty() {
                    let mut buffer = Vec::with_capacity(1);
                    if let Ok(r) = rcv.recv() {
                        buffer.push(r);
                    }
                    buffer.extend(rcv.try_iter());
                    buffer
                } else {
                    rcv.try_iter().collect()
                }
            }
            _ => panic!("cannot take user events from headed EventLoop"),
        }
    }

    /// Hijacks the calling thread and initializes the winit event loop with the provided
    /// closure. Since the closure is `'static`, it must be a `move` closure if it needs to
    /// access any data from the calling context.
    ///
    /// See the [`ControlFlow`] docs for information on how changes to `&mut ControlFlow` impact the
    /// event loop's behavior.
    ///
    /// Any values not passed to this function will *not* be dropped.
    ///
    /// # Panics
    ///
    /// If called when headless panics with the message: `"cannot run headless EventLoop"`.
    ///
    /// [`ControlFlow`]: glutin::event_loop::ControlFlow
    #[inline]
    pub fn run_headed<F>(self, mut event_handler: F) -> !
    where
        F: 'static + FnMut(GEvent<'_, AppEvent>, EventLoopWindowTarget<'_>, &mut ControlFlow),
    {
        match self.0 {
            EventLoopInner::Glutin(el) => el.run(move |e, l, c| event_handler(e, EventLoopWindowTarget(Some(l)), c)),
            EventLoopInner::Headless(_) => panic!("cannot run headless EventLoop"),
        }
    }

    /// Borrows a [`EventLoopWindowTarget`].
    #[inline]
    pub fn window_target(&self) -> EventLoopWindowTarget<'_> {
        match &self.0 {
            EventLoopInner::Glutin(el) => EventLoopWindowTarget(Some(el)),
            EventLoopInner::Headless(_) => EventLoopWindowTarget(None),
        }
    }

    /// Creates an [`EventLoopProxy`] that can be used to dispatch user events to the main event loop.
    pub fn create_proxy(&self) -> EventLoopProxy {
        match &self.0 {
            EventLoopInner::Glutin(el) => EventLoopProxy(EventLoopProxyInner::Winit(el.create_proxy())),
            EventLoopInner::Headless((s, _)) => EventLoopProxy(EventLoopProxyInner::Headless(s.clone())),
        }
    }
}

/// Target that associates windows with an [`EventLoop`].
#[derive(Debug, Clone, Copy)]
pub struct EventLoopWindowTarget<'a>(Option<&'a GEventLoopWindowTarget<AppEvent>>);

impl<'a> EventLoopWindowTarget<'a> {
    /// If this window target is a dummy for a headless context.
    pub fn is_headless(self) -> bool {
        self.0.is_none()
    }

    /// Get the actual window target.
    pub fn headed_target(self) -> Option<&'a GEventLoopWindowTarget<AppEvent>> {
        self.0
    }
}

#[derive(Debug, Clone)]
enum EventLoopProxyInner {
    Winit(GEventLoopProxy<AppEvent>),
    Headless(flume::Sender<AppEvent>),
}

/// Used to send custom events to [`EventLoop`].
#[derive(Debug, Clone)]
pub struct EventLoopProxy(EventLoopProxyInner);
impl EventLoopProxy {
    /// If this event loop is from a [headless app](HeadlessApp).
    pub fn is_headless(&self) -> bool {
        match &self.0 {
            EventLoopProxyInner::Headless(_) => true,
            EventLoopProxyInner::Winit(_) => false,
        }
    }

    /// Send an [`AppEvent`] to the app.
    #[inline]
    pub fn send_event(&self, event: AppEvent) -> Result<(), AppShutdown<AppEvent>> {
        match &self.0 {
            EventLoopProxyInner::Winit(elp) => elp.send_event(event).map_err(|e| e.into()),
            EventLoopProxyInner::Headless(sender) => sender.send(event).map_err(|e| AppShutdown(e.0)),
        }
    }

    /// Convert the loop proxy to a version that implements [`Sync`].
    #[inline]
    pub fn into_sync(self) -> EventLoopProxySync {
        EventLoopProxySync(match self.0 {
            EventLoopProxyInner::Winit(l) => EventLoopProxyInnerSync::Winit(Mutex::new(l)),
            EventLoopProxyInner::Headless(l) => EventLoopProxyInnerSync::Headless(l),
        })
    }
}

#[derive(Debug)]
enum EventLoopProxyInnerSync {
    Winit(Mutex<GEventLoopProxy<AppEvent>>),
    Headless(flume::Sender<AppEvent>),
}
impl Clone for EventLoopProxyInnerSync {
    fn clone(&self) -> Self {
        match self {
            EventLoopProxyInnerSync::Winit(l) => EventLoopProxyInnerSync::Winit(Mutex::new(l.lock().unwrap().clone())),
            EventLoopProxyInnerSync::Headless(l) => EventLoopProxyInnerSync::Headless(l.clone()),
        }
    }
}

/// Alternative of [`EventLoopProxy`] that implements [`Sync`].
///
/// The headless event loop is naturally `Sync`, the headed `winit` event loop proxy is wrapped in a [`Mutex`].
#[derive(Debug, Clone)]
pub struct EventLoopProxySync(EventLoopProxyInnerSync);
impl EventLoopProxySync {
    /// If this event loop is from a [headless app](HeadlessApp).
    pub fn is_headless(&self) -> bool {
        match &self.0 {
            EventLoopProxyInnerSync::Headless(_) => true,
            EventLoopProxyInnerSync::Winit(_) => false,
        }
    }

    /// Send an [`AppEvent`] to the app.
    #[inline]
    pub fn send_event(&self, event: AppEvent) -> Result<(), AppShutdown<AppEvent>> {
        match &self.0 {
            EventLoopProxyInnerSync::Winit(el) => match el.lock() {
                Ok(el) => el.send_event(event).map_err(|e| e.into()),
                Err(e) => {
                    // if the winit channel is corrupted the whole app is F because not all
                    // proxy senders know about this mutex, so we log the error and hope the channel is ok.
                    log::error!("sending message using poisoned `EventLoopProxySync`");
                    e.into_inner().send_event(event).map_err(|e| e.into())
                }
            },
            EventLoopProxyInnerSync::Headless(sender) => sender.send(event).map_err(|e| AppShutdown(e.0)),
        }
    }
}
impl std::task::Wake for EventLoopProxySync {
    /// Sends an [`AppEvent::Update`].
    fn wake(self: std::sync::Arc<Self>) {
        if let Err(e) = self.send_event(AppEvent::Update) {
            log::error!("failed to wake using `EventLoopProxySync`, {}", e);
        }
    }
}
impl From<EventLoopProxy> for EventLoopProxySync {
    fn from(el: EventLoopProxy) -> Self {
        el.into_sync()
    }
}
impl From<EventLoopProxySync> for EventLoopProxy {
    fn from(el: EventLoopProxySync) -> Self {
        Self(match el.0 {
            EventLoopProxyInnerSync::Winit(el) => EventLoopProxyInner::Winit(match el.into_inner() {
                Ok(el) => el,
                Err(e) => {
                    // if the winit channel is corrupted the whole app is F because not all
                    // proxy senders know about this mutex, so we log the error and hope the channel is ok.
                    log::error!("converting poisoned `EventLoopProxySync` to `EventLoopProxy`");
                    e.into_inner()
                }
            }),
            EventLoopProxyInnerSync::Headless(el) => EventLoopProxyInner::Headless(el),
        })
    }
}

#[cfg(debug_assertions)]
impl AppExtended<Vec<Box<dyn AppExtensionBoxed>>> {
    /// Includes an application extension.
    ///
    /// # Panics
    /// * `"app already extended with `{}`"` when the app is already [`extended_with`](AppExtended::extended_with) the
    /// extension type.
    #[inline]
    pub fn extend<F: AppExtension>(self, extension: F) -> AppExtended<Vec<Box<dyn AppExtensionBoxed>>> {
        if self.extended_with::<F>() {
            panic!("app already extended with `{}`", type_name::<F>())
        }

        let mut extensions = self.extensions;
        extensions.push(extension.boxed());

        AppExtended { extensions }
    }
}

#[cfg(not(debug_assertions))]
impl<E: AppExtension> AppExtended<E> {
    /// Includes an application extension.
    ///
    /// # Panics
    /// * `"app already extended with `{}`"` when the app is already [`extended_with`](AppExtended::extended_with) the
    /// extension type.
    #[inline]
    pub fn extend<F: AppExtension>(self, extension: F) -> AppExtended<impl AppExtension> {
        if self.extended_with::<F>() {
            panic!("app already extended with `{}`", type_name::<F>())
        }
        AppExtended {
            extensions: (self.extensions, extension),
        }
    }
}

static HEADED_APP_RUNNING: AtomicBool = AtomicBool::new(false);

impl<E: AppExtension> AppExtended<E> {
    /// Gets if the application is already extended with the extension type.
    #[inline]
    pub fn extended_with<F: AppExtension>(&self) -> bool {
        self.extensions.is_or_contain(TypeId::of::<F>())
    }

    /// Runs the application calling `start` once at the beginning.
    ///
    /// # Panics
    ///
    /// Panics if not called by the main thread. This means you cannot run an app in unit tests, use a headless
    /// app without renderer for that. The main thread is required by some operating systems and OpenGL.
    pub fn run(self, start: impl FnOnce(&mut AppContext)) -> ! {
        if !is_main_thread::is_main_thread().unwrap_or(true) {
            panic!("can only init headed app in the main thread")
        }
        if HEADED_APP_RUNNING.swap(true, std::sync::atomic::Ordering::AcqRel) {
            panic!("only one headed app is allowed per process")
        }

        #[cfg(feature = "app_profiler")]
        register_thread_with_profiler();

        profile_scope!("app::run");

        let event_loop = EventLoop::new(false);

        let mut app = RunningApp::start(self.extensions, event_loop.create_proxy());

        start(&mut app.ctx(event_loop.window_target()));

        app.run_headed(event_loop)
    }

    /// Runs the application with an async `start` function.
    ///
    /// The `start` future is executed in the app thread only (the main thread), it runs up to the first `await` immediately
    /// and subsequent polls happen in app updates, it is async but not parallel. You can use [`Tasks`](crate::task::Tasks)
    /// to start parallel tasks that can be awaited in the app thread.
    ///
    /// # Panics
    ///
    /// Panics if not called by the main thread. The same caveats of [`run`](Self::run) apply to this method.
    pub fn run_async<F, S>(self, start: S) -> !
    where
        F: Future<Output = ()> + 'static,
        S: FnOnce(AppContextMut) -> F,
    {
        self.run(move |ctx| {
            let mut task = ctx.async_task(start);
            if task.update(ctx).is_none() {
                task.run(ctx.updates);
            }
        })
    }

    /// Initializes extensions in headless mode and returns an [`HeadlessApp`].
    ///
    /// # Tests
    ///
    /// If called in a test (`cfg(test)`) this blocks until no other instance of [`HeadlessApp`] and
    /// [`TestWidgetContext`] are running in the current thread.
    pub fn run_headless(self) -> HeadlessApp {
        #[cfg(feature = "app_profiler")]
        let profile_scope = {
            register_thread_with_profiler();
            ProfileScope::new("app::run_headless")
        };

        let event_loop = EventLoop::new(true);

        let app = RunningApp::start(self.extensions.boxed(), event_loop.create_proxy());

        HeadlessApp {
            event_loop,
            app,

            #[cfg(feature = "app_profiler")]
            _pf: profile_scope,
        }
    }

    /// Start a [`RunningApp`] that will be controlled by an external event loop.
    pub fn run_client(self, event_loop: EventLoopProxy) -> RunningApp<E> {
        RunningApp::start(self.extensions, event_loop)
    }

    /// Start a [`RunningApp`] that will be controlled by an external event loop, the app extensions
    /// are boxed making the app type more manageable.
    pub fn run_client_boxed(self, event_loop: EventLoopProxy) -> RunningApp<Box<dyn AppExtensionBoxed>> {
        RunningApp::start(self.extensions.boxed(), event_loop)
    }
}

/// Represents a running app controlled by an external event loop.
pub struct RunningApp<E: AppExtension> {
    extensions: E,
    device_events: bool,
    owned_ctx: OwnedAppContext,

    // need to probe context to see if there are updates.
    maybe_has_updates: bool,
    // WaitUntil time.
    wake_time: Option<Instant>,

    // shutdown was requested.
    exiting: bool,
}
impl<E: AppExtension> RunningApp<E> {
    fn start(mut extensions: E, event_loop: EventLoopProxy) -> Self {
        if App::is_running() {
            if cfg!(any(test, doc, feature = "pub_test")) {
                panic!("only one app or `TestWidgetContext` is allowed per thread")
            } else {
                panic!("only one app is allowed per thread")
            }
        }

        let mut owned_ctx = OwnedAppContext::instance(event_loop);

        let mut init_ctx = owned_ctx.borrow_init();
        init_ctx.services.register(AppProcess::new(init_ctx.updates.sender().clone()));
        extensions.init(&mut init_ctx);

        RunningApp {
            device_events: extensions.enable_device_events(),
            extensions,
            owned_ctx,
            maybe_has_updates: true,
            wake_time: None,
            exiting: false,
        }
    }

    fn run_headed(self, event_loop: EventLoop) -> ! {
        let mut app = Some(self);
        event_loop.run_headed(move |event, event_loop, control_flow| {
            if let GEvent::LoopDestroyed = &event {
                app.take().unwrap().shutdown(event_loop);
                return;
            }

            let app = app.as_mut().expect("app already shutdown");

            match event {
                GEvent::NewEvents(c) => {
                    if let glutin::event::StartCause::ResumeTimeReached { .. } = c {
                        app.wait_until_elapsed();
                    }
                }
                GEvent::WindowEvent { window_id, event } => app.window_event(event_loop, window_id.into(), &event),
                GEvent::DeviceEvent { device_id, event } => app.device_event(event_loop, device_id, &event),
                GEvent::UserEvent(app_event) => app.app_event(event_loop, app_event),
                GEvent::Suspended => app.suspended(event_loop),
                GEvent::Resumed => app.resumed(event_loop),
                GEvent::MainEventsCleared => {
                    *control_flow = app.update(event_loop, &mut ());
                }
                GEvent::RedrawRequested(window_id) => app.redraw_requested(event_loop, window_id.into()),
                GEvent::RedrawEventsCleared => {}
                GEvent::LoopDestroyed => unreachable!(),
            }
        })
    }

    /// Exclusive borrow the app context.
    pub fn ctx<'a>(&'a mut self, event_loop: EventLoopWindowTarget<'a>) -> AppContext<'a> {
        self.maybe_has_updates = true;
        self.owned_ctx.borrow(event_loop)
    }

    /// Event loop has awakened because [`WaitUntil`](ControlFlow::WaitUntil) was requested.
    pub fn wait_until_elapsed(&mut self) {
        self.maybe_has_updates = true;
    }

    /// Process window event.
    pub fn window_event(&mut self, event_loop: EventLoopWindowTarget, window_id: WindowId, event: &WindowEvent) {
        let mut ctx = self.owned_ctx.borrow(event_loop);
        self.extensions.window_event(&mut ctx, window_id, event);
        self.maybe_has_updates = true;
    }

    /// Process device event.
    pub fn device_event(&mut self, event_loop: EventLoopWindowTarget, device_id: DeviceId, event: &DeviceEvent) {
        if self.device_events {
            let mut ctx = self.owned_ctx.borrow(event_loop);
            self.extensions.device_event(&mut ctx, device_id, event);
            self.maybe_has_updates = true;
        }
    }

    /// Process an [`AppEvent`].
    pub fn app_event(&mut self, event_loop: EventLoopWindowTarget, app_event: AppEvent) {
        match app_event {
            AppEvent::NewFrameReady(window_id) => {
                let mut ctx = self.owned_ctx.borrow(event_loop);
                self.extensions.new_frame_ready(&mut ctx, window_id);
            }
            AppEvent::Update => {
                self.owned_ctx.borrow(event_loop).updates.update();
            }
            AppEvent::Event(e) => {
                self.owned_ctx.borrow(event_loop).events.notify_app_event(e);
            }
            AppEvent::Var => {
                self.owned_ctx.borrow(event_loop).vars.sync();
            }
        }
        self.maybe_has_updates = true;
    }

    /// Process application suspension.
    pub fn suspended(&mut self, _event_loop: EventLoopWindowTarget) {
        log::error!(target: "app", "TODO suspended");
    }

    /// Process application resume from suspension.
    pub fn resumed(&mut self, _event_loop: EventLoopWindowTarget) {
        log::error!(target: "app", "TODO resumed");
    }

    /// Does pending event and updates until there is no more updates generated, then returns
    /// [`WaitUntil`](ControlFlow::WaitUntil) are timers running or returns [`Wait`](ControlFlow::WaitUntil)
    /// if there aren't.
    ///
    /// You can use an [`AppUpdateObserver`] to watch all of these actions or pass `&mut ()` as a NOP observer.
    pub fn update<O: AppUpdateObserver>(&mut self, event_loop: EventLoopWindowTarget, observer: &mut O) -> ControlFlow {
        if self.maybe_has_updates {
            self.maybe_has_updates = false;

            let mut display_update = UpdateDisplayRequest::None;

            let mut limit = 100_000;
            loop {
                limit -= 1;
                if limit == 0 {
                    panic!("update loop polled 100,000 times, probably stuck in an infinite loop");
                }

                let u = self.owned_ctx.apply_updates();

                self.wake_time = u.wake_time;
                display_update |= u.display_update;

                if u.update {
                    let mut ctx = self.owned_ctx.borrow(event_loop);

                    // check shutdown.
                    if let Some(r) = ctx.services.app_process().take_requests() {
                        let args = ShutdownRequestedArgs::now();
                        self.extensions.shutdown_requested(&mut ctx, &args);
                        if args.cancel_requested() {
                            r.respond(ctx.vars, ShutdownCancelled);
                        }
                        self.exiting = !args.cancel_requested();
                        if self.exiting {
                            return ControlFlow::Exit;
                        }
                    }

                    // does `Timers::on_*` notifications.
                    Timers::notify(&mut ctx);

                    // does `Event` notifications.
                    for event in u.events {
                        self.extensions.event_preview(&mut ctx, &event);
                        observer.event_preview(&mut ctx, &event);
                        Events::on_pre_events(&mut ctx, &event);

                        self.extensions.event_ui(&mut ctx, &event);
                        observer.event_ui(&mut ctx, &event);

                        self.extensions.event(&mut ctx, &event);
                        observer.event(&mut ctx, &event);
                        Events::on_events(&mut ctx, &event);
                    }

                    // does general updates.
                    self.extensions.update_preview(&mut ctx);
                    observer.update_preview(&mut ctx);
                    Updates::on_pre_updates(&mut ctx);

                    self.extensions.update_ui(&mut ctx);
                    observer.update_ui(&mut ctx);

                    self.extensions.update(&mut ctx);
                    observer.update(&mut ctx);
                    Updates::on_updates(&mut ctx);
                } else if display_update != UpdateDisplayRequest::None {
                    display_update = UpdateDisplayRequest::None;

                    let mut ctx = self.owned_ctx.borrow(event_loop);

                    self.extensions.update_display(&mut ctx, display_update);
                    observer.update_display(&mut ctx, display_update);
                } else {
                    break;
                }
            }
        }

        if self.exiting {
            ControlFlow::Exit
        } else if let Some(wake) = self.wake_time {
            ControlFlow::WaitUntil(wake)
        } else {
            ControlFlow::Wait
        }
    }

    /// OS requested a redraw.
    pub fn redraw_requested(&mut self, event_loop: EventLoopWindowTarget, window_id: WindowId) {
        let mut ctx = self.owned_ctx.borrow(event_loop);
        self.extensions.redraw_requested(&mut ctx, window_id);
    }

    /// De-initializes extensions and drops.
    pub fn shutdown(mut self, event_loop: EventLoopWindowTarget) {
        let mut ctx = self.owned_ctx.borrow(event_loop);
        self.extensions.deinit(&mut ctx);
    }
}

/// Raw events generated by the app.
#[derive(Debug)]
pub enum AppEvent {
    /// An [`EventSender`](crate::event::EventSender) update.
    Event(crate::event::BoxedSendEventUpdate),
    /// A [`VarSender`](crate::var::VarSender) has updated.
    Var,
    /// A window frame is ready to be shown.
    NewFrameReady(WindowId),
    /// An update was requested.
    Update,
}

/// A headless app controller.
///
/// Headless apps don't cause external side-effects like visible windows and don't listen to system events.
/// They can be used for creating apps like a command line app that renders widgets, or for creating integration tests.
pub struct HeadlessApp {
    event_loop: EventLoop,
    app: RunningApp<Box<dyn AppExtensionBoxed>>,
    #[cfg(feature = "app_profiler")]
    _pf: ProfileScope,
}
impl HeadlessApp {
    /// Headless state.
    ///
    /// Can be accessed in a context using [`HeadlessInfo`].
    pub fn headless_state(&self) -> &StateMap {
        self.app.owned_ctx.headless_state().unwrap()
    }

    /// Mutable headless state.
    pub fn headless_state_mut(&mut self) -> &mut StateMap {
        self.app.owned_ctx.headless_state_mut().unwrap()
    }

    /// App state.
    pub fn app_state(&self) -> &StateMap {
        self.app.owned_ctx.app_state()
    }

    /// Mutable app state.
    pub fn app_state_mut(&mut self) -> &mut StateMap {
        self.app.owned_ctx.app_state_mut()
    }

    /// If headless rendering is enabled.
    ///
    /// This is disabled by default.
    ///
    /// See [`enable_renderer`](Self::enable_renderer) for more details.
    pub fn renderer_enabled(&self) -> bool {
        self.headless_state()
            .get::<HeadlessRendererEnabledKey>()
            .copied()
            .unwrap_or_default()
    }

    /// Enable or disable headless rendering.
    ///
    /// When enabled windows are still not visible but you can request [frame pixels](crate::window::OpenWindow::frame_pixels)
    /// to get the frame image. Renderer is disabled by default in a headless app.
    ///
    /// Only windows opened after enabling have a renderer. Already open windows are not changed by this method. When enabled
    /// headless windows can only be initialized in the main thread due to limitations of OpenGL, this means you cannot run
    /// a headless renderer in units tests.
    ///
    /// Note that [`UiNode::render`](crate::UiNode::render) is still called when a renderer is disabled and you can still
    /// query the latest frame from [`OpenWindow::frame_info`](crate::window::OpenWindow::frame_info). The only thing that
    /// is disabled is WebRender and the generation of frame textures.
    ///
    /// This sets the [`HeadlessRendererEnabledKey`] state in the [headless state](Self::headless_state).
    pub fn enable_renderer(&mut self, enabled: bool) {
        self.headless_state_mut().set::<HeadlessRendererEnabledKey>(enabled);
    }

    /// Notifies extensions of a [device event](DeviceEvent).
    pub fn device_event(&mut self, device_id: DeviceId, event: &DeviceEvent) {
        profile_scope!("headless_app::device_event");
        self.app.device_event(self.event_loop.window_target(), device_id, event);
    }

    /// Notifies extensions of a [window event](WindowEvent).
    pub fn window_event(&mut self, window_id: WindowId, event: &WindowEvent) {
        profile_scope!("headless_app::device_event");
        self.app.window_event(self.event_loop.window_target(), window_id, event);
    }

    /// Sends an [app event](AppEvent).
    pub fn app_event(&mut self, event: AppEvent) {
        profile_scope!("headless_app::on_app_event");
        let _ = self.event_loop.create_proxy().send_event(event);
    }

    /// Runs a custom action in the headless app context.
    pub fn ctx(&mut self) -> AppContext {
        profile_scope!("headless_app::with_context");
        self.app.ctx(self.event_loop.window_target())
    }

    /// Does updates unobserved.
    ///
    /// See [`update_observed`](Self::update_observed) for more details.
    #[inline]
    pub fn update(&mut self, wait_app_event: bool) -> ControlFlow {
        self.update_observed(&mut (), wait_app_event)
    }

    /// Does updates observing [`update`](AppUpdateObserver::update) only.
    ///
    /// See [`update_observed`](Self::update_observed) for more details.
    pub fn update_observe(&mut self, on_update: impl FnMut(&mut AppContext), wait_app_event: bool) -> ControlFlow {
        struct Observer<F>(F);
        impl<F: FnMut(&mut AppContext)> AppUpdateObserver for Observer<F> {
            fn update(&mut self, ctx: &mut AppContext) {
                (self.0)(ctx)
            }
        }
        let mut observer = Observer(on_update);
        self.update_observed(&mut observer, wait_app_event)
    }

    /// Does updates observing [`event`](AppUpdateObserver::event) only.
    ///
    /// See [`update_observed`](Self::update_observed) for more details.
    pub fn update_observe_event(&mut self, on_event: impl FnMut(&mut AppContext, &AnyEventUpdate), wait_app_event: bool) -> ControlFlow {
        struct Observer<F>(F);
        impl<F: FnMut(&mut AppContext, &AnyEventUpdate)> AppUpdateObserver for Observer<F> {
            fn event<EU: EventUpdateArgs>(&mut self, ctx: &mut AppContext, args: &EU) {
                let args = args.as_any();
                (self.0)(ctx, &args);
            }
        }
        let mut observer = Observer(on_event);
        self.update_observed(&mut observer, wait_app_event)
    }

    /// Does updates with an [`AppUpdateObserver`].
    ///
    /// If `wait_app_event` is `true` the thread sleeps until at least one app event is received,
    /// if it is `false` only responds to app events already in the buffer.
    ///
    /// Does updates until there are no more updates to do, returns [`Exit`](ControlFlow::Exit) if app has shutdown,
    /// or returns [`WaitUntil`](ControlFlow::WaitUntil) if a timer is running or returns [`Wait`](ControlFlow::Wait)
    /// if the app is sleeping.
    pub fn update_observed<O: AppUpdateObserver>(&mut self, observer: &mut O, wait_app_event: bool) -> ControlFlow {
        let event_loop = self.event_loop.window_target();
        for event in self.event_loop.take_headless_app_events(wait_app_event) {
            self.app.app_event(event_loop, event);
        }

        let r = self.app.update(event_loop, observer);
        debug_assert!(r != ControlFlow::Poll);

        r
    }
}

/// Observer for [`HeadlessApp::update_observed`] and [`RunningApp::update`].
pub trait AppUpdateObserver {
    /// Called just after [`AppExtension::event_preview`].
    fn event_preview<EU: EventUpdateArgs>(&mut self, ctx: &mut AppContext, args: &EU) {
        let _ = (ctx, args);
    }

    /// Called just after [`AppExtension::event_ui`].
    fn event_ui<EU: EventUpdateArgs>(&mut self, ctx: &mut AppContext, args: &EU) {
        let _ = (ctx, args);
    }

    /// Called just after [`AppExtension::event`].
    fn event<EU: EventUpdateArgs>(&mut self, ctx: &mut AppContext, args: &EU) {
        let _ = (ctx, args);
    }

    /// Called just after [`AppExtension::update_preview`].
    fn update_preview(&mut self, ctx: &mut AppContext) {
        let _ = ctx;
    }

    /// Called just after [`AppExtension::update_ui`].
    fn update_ui(&mut self, ctx: &mut AppContext) {
        let _ = ctx;
    }

    /// Called just after [`AppExtension::update`].
    fn update(&mut self, ctx: &mut AppContext) {
        let _ = ctx;
    }

    /// Called just after [`AppExtension::update_display`].
    fn update_display(&mut self, ctx: &mut AppContext, update: UpdateDisplayRequest) {
        let _ = (ctx, update);
    }
}
/// Nil observer, does nothing.
impl AppUpdateObserver for () {}

state_key! {
    /// If render is enabled in [headless mode](AppExtended::run_headless).
    ///
    /// See [`HeadlessApp::enable_renderer`] for for details.
    pub struct HeadlessRendererEnabledKey: bool;
}

impl AppExtension for () {
    #[inline]
    fn is_or_contain(&self, _: TypeId) -> bool {
        false
    }
}
impl<A: AppExtension, B: AppExtension> AppExtension for (A, B) {
    #[inline]
    fn init(&mut self, ctx: &mut AppInitContext) {
        self.0.init(ctx);
        self.1.init(ctx);
    }

    #[inline]
    fn is_or_contain(&self, app_extension_id: TypeId) -> bool {
        self.0.is_or_contain(app_extension_id) || self.1.is_or_contain(app_extension_id)
    }

    #[inline]
    fn enable_device_events(&self) -> bool {
        self.0.enable_device_events() || self.1.enable_device_events()
    }

    #[inline]
    fn device_event(&mut self, ctx: &mut AppContext, device_id: DeviceId, event: &DeviceEvent) {
        self.0.device_event(ctx, device_id, event);
        self.1.device_event(ctx, device_id, event);
    }

    #[inline]
    fn window_event(&mut self, ctx: &mut AppContext, window_id: WindowId, event: &WindowEvent) {
        self.0.window_event(ctx, window_id, event);
        self.1.window_event(ctx, window_id, event);
    }

    #[inline]
    fn new_frame_ready(&mut self, ctx: &mut AppContext, window_id: WindowId) {
        self.0.new_frame_ready(ctx, window_id);
        self.1.new_frame_ready(ctx, window_id);
    }

    #[inline]
    fn update_preview(&mut self, ctx: &mut AppContext) {
        self.0.update_preview(ctx);
        self.1.update_preview(ctx);
    }

    #[inline]
    fn update_ui(&mut self, ctx: &mut AppContext) {
        self.0.update_ui(ctx);
        self.1.update_ui(ctx);
    }

    #[inline]
    fn update(&mut self, ctx: &mut AppContext) {
        self.0.update(ctx);
        self.1.update(ctx);
    }

    #[inline]
    fn update_display(&mut self, ctx: &mut AppContext, update: UpdateDisplayRequest) {
        self.0.update_display(ctx, update);
        self.1.update_display(ctx, update);
    }

    #[inline]
    fn event_preview<EV: EventUpdateArgs>(&mut self, ctx: &mut AppContext, args: &EV) {
        self.0.event_preview(ctx, args);
        self.1.event_preview(ctx, args);
    }

    #[inline]
    fn event_ui<EV: EventUpdateArgs>(&mut self, ctx: &mut AppContext, args: &EV) {
        self.0.event_ui(ctx, args);
        self.1.event_ui(ctx, args);
    }

    #[inline]
    fn event<EV: EventUpdateArgs>(&mut self, ctx: &mut AppContext, args: &EV) {
        self.0.event(ctx, args);
        self.1.event(ctx, args);
    }

    #[inline]
    fn redraw_requested(&mut self, ctx: &mut AppContext, window_id: WindowId) {
        self.0.redraw_requested(ctx, window_id);
        self.1.redraw_requested(ctx, window_id);
    }

    #[inline]
    fn shutdown_requested(&mut self, ctx: &mut AppContext, args: &ShutdownRequestedArgs) {
        self.0.shutdown_requested(ctx, args);
        self.1.shutdown_requested(ctx, args);
    }

    #[inline]
    fn deinit(&mut self, ctx: &mut AppContext) {
        self.0.deinit(ctx);
        self.1.deinit(ctx);
    }
}

#[cfg(debug_assertions)]
impl AppExtension for Vec<Box<dyn AppExtensionBoxed>> {
    fn init(&mut self, ctx: &mut AppInitContext) {
        for ext in self {
            ext.init(ctx);
        }
    }

    fn is_or_contain(&self, app_extension_id: TypeId) -> bool {
        for ext in self {
            if ext.is_or_contain(app_extension_id) {
                return true;
            }
        }
        false
    }

    fn enable_device_events(&self) -> bool {
        self.iter().any(|e| e.enable_device_events())
    }

    fn device_event(&mut self, ctx: &mut AppContext, device_id: DeviceId, event: &DeviceEvent) {
        for ext in self {
            ext.device_event(ctx, device_id, event);
        }
    }

    fn window_event(&mut self, ctx: &mut AppContext, window_id: WindowId, event: &WindowEvent) {
        for ext in self {
            ext.window_event(ctx, window_id, event);
        }
    }

    fn new_frame_ready(&mut self, ctx: &mut AppContext, window_id: WindowId) {
        for ext in self {
            ext.new_frame_ready(ctx, window_id);
        }
    }

    fn update_preview(&mut self, ctx: &mut AppContext) {
        for ext in self {
            ext.update_preview(ctx);
        }
    }

    fn update_ui(&mut self, ctx: &mut AppContext) {
        for ext in self {
            ext.update_ui(ctx);
        }
    }

    fn update(&mut self, ctx: &mut AppContext) {
        for ext in self {
            ext.update(ctx);
        }
    }

    fn update_display(&mut self, ctx: &mut AppContext, update: UpdateDisplayRequest) {
        for ext in self {
            ext.update_display(ctx, update);
        }
    }

    fn event_preview<EV: EventUpdateArgs>(&mut self, ctx: &mut AppContext, args: &EV) {
        for ext in self {
            ext.event_preview(ctx, args);
        }
    }

    fn event_ui<EV: EventUpdateArgs>(&mut self, ctx: &mut AppContext, args: &EV) {
        for ext in self {
            ext.event_ui(ctx, args);
        }
    }

    fn event<EV: EventUpdateArgs>(&mut self, ctx: &mut AppContext, args: &EV) {
        for ext in self {
            ext.event(ctx, args);
        }
    }

    fn redraw_requested(&mut self, ctx: &mut AppContext, window_id: WindowId) {
        for ext in self {
            ext.redraw_requested(ctx, window_id);
        }
    }

    fn shutdown_requested(&mut self, ctx: &mut AppContext, args: &ShutdownRequestedArgs) {
        for ext in self {
            ext.shutdown_requested(ctx, args);
        }
    }

    fn deinit(&mut self, ctx: &mut AppContext) {
        for ext in self {
            ext.deinit(ctx);
        }
    }
}

#[cfg(test)]
mod headless_tests {
    use super::*;

    #[test]
    fn new_default() {
        let mut app = App::default().run_headless();
        app.update(false);
    }

    #[test]
    fn new_empty() {
        let mut app = App::blank().run_headless();
        app.update(false);
    }

    #[test]
    pub fn new_window_no_render() {
        let mut app = App::default().run_headless();
        assert!(!app.renderer_enabled());

        let render_enabled = app
            .ctx()
            .headless
            .state()
            .and_then(|s| s.get::<HeadlessRendererEnabledKey>().copied())
            .unwrap_or_default();

        assert!(!render_enabled);

        app.update(false);
    }

    #[test]
    pub fn new_window_with_render() {
        let mut app = App::default().run_headless();
        app.enable_renderer(true);
        assert!(app.renderer_enabled());

        let render_enabled = app
            .ctx()
            .headless
            .state()
            .and_then(|s| s.get::<HeadlessRendererEnabledKey>().copied())
            .unwrap_or_default();

        assert!(render_enabled);
        app.update(false);
    }

    #[test]
    #[should_panic(expected = "only one app or `TestWidgetContext` is allowed per thread")]
    pub fn two_in_one_thread() {
        let _a = App::default().run_headless();
        let _b = App::default().run_headless();
    }

    #[test]
    #[should_panic(expected = "only one `TestWidgetContext` or app is allowed per thread")]
    pub fn app_and_test_ctx() {
        let _a = App::default().run_headless();
        let _b = TestWidgetContext::new();
    }

    #[test]
    #[should_panic(expected = "only one app or `TestWidgetContext` is allowed per thread")]
    pub fn test_ctx_and_app() {
        let _a = TestWidgetContext::new();
        let _b = App::default().run_headless();
    }
}

#[cfg(debug_assertions)]
struct DebugLogger;

#[cfg(debug_assertions)]
impl DebugLogger {
    fn init() {
        if log::set_logger(&DebugLogger).is_ok() {
            log::set_max_level(log::LevelFilter::Warn);
        }
    }
}

#[cfg(debug_assertions)]
impl log::Log for DebugLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= log::Level::Warn
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            use colored::*;
            match record.metadata().level() {
                log::Level::Error => {
                    eprintln!("{}: [{}] {}", "error".bright_red().bold(), record.target(), record.args())
                }
                log::Level::Warn => {
                    eprintln!("{}: [{}] {}", "warn".bright_yellow().bold(), record.target(), record.args())
                }
                _ => {}
            }
        }
    }

    fn flush(&self) {}
}
