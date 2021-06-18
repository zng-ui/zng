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
pub use glutin::event_loop::ControlFlow;
use std::future::Future;
use std::task::{Wake, Waker};
use std::{
    any::{type_name, TypeId},
    fmt,
    sync::atomic::AtomicBool,
    sync::{Arc, Mutex},
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
        write!(f, "AppHasShutdown<{}>", type_name::<T>())
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
    fn init(&mut self, ctx: &mut AppContext) {
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
    fn init_boxed(&mut self, ctx: &mut AppContext);
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

    fn init_boxed(&mut self, ctx: &mut AppContext) {
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

    fn init(&mut self, ctx: &mut AppContext) {
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
/// This service is registered for all apps.
#[derive(Service)]
pub struct AppProcess {
    shutdown_requests: Option<ResponderVar<ShutdownCancelled>>,
    update_sender: AppEventSender,
}
impl AppProcess {
    fn new(update_sender: AppEventSender) -> Self {
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
            let _ = self.update_sender.send_update();
            response
        }
    }

    fn take_requests(&mut self) -> Option<ResponderVar<ShutdownCancelled>> {
        self.shutdown_requests.take()
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

        let event_loop = glutin::event_loop::EventLoop::with_user_event();
        let sender = AppEventSender::from_winit(event_loop.create_proxy());
        let window_target = WindowTarget::from_winit(&event_loop);

        let mut app = RunningApp::start(self.extensions, sender, window_target);

        start(&mut app.ctx(window_target));

        app.run_headed(event_loop)
    }

    /// Runs the application with an async `start` function.
    ///
    /// The `start` future is executed in the app thread only (the main thread), it runs up to the first `await` immediately
    /// and subsequent polls happen in app updates, it is async but not parallel. You can use [`task`](crate::task)
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

        let (sender, receiver) = AppEventSender::new_headless();

        let app = RunningApp::start(self.extensions.boxed(), sender, WindowTarget::headless());

        HeadlessApp {
            app_event_receiver: receiver,
            app,

            #[cfg(feature = "app_profiler")]
            _pf: profile_scope,
        }
    }

    /// Start a [`RunningApp`] that will be controlled by an external event loop.
    pub fn run_client(self, app_event_sender: AppEventSender, window_target: WindowTarget) -> RunningApp<E> {
        RunningApp::start(self.extensions, app_event_sender, window_target)
    }

    /// Start a [`RunningApp`] that will be controlled by an external event loop, the app extensions
    /// are boxed making the app type more manageable.
    pub fn run_client_boxed(self, app_event_sender: AppEventSender, window_target: WindowTarget) -> RunningApp<Box<dyn AppExtensionBoxed>> {
        RunningApp::start(self.extensions.boxed(), app_event_sender, window_target)
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
    fn start(mut extensions: E, event_sender: AppEventSender, window_target: WindowTarget) -> Self {
        if App::is_running() {
            if cfg!(any(test, doc, feature = "pub_test")) {
                panic!("only one app or `TestWidgetContext` is allowed per thread")
            } else {
                panic!("only one app is allowed per thread")
            }
        }

        let mut owned_ctx = OwnedAppContext::instance(event_sender);

        let mut ctx = owned_ctx.borrow(window_target);
        ctx.services.register(AppProcess::new(ctx.updates.sender()));
        extensions.init(&mut ctx);

        RunningApp {
            device_events: extensions.enable_device_events(),
            extensions,
            owned_ctx,
            maybe_has_updates: true,
            wake_time: None,
            exiting: false,
        }
    }

    fn run_headed(self, event_loop: glutin::event_loop::EventLoop<AppEvent>) -> ! {
        let mut app = Some(self);
        event_loop.run(move |event, window_target, control_flow| {
            let window_target = WindowTarget::from_winit(window_target);

            if let GEvent::LoopDestroyed = &event {
                app.take().unwrap().shutdown(window_target);
                return;
            }

            let app = app.as_mut().expect("app already shutdown");

            match event {
                GEvent::NewEvents(c) => {
                    if let glutin::event::StartCause::ResumeTimeReached { .. } = c {
                        app.wait_until_elapsed();
                    }
                }
                GEvent::WindowEvent { window_id, event } => app.window_event(window_target, window_id.into(), &event),
                GEvent::DeviceEvent { device_id, event } => app.device_event(window_target, device_id, &event),
                GEvent::UserEvent(app_event) => app.app_event(window_target, app_event),
                GEvent::Suspended => app.suspended(window_target),
                GEvent::Resumed => app.resumed(window_target),
                GEvent::MainEventsCleared => {
                    *control_flow = app.update(window_target, &mut ());
                }
                GEvent::RedrawRequested(window_id) => app.redraw_requested(window_target, window_id.into()),
                GEvent::RedrawEventsCleared => {}
                GEvent::LoopDestroyed => unreachable!(),
            }
        })
    }

    /// Exclusive borrow the app context.
    pub fn ctx<'a, 'w>(&'a mut self, window_target: WindowTarget<'w>) -> AppContext<'a, 'w> {
        self.maybe_has_updates = true;
        self.owned_ctx.borrow(window_target)
    }

    /// Event loop has awakened because [`WaitUntil`](ControlFlow::WaitUntil) was requested.
    pub fn wait_until_elapsed(&mut self) {
        self.maybe_has_updates = true;
    }

    /// Process window event.
    pub fn window_event(&mut self, window_target: WindowTarget, window_id: WindowId, event: &WindowEvent) {
        let mut ctx = self.owned_ctx.borrow(window_target);
        self.extensions.window_event(&mut ctx, window_id, event);
        self.maybe_has_updates = true;
    }

    /// Process device event.
    pub fn device_event(&mut self, window_target: WindowTarget, device_id: DeviceId, event: &DeviceEvent) {
        if self.device_events {
            let mut ctx = self.owned_ctx.borrow(window_target);
            self.extensions.device_event(&mut ctx, device_id, event);
            self.maybe_has_updates = true;
        }
    }

    /// Process an [`AppEvent`].
    pub fn app_event(&mut self, window_target: WindowTarget, app_event: AppEvent) {
        match app_event.0 {
            AppEventData::NewFrameReady(window_id) => {
                let mut ctx = self.owned_ctx.borrow(window_target);
                self.extensions.new_frame_ready(&mut ctx, window_id);
            }
            AppEventData::Update => {
                self.owned_ctx.borrow(window_target).updates.update();
            }
            AppEventData::Event(e) => {
                self.owned_ctx.borrow(window_target).events.notify_app_event(e);
            }
            AppEventData::Var => {
                self.owned_ctx.borrow(window_target).vars.receive_sended_modify();
            }
        }
        self.maybe_has_updates = true;
    }

    /// Process application suspension.
    pub fn suspended(&mut self, _event_loop: WindowTarget) {
        log::error!(target: "app", "TODO suspended");
    }

    /// Process application resume from suspension.
    pub fn resumed(&mut self, _event_loop: WindowTarget) {
        log::error!(target: "app", "TODO resumed");
    }

    /// Does pending event and updates until there is no more updates generated, then returns
    /// [`WaitUntil`](ControlFlow::WaitUntil) are timers running or returns [`Wait`](ControlFlow::WaitUntil)
    /// if there aren't.
    ///
    /// You can use an [`AppUpdateObserver`] to watch all of these actions or pass `&mut ()` as a NOP observer.
    pub fn update<O: AppUpdateObserver>(&mut self, window_target: WindowTarget, observer: &mut O) -> ControlFlow {
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
                    let mut ctx = self.owned_ctx.borrow(window_target);

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

                    let mut ctx = self.owned_ctx.borrow(window_target);

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
    pub fn redraw_requested(&mut self, window_target: WindowTarget, window_id: WindowId) {
        let mut ctx = self.owned_ctx.borrow(window_target);
        self.extensions.redraw_requested(&mut ctx, window_id);
    }

    /// De-initializes extensions and drops.
    pub fn shutdown(mut self, window_target: WindowTarget) {
        let mut ctx = self.owned_ctx.borrow(window_target);
        self.extensions.deinit(&mut ctx);
    }
}

/// A headless app controller.
///
/// Headless apps don't cause external side-effects like visible windows and don't listen to system events.
/// They can be used for creating apps like a command line app that renders widgets, or for creating integration tests.
pub struct HeadlessApp {
    app_event_receiver: flume::Receiver<AppEvent>,
    app: RunningApp<Box<dyn AppExtensionBoxed>>,
    #[cfg(feature = "app_profiler")]
    _pf: ProfileScope,
}
impl HeadlessApp {
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
        self.app_state().get::<HeadlessRendererEnabledKey>().copied().unwrap_or_default()
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
    /// This sets the [`HeadlessRendererEnabledKey`] state in the [app state](Self::app_state).
    pub fn enable_renderer(&mut self, enabled: bool) {
        self.app_state_mut().set::<HeadlessRendererEnabledKey>(enabled);
    }

    /// Notifies extensions of a [device event](DeviceEvent).
    pub fn device_event(&mut self, device_id: DeviceId, event: &DeviceEvent) {
        profile_scope!("headless_app::device_event");
        self.app.device_event(WindowTarget::headless(), device_id, event);
    }

    /// Notifies extensions of a [window event](WindowEvent).
    pub fn window_event(&mut self, window_id: WindowId, event: &WindowEvent) {
        profile_scope!("headless_app::device_event");
        self.app.window_event(WindowTarget::headless(), window_id, event);
    }

    /// Borrows the app context.
    pub fn ctx<'a>(&'a mut self) -> AppContext<'a, 'static> {
        profile_scope!("headless_app::with_context");
        self.app.ctx(WindowTarget::headless())
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
        if wait_app_event {
            if let Ok(event) = self.app_event_receiver.recv() {
                self.app.app_event(WindowTarget::headless(), event);
            }
        }
        for event in self.app_event_receiver.try_iter() {
            self.app.app_event(WindowTarget::headless(), event);
        }

        let r = self.app.update(WindowTarget::headless(), observer);
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
    fn init(&mut self, ctx: &mut AppContext) {
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
    fn init(&mut self, ctx: &mut AppContext) {
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

/// Raw event for [`RunningApp`].
#[derive(Debug)]
pub struct AppEvent(AppEventData);
#[derive(Debug)]
enum AppEventData {
    /// Notify [`Events`](crate::var::Events).
    Event(crate::event::BoxedSendEventUpdate),
    /// Notify [`Vars`](crate::var::Vars).
    Var,
    /// Call [`AppExtension::new_frame_ready`].
    NewFrameReady(WindowId),
    /// Do an update cycle.
    Update,
}

/// An [`AppEvent`] sender that can awake apps and insert events into their loop.
#[derive(Clone)]
pub struct AppEventSender(AppEventSenderData);
impl AppEventSender {
    /// New headed event loop connected to an `winit` loop with [`AppEvent`] as the event type.
    pub fn from_winit(el: glutin::event_loop::EventLoopProxy<AppEvent>) -> Self {
        AppEventSender(AppEventSenderData::Winit(el))
    }

    /// New headed event loop connected to an external event loop using an [adapter](AppEventSenderAdapter).
    pub fn from_adapter(el: Box<dyn AppEventSenderAdapter>) -> Self {
        AppEventSender(AppEventSenderData::Adapter(el))
    }

    /// If the app is running in headless mode.
    pub fn is_headless(&self) -> bool {
        matches!(&self.0, AppEventSenderData::Headless(_))
    }

    pub(crate) fn new_headless() -> (Self, flume::Receiver<AppEvent>) {
        let (send, rcv) = flume::unbounded();
        (AppEventSender(AppEventSenderData::Headless(send)), rcv)
    }

    #[inline(always)]
    fn send_app_event(&self, event: AppEvent) -> Result<(), AppShutdown<AppEvent>> {
        match &self.0 {
            AppEventSenderData::Winit(w) => w.send_event(event)?,
            AppEventSenderData::Headless(s) => s.send(event)?,
            AppEventSenderData::Adapter(a) => a.send_event(event)?,
        }
        Ok(())
    }

    /// Causes an update cycle to happen in the app.
    #[inline]
    pub fn send_update(&self) -> Result<(), AppShutdown<()>> {
        self.send_app_event(AppEvent(AppEventData::Update)).map_err(|_| AppShutdown(()))
    }

    /// Causes a call to [`AppExtension::new_frame_ready`].
    #[inline]
    pub fn send_new_frame_ready(&self, window_id: WindowId) -> Result<(), AppShutdown<WindowId>> {
        self.send_app_event(AppEvent(AppEventData::NewFrameReady(window_id)))
            .map_err(|_| AppShutdown(window_id))
    }

    /// [`VarSender`](crate::var::VarSender) util.
    #[inline]
    pub(crate) fn send_var(&self) -> Result<(), AppShutdown<()>> {
        self.send_app_event(AppEvent(AppEventData::Var)).map_err(|_| AppShutdown(()))
    }

    /// [`EventSender`](crate::event::EventSender) util.
    pub(crate) fn send_event(
        &self,
        event: crate::event::BoxedSendEventUpdate,
    ) -> Result<(), AppShutdown<crate::event::BoxedSendEventUpdate>> {
        self.send_app_event(AppEvent(AppEventData::Event(event))).map_err(|e| match e.0 .0 {
            AppEventData::Event(ev) => AppShutdown(ev),
            _ => unreachable!(),
        })
    }

    /// [`Waker`] that causes a [`send_update`](Self::send_update).
    pub fn waker(&self) -> Waker {
        let sync = match &self.0 {
            AppEventSenderData::Winit(el) => AppEventSenderDataSync::Winit(Mutex::new(el.clone())),
            AppEventSenderData::Headless(s) => AppEventSenderDataSync::Headless(s.clone()),
            AppEventSenderData::Adapter(a) => AppEventSenderDataSync::Adapter(Mutex::new(a.clone_boxed())),
        };
        Arc::new(sync).into()
    }
}
enum AppEventSenderData {
    Winit(glutin::event_loop::EventLoopProxy<AppEvent>),
    Headless(flume::Sender<AppEvent>),
    Adapter(Box<dyn AppEventSenderAdapter>),
}
impl Clone for AppEventSenderData {
    fn clone(&self) -> Self {
        match self {
            AppEventSenderData::Winit(el) => AppEventSenderData::Winit(el.clone()),
            AppEventSenderData::Headless(s) => AppEventSenderData::Headless(s.clone()),
            AppEventSenderData::Adapter(a) => AppEventSenderData::Adapter(a.clone_boxed()),
        }
    }
}
enum AppEventSenderDataSync {
    Winit(Mutex<glutin::event_loop::EventLoopProxy<AppEvent>>),
    Headless(flume::Sender<AppEvent>),
    Adapter(Mutex<Box<dyn AppEventSenderAdapter>>),
}
impl Wake for AppEventSenderDataSync {
    fn wake(self: Arc<Self>) {
        let update = AppEvent(AppEventData::Update);
        match &*self {
            AppEventSenderDataSync::Winit(m) => {
                let _ = match m.lock() {
                    Ok(el) => el.send_event(update),
                    Err(e) => e.into_inner().send_event(update),
                };
            }
            AppEventSenderDataSync::Headless(s) => {
                let _ = s.send(update);
            }
            AppEventSenderDataSync::Adapter(m) => {
                let _ = match m.lock() {
                    Ok(el) => el.send_event(update),
                    Err(e) => e.into_inner().send_event(update),
                };
            }
        }
    }
}

/// Represents an external event loop in a [`AppEventSender`].
///
/// The external event loop must be awaken from this, receive an [`AppEvent`] as pass it to the client app using
/// [`RunningApp::app_event`].
pub trait AppEventSenderAdapter: Send + 'static {
    /// Clone `self` and boxes it. The clone must send events to the same event loop as `self`.
    fn clone_boxed(&self) -> Box<dyn AppEventSenderAdapter>;

    /// Awake the app and insert the `event` into the loop,
    /// or return the [`AppShutdown`] error if the event loop has closed.
    fn send_event(&self, event: AppEvent) -> Result<(), AppShutdown<AppEvent>>;
}

/// Event loop window target for headed
#[derive(Clone, Copy)]
pub struct WindowTarget<'a>(WindowTargetData<'a>);
impl<'a> WindowTarget<'a> {
    /// Reference a headed event loop with [`AppEvent`] as the event type.
    ///
    /// **Note:** The [`winit::event_loop::EventLoop`](glutin::event_loop::EventLoop) type dereferences
    /// to the input for this method.
    pub fn from_winit(window_target: &'a glutin::event_loop::EventLoopWindowTarget<AppEvent>) -> WindowTarget<'a> {
        WindowTarget(WindowTargetData::Winit(window_target))
    }

    /// Reference an external event loop using an [adapter](WindowTargetAdapter).
    pub fn from_adapter(adapter: &'a dyn WindowTargetAdapter<'a>) -> WindowTarget<'a> {
        WindowTarget(WindowTargetData::Adapter(adapter))
    }

    fn headless() -> WindowTarget<'static> {
        WindowTarget(WindowTargetData::Headless)
    }
}

impl<'a> WindowTarget<'a> {
    /// Call [`glutin::ContextBuilder::build_windowed`] for headed event loops.
    ///
    /// # Errors
    ///
    /// The error can be any from the `glutin` builder or [`glutin::CreationError::NotSupported`] with the message
    /// `"cannot build `WindowedContext` in headless event loop"` if [`is_headless`](Self::is_headless).
    pub fn build_glutin_window(
        &self,
        context_builder: glutin::ContextBuilder<glutin::NotCurrent>,
        window_builder: glutin::window::WindowBuilder,
    ) -> Result<glutin::WindowedContext<glutin::NotCurrent>, glutin::CreationError> {
        match &self.0 {
            WindowTargetData::Winit(wt) => context_builder.build_windowed(window_builder, wt),
            WindowTargetData::Adapter(a) => a.build_glutin_window(context_builder, window_builder),
            WindowTargetData::Headless => Err(glutin::CreationError::NotSupported(
                "cannot build `WindowedContext` in headless event loop".to_owned(),
            )),
        }
    }

    /// Call [`winit::window::WindowBuilder::build`](glutin::window::WindowBuilder::build) for headed event loops.
    ///
    /// # Errors
    ///
    /// The error can be a [`glutin::CreationError::Window`] from the `winit` builder or [`glutin::CreationError::NotSupported`]
    /// if [`is_headless`](Self::is_headless)
    pub fn build_winit_window(
        &self,
        window_builder: glutin::window::WindowBuilder,
    ) -> Result<glutin::window::Window, glutin::CreationError> {
        match &self.0 {
            WindowTargetData::Winit(wt) => window_builder.build(wt).map_err(glutin::CreationError::Window),
            WindowTargetData::Adapter(a) => a.build_winit_window(window_builder).map_err(glutin::CreationError::Window),
            WindowTargetData::Headless => Err(glutin::CreationError::NotSupported(
                "cannot build `WindowedContext` in headless event loop".to_owned(),
            )),
        }
    }

    /// If this is a dummy window target for a headless app.
    ///
    /// If `true` both build methods will always return [`glutin::CreationError::NotSupported`].
    pub fn is_headless(&self) -> bool {
        matches!(&self.0, &WindowTargetData::Headless)
    }
}
#[derive(Clone, Copy)]
enum WindowTargetData<'a> {
    Winit(&'a glutin::event_loop::EventLoopWindowTarget<AppEvent>),
    Headless,
    Adapter(&'a dyn WindowTargetAdapter<'a>),
}

/// Represents an external event loop in a [`WindowTarget`].
pub trait WindowTargetAdapter<'a> {
    /// Call [`glutin::ContextBuilder::build_windowed`].
    fn build_glutin_window(
        &self,
        context_builder: glutin::ContextBuilder<glutin::NotCurrent>,
        window_builder: glutin::window::WindowBuilder,
    ) -> Result<glutin::WindowedContext<glutin::NotCurrent>, glutin::CreationError>;

    /// Call [`winit::window::WindowBuilder::build`](glutin::window::WindowBuilder::build).
    fn build_winit_window(&self, window_builder: glutin::window::WindowBuilder) -> Result<glutin::window::Window, glutin::error::OsError>;
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

        let render_enabled = app.app_state().get::<HeadlessRendererEnabledKey>().copied().unwrap_or_default();

        assert!(!render_enabled);

        app.update(false);
    }

    #[test]
    pub fn new_window_with_render() {
        let mut app = App::default().run_headless();
        app.enable_renderer(true);
        assert!(app.renderer_enabled());

        let render_enabled = app.app_state().get::<HeadlessRendererEnabledKey>().copied().unwrap_or_default();

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
