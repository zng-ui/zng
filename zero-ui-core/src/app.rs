//! App startup and app extension API.

use crate::context::*;
use crate::crate_util::PanicPayload;
use crate::event::{cancelable_event_args, AnyEventUpdate, EventUpdate, EventUpdateArgs, Events};
use crate::image::ImageManager;
use crate::profiler::*;
use crate::render::FrameId;
use crate::timer::Timers;
use crate::var::{response_var, ResponderVar, ResponseVar, Vars};
use crate::{
    focus::FocusManager,
    gesture::GestureManager,
    keyboard::KeyboardManager,
    mouse::MouseManager,
    service::Service,
    text::FontManager,
    window::{WindowId, WindowManager},
};

use linear_map::LinearMap;
use once_cell::sync::Lazy;
use std::future::Future;
use std::path::PathBuf;
use std::sync::Arc;
use std::task::Waker;
use std::{
    any::{type_name, TypeId},
    fmt,
    time::Instant,
};

/// Call this function before anything else in the app `main` function.
///
/// If the process is started with the right environment configuration this function
/// high-jacks the process and turns it into a *View Process*, never returning.
///
/// This function does nothing if the *View Process* environment is not set, you can safely call it more then once.
/// The [`App::default`] and [`App::blank`] methods also call this function, so if the first line of the `main` is
/// `App::default` you don't need to explicitly call the function.
///
/// # Examples
///
/// Calling the function as early as possible stops the process from doing things that it should only do in the app process:
///
/// ```no_run
/// # use zero_ui_core::app::*;
/// # fn do_app_process_init_things() { }
/// fn main() {
///     init_view_process();
///
///     do_app_process_init_things();
///
///     App::default().run(|ctx| {
///         todo!()
///     });
/// }
/// ```
///
/// But [`App::default`] also calls this function so you can omit if the app creation is the first line of code:
///
/// ```no_run
/// # use zero_ui_core::app::*;
/// # fn do_app_process_init_things() { }
/// fn main() {
///     App::default().run(|ctx| {
///         do_app_process_init_things();
///
///         todo!()
///     });
/// }
/// ```
///
/// Just be careful more code does not get added before `App::default` later.
#[inline]
pub fn init_view_process() {
    zero_ui_vp::init_view_process();
}

/// Run both View and App in the same process.
///
/// This function must be called in the main thread, it initializes the View and calls `run_app`
/// in a new thread to initialize the App.
///
/// The primary use of this function is debugging the view process code, just move your main function code to inside the `run_app` and
/// start debugging. You can also use this to trade-off memory use for more risk of fatal crashes.
///
/// # Examples
///
/// A setup that runs the app in a single process in debug builds, and split processes in release builds.
///
/// ```no_run
/// # use zero_ui_core::app::*;
/// #
/// fn main() {
///     if cfg!(debug_assertions) {
///         run_same_process(app_main);
///     } else {
///         init_view_process();
///         app_main();
///     }
/// }
///
/// fn app_main() {
///     App::default().run(|ctx| {
///         todo!()
///     });
/// }
/// ```
#[inline]
pub fn run_same_process(run_app: impl FnOnce() + Send + 'static) -> ! {
    zero_ui_vp::run_same_process(run_app)
}

/// Error when the app connected to a sender/receiver channel has shutdown.
///
/// Contains the value that could not be send or `()` for receiver errors.
pub struct AppShutdown<T>(pub T);
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
#[cfg_attr(doc_nightly, doc(notable_trait))]
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

    /// If the application should notify raw device events.
    ///
    /// Device events are raw events not targeting any window, like a mouse move on any part of the screen.
    /// They tend to be high-volume events so there is a performance cost to activating this. Note that if
    /// this is `false` you still get the mouse move over windows of the app.
    ///
    /// This is called zero or one times before [`init`](Self::init).
    ///
    /// Returns `false` by default.
    #[inline]
    fn enable_device_events(&self) -> bool {
        false
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

    /// Called when a new frame is ready to be inspected.
    #[inline]
    fn new_frame(&mut self, ctx: &mut AppContext, window_id: WindowId, frame_id: FrameId) {
        let _ = (ctx, window_id, frame_id);
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
    fn update_preview_boxed(&mut self, ctx: &mut AppContext);
    fn update_ui_boxed(&mut self, ctx: &mut AppContext);
    fn update_boxed(&mut self, ctx: &mut AppContext);
    fn event_preview_boxed(&mut self, ctx: &mut AppContext, args: &AnyEventUpdate);
    fn event_ui_boxed(&mut self, ctx: &mut AppContext, args: &AnyEventUpdate);
    fn event_boxed(&mut self, ctx: &mut AppContext, args: &AnyEventUpdate);
    fn update_display_boxed(&mut self, ctx: &mut AppContext, update: UpdateDisplayRequest);
    fn new_frame_boxed(&mut self, ctx: &mut AppContext, window_id: WindowId, frame_id: FrameId);
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

    fn new_frame_boxed(&mut self, ctx: &mut AppContext, window_id: WindowId, frame_id: FrameId) {
        self.new_frame(ctx, window_id, frame_id);
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

    fn new_frame(&mut self, ctx: &mut AppContext, window_id: WindowId, frame_id: FrameId) {
        self.as_mut().new_frame_boxed(ctx, window_id, frame_id);
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
/// # View Process
///
/// The [`init_view_process`] function must be called before all other code in the app `main` function when
/// creating an app with renderer. If the process is started with the right environment configuration this function
/// high-jacks the process and turns it into a *View Process*, never returning.
///
/// Note that [`init_view_process`] does nothing if the *View Process* environment is not set, you can safely call it more then once.
/// The [`blank`] and [`default`] methods also call this function, so if the first line of the `main` is `App::default` you don't
/// need to explicitly call the function.
///
/// # Debug Log
///
/// In debug builds, `App` sets a [`logger`](log) that prints warnings and errors to `stderr`
/// if no logger was registered before the call to [`blank`] or [`default`].
///
/// [`blank`]: App::blank
/// [`default`]: App::default
pub struct App;

impl App {
    /// If an app is already running in the current thread.
    ///
    /// Only a single app is allowed per-thread.
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
        init_view_process();
        AppExtended {
            extensions: (),
            view_process_exe: None,
        }
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
    /// * [ImageManager]
    #[inline]
    pub fn default() -> AppExtended<impl AppExtension> {
        App::blank()
            .extend(MouseManager::default())
            .extend(KeyboardManager::default())
            .extend(GestureManager::default())
            .extend(WindowManager::default())
            .extend(FontManager::default())
            .extend(FocusManager::default())
            .extend(ImageManager::default())
    }
}

// In debug mode we use dynamic dispatch to reduce the number of types
// in the stack-trace and compile more quickly.
#[cfg(debug_assertions)]
impl App {
    /// Application without any extension and without device events.
    pub fn blank() -> AppExtended<Vec<Box<dyn AppExtensionBoxed>>> {
        init_view_process();
        DebugLogger::init();
        AppExtended {
            extensions: vec![],
            view_process_exe: None,
        }
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
    /// * [ImageManager]
    pub fn default() -> AppExtended<Vec<Box<dyn AppExtensionBoxed>>> {
        App::blank()
            .extend(MouseManager::default())
            .extend(KeyboardManager::default())
            .extend(GestureManager::default())
            .extend(WindowManager::default())
            .extend(FontManager::default())
            .extend(FocusManager::default())
            .extend(ImageManager::default())
    }
}

/// Application with extensions.
pub struct AppExtended<E: AppExtension> {
    extensions: E,
    view_process_exe: Option<PathBuf>,
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
    ///
    /// * `"app already extended with `{}`"` when the app is already [`extended_with`](AppExtended::extended_with) the
    /// extension type.
    #[inline]
    pub fn extend<F: AppExtension>(mut self, extension: F) -> AppExtended<Vec<Box<dyn AppExtensionBoxed>>> {
        if self.extended_with::<F>() {
            panic!("app already extended with `{}`", type_name::<F>())
        }

        self.extensions.push(extension.boxed());

        self
    }

    /// If the application should notify raw device events.
    ///
    /// Device events are raw events not targeting any window, like a mouse move on any part of the screen.
    /// They tend to be high-volume events so there is a performance cost to activating this. Note that if
    /// this is `false` you still get the mouse move over windows of the app.
    pub fn enable_device_events(self) -> AppExtended<Vec<Box<dyn AppExtensionBoxed>>> {
        struct EnableDeviceEvents;
        impl AppExtension for EnableDeviceEvents {
            fn enable_device_events(&self) -> bool {
                true
            }
        }
        self.extend(EnableDeviceEvents)
    }
}

#[cfg(not(debug_assertions))]
impl<E: AppExtension> AppExtended<E> {
    /// Includes an application extension.
    ///
    /// # Panics
    ///
    /// * `"app already extended with `{}`"` when the app is already [`extended_with`](AppExtended::extended_with) the
    /// extension type.
    #[inline]
    pub fn extend<F: AppExtension>(self, extension: F) -> AppExtended<impl AppExtension> {
        if self.extended_with::<F>() {
            panic!("app already extended with `{}`", type_name::<F>())
        }
        AppExtended {
            extensions: (self.extensions, extension),
            view_process_exe: self.view_process_exe,
        }
    }

    /// If the application should notify raw device events.
    ///
    /// Device events are raw events not targeting any window, like a mouse move on any part of the screen.
    /// They tend to be high-volume events so there is a performance cost to activating this. Note that if
    /// this is `false` you still get the mouse move over windows of the app.
    pub fn enable_device_events(self) -> AppExtended<impl AppExtension> {
        struct EnableDeviceEvents;
        impl AppExtension for EnableDeviceEvents {
            fn enable_device_events(&self) -> bool {
                true
            }
        }
        self.extend(EnableDeviceEvents)
    }
}
impl<E: AppExtension> AppExtended<E> {
    /// Gets if the application is already extended with the extension type.
    #[inline]
    pub fn extended_with<F: AppExtension>(&self) -> bool {
        self.extensions.is_or_contain(TypeId::of::<F>())
    }

    /// Set the path to the executable for the *View Process*.
    ///
    /// By the default the current executable is started again as a *View Process*, you use
    /// two executables instead, by setting this value.
    ///
    /// Note that the [`init_view_process`] function must be called in the `view_process_exe` and both
    /// executables must be build using the same exact [`VERSION`].
    ///
    /// [`VERSION`]: zero_ui_vp::VERSION  
    pub fn view_process_exe(mut self, view_process_exe: impl Into<PathBuf>) -> Self {
        self.view_process_exe = Some(view_process_exe.into());
        self
    }

    /// Runs the application calling `start` once at the beginning.
    ///
    /// This method only returns when the app has shutdown.
    ///
    /// # Panics
    ///
    /// Panics if not called by the main thread. This means you cannot run an app in unit tests, use a headless
    /// app without renderer for that. The main thread is required by some operating systems and OpenGL.
    pub fn run(self, start: impl FnOnce(&mut AppContext)) {
        #[cfg(feature = "app_profiler")]
        register_thread_with_profiler();

        profile_scope!("app::run");

        let mut app = RunningApp::start(self.extensions, true, true, self.view_process_exe);

        start(&mut app.ctx());

        app.run_headed();
    }

    /// Initializes extensions in headless mode and returns an [`HeadlessApp`].
    ///
    /// If `with_renderer` is `true` spawns a renderer process for headless rendering. See [`HeadlessApp::renderer_enabled`]
    /// for more details.
    ///
    /// # Tests
    ///
    /// If called in a test (`cfg(test)`) this blocks until no other instance of [`HeadlessApp`] and
    /// [`TestWidgetContext`] are running in the current thread.
    pub fn run_headless(self, with_renderer: bool) -> HeadlessApp {
        #[cfg(feature = "app_profiler")]
        let profile_scope = {
            register_thread_with_profiler();
            ProfileScope::new("app::run_headless")
        };

        let app = RunningApp::start(self.extensions.boxed(), false, with_renderer, self.view_process_exe);

        HeadlessApp {
            app,

            #[cfg(feature = "app_profiler")]
            _pf: profile_scope,
        }
    }
}

/// Represents a running app controlled by an external event loop.
pub struct RunningApp<E: AppExtension> {
    extensions: E,
    device_events: bool,
    owned_ctx: OwnedAppContext,
    receiver: flume::Receiver<AppEvent>,

    // need to probe context to see if there are updates.
    maybe_has_updates: bool,
    // WaitUntil time.
    wake_time: Option<Instant>,

    // shutdown was requested.
    exiting: bool,
}
impl<E: AppExtension> RunningApp<E> {
    fn start(mut extensions: E, is_headed: bool, with_renderer: bool, view_process_exe: Option<PathBuf>) -> Self {
        if App::is_running() {
            if cfg!(any(test, doc, feature = "test_util")) {
                panic!("only one app or `TestWidgetContext` is allowed per thread")
            } else {
                panic!("only one app is allowed per thread")
            }
        }

        let (sender, receiver) = AppEventSender::new();

        let mut owned_ctx = OwnedAppContext::instance(sender);

        let mut ctx = owned_ctx.borrow();
        ctx.services.register(AppProcess::new(ctx.updates.sender()));

        let device_events = extensions.enable_device_events();

        if is_headed {
            debug_assert!(with_renderer);

            let view_evs_sender = ctx.updates.sender();
            let view_app = view_process::ViewProcess::start(view_process_exe, device_events, false, move |ev| {
                view_evs_sender.send_view_event(ev).unwrap()
            });
            ctx.services.register(view_app);
        } else if with_renderer {
            let renderer = view_process::ViewProcess::start(view_process_exe, false, true, |_| unreachable!());
            ctx.services.register(renderer);
        }

        extensions.init(&mut ctx);

        RunningApp {
            device_events,
            extensions,
            owned_ctx,
            receiver,
            maybe_has_updates: true,
            wake_time: None,
            exiting: false,
        }
    }

    fn run_headed(mut self) {
        loop {
            match self.poll(&mut ()) {
                ControlFlow::Wait => {} // poll waits
                ControlFlow::Exit => break,
                ControlFlow::Poll => unreachable!(),
            }
        }
    }

    /// If device events are enabled in this app.
    #[inline]
    pub fn device_events(&self) -> bool {
        self.device_events
    }

    /// Exclusive borrow the app context.
    pub fn ctx(&mut self) -> AppContext {
        self.maybe_has_updates = true;
        self.owned_ctx.borrow()
    }

    /// Borrow the [`Vars`] only.
    pub fn vars(&self) -> &Vars {
        self.owned_ctx.vars()
    }

    /// Notify an event directly to the app extensions.
    pub fn notify_event<Ev: crate::event::Event>(&mut self, _event: Ev, args: Ev::Args) {
        let update = EventUpdate::<Ev>(args);
        let mut ctx = self.owned_ctx.borrow();
        self.extensions.event_preview(&mut ctx, &update);
        self.extensions.event_ui(&mut ctx, &update);
        self.extensions.event(&mut ctx, &update);
        self.maybe_has_updates = true;
    }

    fn window_id(&mut self, id: zero_ui_vp::WinId) -> WindowId {
        self.ctx()
            .services
            .req::<view_process::ViewProcess>()
            .window_id(id)
            .expect("unknown window id")
    }

    fn device_id(&mut self, id: zero_ui_vp::DevId) -> DeviceId {
        self.ctx().services.req::<view_process::ViewProcess>().device_id(id)
    }

    /// Repeatedly sleeps-waits for app events until the control flow changes to something other the [`Poll`].
    ///
    /// This method also manages timers, awaking when a timer deadline elapses and causing an update cycle.
    ///
    /// [`Poll`]: ControlFlow::Poll
    #[inline]
    pub fn poll<O: AppEventObserver>(&mut self, observer: &mut O) -> ControlFlow {
        let mut flow = ControlFlow::Poll;
        while let ControlFlow::Poll = flow {
            if let Some(timer) = self.wake_time {
                flow = match self.receiver.recv_deadline(timer) {
                    Ok(ev) => self.app_event(ev, observer),
                    Err(e) => match e {
                        flume::RecvTimeoutError::Timeout => self.update(observer),
                        flume::RecvTimeoutError::Disconnected => panic!("app events channel disconnected"),
                    },
                }
            } else {
                let ev = self.receiver.recv().expect("app events channel disconnected");
                flow = self.app_event(ev, observer);
            }
        }
        flow
    }

    /// Try to receive app events until the control flow changes to something other the [`Poll`], if
    /// there is no app event in the channel returns tries an update cycle.
    ///
    /// This method does not manages timers, you can probe [`wake_time`] to get the next timer deadline.
    ///
    /// [`Poll`]: ControlFlow::Poll
    /// [`wake_time`]: RunningApp::wake_time
    pub fn try_poll<O: AppEventObserver>(&mut self, observer: &mut O) -> ControlFlow {
        let mut flow = ControlFlow::Poll;
        while let ControlFlow::Poll = flow {
            flow = match self.receiver.try_recv() {
                Ok(ev) => self.app_event(ev, observer),
                Err(flume::TryRecvError::Empty) => self.update(observer),
                Err(e) => panic!("{:?}", e),
            }
        }
        flow
    }

    /// Next timer deadline.
    #[inline]
    pub fn wake_time(&self) -> Option<Instant> {
        self.wake_time
    }

    /// Process an [`AppEvent`].
    fn app_event<O: AppEventObserver>(&mut self, app_event: AppEvent, observer: &mut O) -> ControlFlow {
        self.maybe_has_updates = true;

        match app_event {
            AppEvent::ViewEvent(ev) => {
                return self.view_event(ev, observer);
            }
            AppEvent::Update => self.owned_ctx.borrow().updates.update(),
            AppEvent::Event(e) => {
                self.owned_ctx.borrow().events.notify_app_event(e);
            }
            AppEvent::Var => {
                self.owned_ctx.borrow().vars.receive_sended_modify();
            }
            AppEvent::ResumeUnwind(p) => std::panic::resume_unwind(p),
        }

        self.update(observer)
    }

    /// Process a View Process event.
    ///
    /// Does `update` on `EventsCleared`.
    fn view_event<O: AppEventObserver>(&mut self, ev: zero_ui_vp::Ev, observer: &mut O) -> ControlFlow {
        use raw_device_events::*;
        use raw_events::*;

        match ev {
            zero_ui_vp::Ev::EventsCleared => {
                return self.update(observer);
            }
            zero_ui_vp::Ev::WindowResized(w_id, size) => {
                let args = RawWindowResizedArgs::now(self.window_id(w_id), size);
                self.notify_event(RawWindowResizedEvent, args);
            }
            zero_ui_vp::Ev::WindowMoved(w_id, pos) => {
                let args = RawWindowMovedArgs::now(self.window_id(w_id), pos);
                self.notify_event(RawWindowMovedEvent, args);
            }
            zero_ui_vp::Ev::DroppedFile(w_id, file) => {
                let args = RawDroppedFileArgs::now(self.window_id(w_id), file);
                self.notify_event(RawDroppedFileEvent, args);
            }
            zero_ui_vp::Ev::HoveredFile(w_id, file) => {
                let args = RawHoveredFileArgs::now(self.window_id(w_id), file);
                self.notify_event(RawHoveredFileEvent, args);
            }
            zero_ui_vp::Ev::HoveredFileCancelled(w_id) => {
                let args = RawHoveredFileCancelledArgs::now(self.window_id(w_id));
                self.notify_event(RawHoveredFileCancelledEvent, args);
            }
            zero_ui_vp::Ev::ReceivedCharacter(w_id, c) => {
                let args = RawCharInputArgs::now(self.window_id(w_id), c);
                self.notify_event(RawCharInputEvent, args);
            }
            zero_ui_vp::Ev::Focused(w_id, focused) => {
                let args = RawWindowFocusArgs::now(self.window_id(w_id), focused);
                self.notify_event(RawWindowFocusEvent, args);
            }
            zero_ui_vp::Ev::KeyboardInput(w_id, d_id, input) => {
                let args = RawKeyInputArgs::now(
                    self.window_id(w_id),
                    self.device_id(d_id),
                    input.scancode,
                    input.state,
                    input.virtual_keycode.map(Into::into),
                );
                self.notify_event(RawKeyInputEvent, args);
            }
            zero_ui_vp::Ev::ModifiersChanged(w_id, state) => {
                let args = RawModifiersChangedArgs::now(self.window_id(w_id), state);
                self.notify_event(RawModifiersChangedEvent, args);
            }
            zero_ui_vp::Ev::CursorMoved(w_id, d_id, pos) => {
                let args = RawCursorMovedArgs::now(self.window_id(w_id), self.device_id(d_id), pos);
                self.notify_event(RawCursorMovedEvent, args);
            }
            zero_ui_vp::Ev::CursorEntered(w_id, d_id) => {
                let args = RawCursorArgs::now(self.window_id(w_id), self.device_id(d_id));
                self.notify_event(RawCursorEnteredEvent, args);
            }
            zero_ui_vp::Ev::CursorLeft(w_id, d_id) => {
                let args = RawCursorArgs::now(self.window_id(w_id), self.device_id(d_id));
                self.notify_event(RawCursorLeftEvent, args);
            }
            zero_ui_vp::Ev::MouseWheel(w_id, d_id, delta, phase) => {
                // TODO
                let _ = (delta, phase);
                let args = RawMouseWheelArgs::now(self.window_id(w_id), self.device_id(d_id));
                self.notify_event(RawMouseWheelEvent, args);
            }
            zero_ui_vp::Ev::MouseInput(w_id, d_id, state, button) => {
                let args = RawMouseInputArgs::now(self.window_id(w_id), self.device_id(d_id), state, button);
                self.notify_event(RawMouseInputEvent, args);
            }
            zero_ui_vp::Ev::TouchpadPressure(w_id, d_id, pressure, stage) => {
                // TODO
                let _ = (pressure, stage);
                let args = RawTouchpadPressureArgs::now(self.window_id(w_id), self.device_id(d_id));
                self.notify_event(RawTouchpadPressureEvent, args);
            }
            zero_ui_vp::Ev::AxisMotion(w_id, d_id, axis, value) => {
                let args = RawAxisMotionArgs::now(self.window_id(w_id), self.device_id(d_id), axis, value);
                self.notify_event(RawAxisMotionEvent, args);
            }
            zero_ui_vp::Ev::Touch(w_id, d_id, phase, pos, force, finger_id) => {
                // TODO
                let _ = (phase, pos, force, finger_id);
                let args = RawTouchArgs::now(self.window_id(w_id), self.device_id(d_id));
                self.notify_event(RawTouchEvent, args);
            }
            zero_ui_vp::Ev::ScaleFactorChanged(w_id, scale) => {
                let args = RawWindowScaleFactorChangedArgs::now(self.window_id(w_id), scale);
                self.notify_event(RawWindowScaleFactorChangedEvent, args);
            }
            zero_ui_vp::Ev::MonitorsChanged(monitors) => {
                let view = self.ctx().services.req::<view_process::ViewProcess>();
                let monitors: Vec<_> = monitors.into_iter().map(|(id, info)| (view.monitor_id(id), info)).collect();
                let args = RawMonitorsChangedArgs::now(monitors);
                self.notify_event(RawMonitorsChangedEvent, args);
            }
            zero_ui_vp::Ev::ThemeChanged(w_id, theme) => {
                let args = RawWindowThemeChangedArgs::now(self.window_id(w_id), theme);
                self.notify_event(RawWindowThemeChangedEvent, args);
            }
            zero_ui_vp::Ev::WindowCloseRequested(w_id) => {
                let args = RawWindowCloseRequestedArgs::now(self.window_id(w_id));
                self.notify_event(RawWindowCloseRequestedEvent, args);
            }
            zero_ui_vp::Ev::WindowClosed(w_id) => {
                let args = RawWindowCloseArgs::now(self.window_id(w_id));
                self.notify_event(RawWindowCloseEvent, args);
            }

            // config events
            zero_ui_vp::Ev::FontsChanged => {
                let args = RawFontChangedArgs::now();
                self.notify_event(RawFontChangedEvent, args);
            }
            zero_ui_vp::Ev::TextAaChanged(aa) => {
                let args = RawTextAaChangedArgs::now(aa);
                self.notify_event(RawTextAaChangedEvent, args);
            }
            zero_ui_vp::Ev::MultiClickConfigChanged(cfg) => {
                let args = RawMultiClickConfigChangedArgs::now(cfg);
                self.notify_event(RawMultiClickConfigChangedEvent, args);
            }
            zero_ui_vp::Ev::AnimationEnabledChanged(enabled) => {
                let args = RawAnimationEnabledChangedArgs::now(enabled);
                self.notify_event(RawAnimationEnabledChangedEvent, args);
            }
            zero_ui_vp::Ev::KeyRepeatDelayChanged(delay) => {
                let args = RawKeyRepeatDelayChangedArgs::now(delay);
                self.notify_event(RawKeyRepeatDelayChangedEvent, args);
            }

            // `device_events`
            zero_ui_vp::Ev::DeviceAdded(d_id) => {
                let args = DeviceArgs::now(self.device_id(d_id));
                self.notify_event(DeviceAddedEvent, args);
            }
            zero_ui_vp::Ev::DeviceRemoved(d_id) => {
                let args = DeviceArgs::now(self.device_id(d_id));
                self.notify_event(DeviceRemovedEvent, args);
            }
            zero_ui_vp::Ev::DeviceMouseMotion(d_id, delta) => {
                let args = MouseMotionArgs::now(self.device_id(d_id), delta);
                self.notify_event(MouseMotionEvent, args);
            }
            zero_ui_vp::Ev::DeviceMouseWheel(d_id, delta) => {
                let args = MouseWheelArgs::now(self.device_id(d_id), delta);
                self.notify_event(MouseWheelEvent, args);
            }
            zero_ui_vp::Ev::DeviceMotion(d_id, axis, value) => {
                let args = MotionArgs::now(self.device_id(d_id), axis, value);
                self.notify_event(MotionEvent, args);
            }
            zero_ui_vp::Ev::DeviceButton(d_id, button, state) => {
                let args = ButtonArgs::now(self.device_id(d_id), button, state);
                self.notify_event(ButtonEvent, args);
            }
            zero_ui_vp::Ev::DeviceKey(d_id, k) => {
                let args = KeyArgs::now(self.device_id(d_id), k.scancode, k.state, k.virtual_keycode.map(Into::into));
                self.notify_event(KeyEvent, args);
            }
            zero_ui_vp::Ev::DeviceText(d_id, c) => {
                let args = TextArgs::now(self.device_id(d_id), c);
                self.notify_event(TextEvent, args);
            }

            // Other
            zero_ui_vp::Ev::Respawned => {
                let args = view_process::ViewProcessRespawnedArgs::now();
                self.notify_event(view_process::ViewProcessRespawnedEvent, args);
            }
        }

        self.maybe_has_updates = true;
        ControlFlow::Poll
    }

    /// Does pending event and updates until there is no more updates generated, then returns
    /// [`Wait`] or [`Exit`]. If [`Wait`] is returned you must check [`wake_time`] for a timeout that
    /// must be used, when the timeout elapses update must be called again, to advance timers.
    ///
    /// You can use an [`AppEventObserver`] to watch all of these actions or pass `&mut ()` as a NOP observer.
    ///
    /// [`Wait`]: ControlFlow::Wait
    /// [`Exit`]: ControlFlow::Exit
    /// [`wake_time`]: RunningApp::wake_time
    pub fn update<O: AppEventObserver>(&mut self, observer: &mut O) -> ControlFlow {
        if self.maybe_has_updates {
            self.maybe_has_updates = false;

            let mut display_update = UpdateDisplayRequest::None;
            let mut new_frames = LinearMap::with_capacity(1);

            let mut limit = 100_000;
            loop {
                limit -= 1;
                if limit == 0 {
                    panic!("update loop polled 100,000 times, probably stuck in an infinite loop");
                }

                let u = self.owned_ctx.apply_updates();
                let mut ctx = self.owned_ctx.borrow();

                self.wake_time = u.wake_time;
                display_update |= u.display_update;
                new_frames.extend(u.new_frames);

                if u.update {
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

                    self.extensions.update_display(&mut ctx, display_update);
                    observer.update_display(&mut ctx, display_update);
                } else if !new_frames.is_empty() {
                    for (window_id, frame_id) in new_frames.drain() {
                        self.extensions.new_frame(&mut ctx, window_id, frame_id);
                        observer.new_frame(&mut ctx, window_id, frame_id);
                    }
                } else {
                    break;
                }
            }
        }

        if self.exiting {
            ControlFlow::Exit
        } else {
            ControlFlow::Wait
        }
    }

    /// De-initializes extensions and drops.
    pub fn shutdown(mut self) {
        let mut ctx = self.owned_ctx.borrow();
        self.extensions.deinit(&mut ctx);
    }
}

/// Desired next step of a loop animating a [`RunningApp`].
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[must_use = "methods that return `ControlFlow` expect to be inside a controlled loop"]
pub enum ControlFlow {
    /// Immediately try to receive more app events.
    Poll,
    /// Sleep until an app event is received.
    ///
    /// Note that a deadline might be set in case a timer is running.
    Wait,
    /// Exit the loop and drop the [`RunningApp`].
    Exit,
}

/// A headless app controller.
///
/// Headless apps don't cause external side-effects like visible windows and don't listen to system events.
/// They can be used for creating apps like a command line app that renders widgets, or for creating integration tests.
pub struct HeadlessApp {
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
    /// When enabled windows are still not visible but you can request [frame pixels]
    /// to get the frame image. Renderer is disabled by default in a headless app.
    ///
    /// Only windows opened after enabling have a renderer. Already open windows are not changed by this method. When enabled
    /// headless windows can only be initialized in the main thread due to limitations of OpenGL, this means you cannot run
    /// a headless renderer in units tests.
    ///
    /// Note that [`UiNode::render`] is still called when a renderer is disabled and you can still
    /// query the latest frame from [`Windows::frame_info`]. The only thing that
    /// is disabled is WebRender and the generation of frame textures.
    ///
    /// [frame pixels]: crate::window::Windows::frame_pixels
    /// [`UiNode::render`]: crate::UiNode::render
    /// [`Windows::frame_info`]: crate::window::Windows::frame_info
    pub fn renderer_enabled(&mut self) -> bool {
        self.ctx().services.get::<view_process::ViewProcess>().is_some()
    }

    /// Borrows the app context.
    pub fn ctx(&mut self) -> AppContext {
        self.app.ctx()
    }

    /// Borrow the [`Vars`] only.
    pub fn vars(&self) -> &Vars {
        self.app.vars()
    }

    /// Does updates unobserved.
    ///
    /// See [`update_observed`] for more details.
    ///
    /// [`update_observed`]: HeadlessApp::update
    #[inline]
    pub fn update(&mut self, wait_app_event: bool) -> ControlFlow {
        self.update_observed(&mut (), wait_app_event)
    }

    /// Does updates observing [`update`] only.
    ///
    /// See [`update_observed`] for more details.
    ///
    /// [`update`]: AppEventObserver::update
    /// [`update_observed`]: HeadlessApp::update
    pub fn update_observe(&mut self, on_update: impl FnMut(&mut AppContext), wait_app_event: bool) -> ControlFlow {
        struct Observer<F>(F);
        impl<F: FnMut(&mut AppContext)> AppEventObserver for Observer<F> {
            fn update(&mut self, ctx: &mut AppContext) {
                (self.0)(ctx)
            }
        }
        let mut observer = Observer(on_update);
        self.update_observed(&mut observer, wait_app_event)
    }

    /// Does updates observing [`event`] only.
    ///
    /// See [`update_observed`] for more details.
    ///
    /// [`event`]: AppEventObserver::event
    /// [`update_observed`]: HeadlessApp::update
    pub fn update_observe_event(&mut self, on_event: impl FnMut(&mut AppContext, &AnyEventUpdate), wait_app_event: bool) -> ControlFlow {
        struct Observer<F>(F);
        impl<F: FnMut(&mut AppContext, &AnyEventUpdate)> AppEventObserver for Observer<F> {
            fn event<EU: EventUpdateArgs>(&mut self, ctx: &mut AppContext, args: &EU) {
                let args = args.as_any();
                (self.0)(ctx, &args);
            }
        }
        let mut observer = Observer(on_event);
        self.update_observed(&mut observer, wait_app_event)
    }

    /// Does updates with an [`AppEventObserver`].
    ///
    /// If `wait_app_event` is `true` the thread sleeps until at least one app event is received or the [`wake_time`] is reached,
    /// if it is `false` only responds to app events already in the buffer.
    ///
    /// [`wake_time`]: Self::wake_time
    pub fn update_observed<O: AppEventObserver>(&mut self, observer: &mut O, wait_app_event: bool) -> ControlFlow {
        if wait_app_event {
            self.app.poll(observer)
        } else {
            self.app.try_poll(observer)
        }
    }

    /// Next timer deadline.
    #[inline]
    pub fn wake_time(&mut self) -> Option<Instant> {
        self.app.wake_time()
    }
}

/// Observer for [`HeadlessApp::update_observed`] and [`RunningApp::update`].
pub trait AppEventObserver {
    /// Called for each raw event received.
    fn raw_event(&mut self, ctx: &mut AppContext, ev: &zero_ui_vp::Ev) {
        let _ = (ctx, ev);
    }

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

    /// Called just after [`AppExtension::new_frame`].
    fn new_frame(&mut self, ctx: &mut AppContext, window_id: WindowId, frame_id: FrameId) {
        let _ = (ctx, window_id, frame_id);
    }
}
/// Nil observer, does nothing.
impl AppEventObserver for () {}

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
    fn new_frame(&mut self, ctx: &mut AppContext, window_id: WindowId, frame_id: FrameId) {
        self.0.new_frame(ctx, window_id, frame_id);
        self.1.new_frame(ctx, window_id, frame_id);
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

    fn new_frame(&mut self, ctx: &mut AppContext, window_id: WindowId, frame_id: FrameId) {
        for ext in self {
            ext.new_frame(ctx, window_id, frame_id);
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

/// App events.
#[derive(Debug)]
pub(crate) enum AppEvent {
    /// Event from the View Process.
    ViewEvent(zero_ui_vp::Ev),
    /// Notify [`Events`](crate::var::Events).
    Event(crate::event::BoxedSendEventUpdate),
    /// Notify [`Vars`](crate::var::Vars).
    Var,
    /// Do an update cycle.
    Update,
    /// Resume a panic in the app thread.
    ResumeUnwind(PanicPayload),
}

/// A sender that can awake apps and insert events into the main loop.
#[derive(Clone)]
pub struct AppEventSender(flume::Sender<AppEvent>);
impl AppEventSender {
    pub(crate) fn new() -> (Self, flume::Receiver<AppEvent>) {
        let (sender, receiver) = flume::unbounded();
        (Self(sender), receiver)
    }

    #[inline(always)]
    fn send_app_event(&self, event: AppEvent) -> Result<(), AppShutdown<AppEvent>> {
        self.0.send(event)?;
        Ok(())
    }

    #[inline(always)]
    fn send_view_event(&self, event: zero_ui_vp::Ev) -> Result<(), AppShutdown<AppEvent>> {
        self.0.send(AppEvent::ViewEvent(event))?;
        Ok(())
    }

    /// Causes an update cycle to happen in the app.
    #[inline]
    pub fn send_update(&self) -> Result<(), AppShutdown<()>> {
        self.send_app_event(AppEvent::Update).map_err(|_| AppShutdown(()))
    }

    /// [`VarSender`](crate::var::VarSender) util.
    #[inline]
    pub(crate) fn send_var(&self) -> Result<(), AppShutdown<()>> {
        self.send_app_event(AppEvent::Var).map_err(|_| AppShutdown(()))
    }

    /// [`EventSender`](crate::event::EventSender) util.
    pub(crate) fn send_event(
        &self,
        event: crate::event::BoxedSendEventUpdate,
    ) -> Result<(), AppShutdown<crate::event::BoxedSendEventUpdate>> {
        self.send_app_event(AppEvent::Event(event)).map_err(|e| match e.0 {
            AppEvent::Event(ev) => AppShutdown(ev),
            _ => unreachable!(),
        })
    }

    /// Resume a panic in the app thread.
    pub fn send_resume_unwind(&self, payload: PanicPayload) -> Result<(), AppShutdown<PanicPayload>> {
        self.send_app_event(AppEvent::ResumeUnwind(payload)).map_err(|e| match e.0 {
            AppEvent::ResumeUnwind(p) => AppShutdown(p),
            _ => unreachable!(),
        })
    }

    /// Create an [`Waker`] that causes a [`send_update`](Self::send_update).
    pub fn waker(&self) -> Waker {
        Arc::new(AppWaker(self.0.clone())).into()
    }
}
struct AppWaker(flume::Sender<AppEvent>);
impl std::task::Wake for AppWaker {
    fn wake(self: std::sync::Arc<Self>) {
        let _ = self.0.send(AppEvent::Update);
    }
}

#[cfg(test)]
mod headless_tests {
    use super::*;

    #[test]
    fn new_default() {
        let mut app = App::default().run_headless(false);
        let cf = app.update(false);
        assert_eq!(cf, ControlFlow::Wait);
    }

    #[test]
    fn new_empty() {
        let mut app = App::blank().run_headless(false);
        let cf = app.update(false);
        assert_eq!(cf, ControlFlow::Wait);
    }

    #[test]
    pub fn new_window_no_render() {
        let mut app = App::default().run_headless(false);
        assert!(!app.renderer_enabled());
        let cf = app.update(false);
        assert_eq!(cf, ControlFlow::Wait);
    }

    #[test]
    #[should_panic(expected = "only one app or `TestWidgetContext` is allowed per thread")]
    pub fn two_in_one_thread() {
        let _a = App::default().run_headless(false);
        let _b = App::default().run_headless(false);
    }

    #[test]
    #[should_panic(expected = "only one `TestWidgetContext` or app is allowed per thread")]
    pub fn app_and_test_ctx() {
        let _a = App::default().run_headless(false);
        let _b = TestWidgetContext::new();
    }

    #[test]
    #[should_panic(expected = "only one app or `TestWidgetContext` is allowed per thread")]
    pub fn test_ctx_and_app() {
        let _a = TestWidgetContext::new();
        let _b = App::default().run_headless(false);
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

unique_id! {
    /// Unique identifier of a device event source.
    #[derive(Debug)]
    pub struct DeviceId;
}
impl DeviceId {
    /// Virtual keyboard ID used in keyboard events generated by code.
    pub fn virtual_keyboard() -> DeviceId {
        static ID: Lazy<DeviceId> = Lazy::new(DeviceId::new_unique);
        *ID
    }

    /// Virtual mouse ID used in mouse events generated by code.
    pub fn virtual_mouse() -> DeviceId {
        static ID: Lazy<DeviceId> = Lazy::new(DeviceId::new_unique);
        *ID
    }

    /// Virtual generic device ID used in device events generated by code.
    pub fn virtual_generic() -> DeviceId {
        static ID: Lazy<DeviceId> = Lazy::new(DeviceId::new_unique);
        *ID
    }
}
impl fmt::Display for DeviceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DeviceId({})", self.get())
    }
}

/// View process controller types.
pub mod view_process {
    use std::path::PathBuf;
    use std::rc;
    use std::time::Duration;
    use std::{cell::RefCell, rc::Rc};

    use linear_map::LinearMap;

    use webrender_api::{DynamicProperties, FontInstanceKey, FontKey, HitTestResult, IdNamespace, ImageKey, PipelineId, ResourceUpdate};
    use zero_ui_vp::{Controller, DevId, WinId};
    pub use zero_ui_vp::{
        CursorIcon, Error, Ev, FramePixels, FrameRequest, Icon, MonitorInfo, Result, TextAntiAliasing, VideoMode, WindowConfig, WindowTheme,
    };

    use super::DeviceId;
    use crate::mouse::MultiClickConfig;
    use crate::service::Service;
    use crate::units::{LayoutPoint, LayoutRect, LayoutSize};
    use crate::window::{MonitorId, WindowId};
    use crate::{event, event_args};

    /// Reference to the running View Process.
    ///
    /// This is the lowest level API, used for implementing fundamental services and is a service available
    /// in headed apps or headless apps with renderer.
    ///
    /// This is a strong reference to the view process. The process shuts down when all clones of this struct drops.
    #[derive(Service, Clone)]
    pub struct ViewProcess(Rc<RefCell<ViewApp>>);
    struct ViewApp {
        process: zero_ui_vp::Controller,
        window_ids: LinearMap<WinId, WindowId>,
        device_ids: LinearMap<DevId, DeviceId>,
        monitor_ids: LinearMap<zero_ui_vp::MonId, MonitorId>,
    }
    impl ViewProcess {
        /// Spawn the View Process.
        pub(super) fn start<F>(view_process_exe: Option<PathBuf>, device_events: bool, headless: bool, on_event: F) -> Self
        where
            F: FnMut(Ev) + Send + 'static,
        {
            Self(Rc::new(RefCell::new(ViewApp {
                process: zero_ui_vp::Controller::start(view_process_exe, device_events, headless, on_event),
                window_ids: LinearMap::default(),
                device_ids: LinearMap::default(),
                monitor_ids: LinearMap::default(),
            })))
        }

        /// If is running in headless renderer mode.
        #[inline]
        pub fn headless(&self) -> bool {
            self.0.borrow().process.headless()
        }

        /// If is running both view and app in the same process.
        #[inline]
        pub fn same_process(&self) -> bool {
            self.0.borrow().process.same_process()
        }

        /// Open a window and associate it with the `window_id`.
        pub fn open_window(&self, window_id: WindowId, config: WindowConfig) -> Result<ViewWindow> {
            let mut app = self.0.borrow_mut();
            assert!(app.window_ids.values().all(|&v| v != window_id));

            let id = app.process.open_window(config)?;

            app.window_ids.insert(id, window_id);

            Ok(ViewWindow(Rc::new(WindowConnection { id, app: self.0.clone() })))
        }

        /// Read the system text anti-aliasing config.
        #[inline]
        pub fn text_aa(&self) -> Result<TextAntiAliasing> {
            self.0.borrow_mut().process.text_aa()
        }

        /// Read the system "double-click" config.
        #[inline]
        pub fn multi_click_config(&self) -> Result<MultiClickConfig> {
            self.0.borrow_mut().process.multi_click_config()
        }

        /// Retrieves the keyboard repeat-delay setting from the operating system.
        #[inline]
        pub fn key_repeat_delay(&self) -> Result<Duration> {
            self.0.borrow_mut().process.key_repeat_delay()
        }

        /// Returns the primary monitor if there is any or the first available monitor or none if no monitor was found.
        #[inline]
        pub fn primary_monitor(&self) -> Result<Option<(MonitorId, MonitorInfo)>> {
            let m = self.0.borrow_mut().process.primary_monitor()?;
            Ok(m.map(|(id, m)| (self.monitor_id(id), m)))
        }

        /// Returns all available monitors.
        #[inline]
        pub fn available_monitors(&self) -> Result<Vec<(MonitorId, MonitorInfo)>> {
            let m = self.0.borrow_mut().process.available_monitors()?;
            Ok(m.into_iter().map(|(id, m)| (self.monitor_id(id), m)).collect())
        }

        /// Returns information about the specific monitor, if it exists.
        #[inline]
        pub fn monitor_info(&self, monitor_id: MonitorId) -> Result<Option<MonitorInfo>> {
            if let Some(id) = self.monitor_id_back(monitor_id) {
                self.0.borrow_mut().process.monitor_info(id)
            } else {
                Ok(None)
            }
        }

        /// Translate `WinId` to `WindowId`.
        pub(super) fn window_id(&self, id: WinId) -> Option<WindowId> {
            self.0.borrow().window_ids.get(&id).copied()
        }

        /// Translate `DevId` to `DeviceId`, generates a device id if it was unknown.
        pub(super) fn device_id(&self, id: DevId) -> DeviceId {
            *self.0.borrow_mut().device_ids.entry(id).or_insert_with(DeviceId::new_unique)
        }

        /// Translate `MonId` to `MonitorId`, generates a monitor id if it was unknown.
        pub(super) fn monitor_id(&self, id: zero_ui_vp::MonId) -> MonitorId {
            *self.0.borrow_mut().monitor_ids.entry(id).or_insert_with(MonitorId::new_unique)
        }

        /// Translate `MonitorId` to `MonId`.
        pub(super) fn monitor_id_back(&self, monitor_id: MonitorId) -> Option<zero_ui_vp::MonId> {
            self.0
                .borrow()
                .monitor_ids
                .iter()
                .find(|(_, app_id)| **app_id == monitor_id)
                .map(|(id, _)| *id)
        }
    }

    struct WindowConnection {
        id: WinId,
        app: Rc<RefCell<ViewApp>>,
    }
    impl WindowConnection {
        fn call<R>(&self, f: impl FnOnce(WinId, &mut Controller) -> Result<R>) -> Result<R> {
            f(self.id, &mut self.app.borrow_mut().process)
        }
    }
    impl Drop for WindowConnection {
        fn drop(&mut self) {
            let _ = self.app.borrow_mut().process.close_window(self.id);
        }
    }

    /// Connection to a window open in the View Process.
    ///
    /// This is a strong reference to the window connection. The window closes when all clones of this struct drops.
    #[derive(Clone)]
    pub struct ViewWindow(Rc<WindowConnection>);
    impl ViewWindow {
        /// Set the window title.
        #[inline]
        pub fn set_title(&self, title: String) -> Result<()> {
            self.0.call(|id, p| p.set_title(id, title))
        }

        /// Set the window visibility.
        #[inline]
        pub fn set_visible(&self, visible: bool) -> Result<()> {
            self.0.call(|id, p| p.set_visible(id, visible))
        }

        /// Set if the window is "top-most".
        #[inline]
        pub fn set_always_on_top(&self, always_on_top: bool) -> Result<()> {
            self.0.call(|id, p| p.set_always_on_top(id, always_on_top))
        }

        /// Set if the user can drag-move the window.
        #[inline]
        pub fn set_movable(&self, movable: bool) -> Result<()> {
            self.0.call(|id, p| p.set_movable(id, movable))
        }

        /// Set if the user can resize the window.
        #[inline]
        pub fn set_resizable(&self, resizable: bool) -> Result<()> {
            self.0.call(|id, p| p.set_resizable(id, resizable))
        }

        /// Set the window icon.
        #[inline]
        pub fn set_icon(&self, icon: Option<Icon>) -> Result<()> {
            self.0.call(|id, p| p.set_icon(id, icon))
        }

        /// Set the window icon visibility in the taskbar.
        #[inline]
        pub fn set_taskbar_visible(&self, visible: bool) -> Result<()> {
            self.0.call(|id, p| p.set_taskbar_visible(id, visible))
        }

        /// Set the window parent and if `self` has a modal connection to it.
        ///
        /// The `parent` window must be already open or this returns `WindowNotFound(0)`.
        #[inline]
        pub fn set_parent(&self, parent: Option<WindowId>, modal: bool) -> Result<()> {
            if let Some(parent) = parent {
                if let Some((parent_id, _)) = self.0.app.borrow().window_ids.iter().find(|(_, window_id)| **window_id == parent) {
                    self.0.call(|id, p| p.set_parent(id, Some(*parent_id), modal))
                } else {
                    self.0.call(|id, p| p.set_parent(id, None, modal))?;
                    Err(Error::WindowNotFound(0))
                }
            } else {
                self.0.call(|id, p| p.set_parent(id, None, modal))
            }
        }

        /// Set if the window is see-through.
        #[inline]
        pub fn set_transparent(&self, transparent: bool) -> Result<()> {
            self.0.call(|id, p| p.set_transparent(id, transparent))
        }

        /// Set the window position.
        #[inline]
        pub fn set_position(&self, pos: LayoutPoint) -> Result<()> {
            self.0.call(|id, p| p.set_position(id, pos))
        }

        /// Set the window size.
        #[inline]
        pub fn set_size(&self, size: LayoutSize) -> Result<()> {
            self.0.call(|id, p| p.set_size(id, size))
        }

        /// Set the window minimum size.
        #[inline]
        pub fn set_min_size(&self, size: LayoutSize) -> Result<()> {
            self.0.call(|id, p| p.set_min_size(id, size))
        }

        /// Set the window maximum size.
        #[inline]
        pub fn set_max_size(&self, size: LayoutSize) -> Result<()> {
            self.0.call(|id, p| p.set_max_size(id, size))
        }

        /// Set the visibility of the native window borders and title.
        #[inline]
        pub fn set_chrome_visible(&self, visible: bool) -> Result<()> {
            self.0.call(|id, p| p.set_chrome_visible(id, visible))
        }

        /// Reference the window renderer.
        #[inline]
        pub fn renderer(&self) -> ViewRenderer {
            ViewRenderer(Rc::downgrade(&self.0))
        }

        /// In Windows stops the system from requesting a window close on `ALT+F4` and sends a key
        /// press for F4 instead.
        #[inline]
        pub fn set_allow_alt_f4(&self, allow: bool) -> Result<()> {
            self.0.call(|id, p| p.set_allow_alt_f4(id, allow))
        }

        /// Drop `self`.
        pub fn close(self) {
            drop(self)
        }
    }

    /// Connection to a renderer in the View Process.
    ///
    /// This is only a weak reference, every method returns [`WindowNotFound`] if the
    /// renderer has been dropped.
    ///
    /// [`WindowNotFound`]: Error::WindowNotFound(0)
    #[derive(Clone)]
    pub struct ViewRenderer(rc::Weak<WindowConnection>);
    impl ViewRenderer {
        fn call<R>(&self, f: impl FnOnce(WinId, &mut Controller) -> Result<R>) -> Result<R> {
            if let Some(c) = self.0.upgrade() {
                c.call(f)
            } else {
                Err(Error::WindowNotFound(0))
            }
        }

        /// Gets the root pipeline ID.
        pub fn pipeline_id(&self) -> Result<PipelineId> {
            self.call(|id, p| p.pipeline_id(id))
        }

        /// Gets the resource namespace.
        pub fn namespace_id(&self) -> Result<IdNamespace> {
            self.call(|id, p| p.namespace_id(id))
        }

        /// New image resource key.
        pub fn generate_image_key(&self) -> Result<ImageKey> {
            self.call(|id, p| p.generate_image_key(id))
        }

        /// New font resource key.
        pub fn generate_font_key(&self) -> Result<FontKey> {
            self.call(|id, p| p.generate_font_key(id))
        }

        /// New font instance key.
        pub fn generate_font_instance_key(&self) -> Result<FontInstanceKey> {
            self.call(|id, p| p.generate_font_instance_key(id))
        }

        /// Gets the viewport size (window inner size).
        pub fn size(&self) -> Result<LayoutSize> {
            self.call(|id, p| p.size(id))
        }

        /// Gets the window scale factor.
        pub fn scale_factor(&self) -> Result<f32> {
            self.call(|id, p| p.scale_factor(id))
        }

        /// Read a `rect` of pixels from the current frame.
        ///
        /// This is a call to `glReadPixels`, each pixel row is stacked from
        /// bottom-to-top with the pixel type BGRA8.
        pub fn read_pixels_rect(&self, rect: LayoutRect) -> Result<FramePixels> {
            self.call(|id, p| p.read_pixels_rect(id, rect))
        }

        /// Read all pixels of the current frame.
        ///
        /// This is a call to `glReadPixels`, the first pixel row order is bottom-to-top and the pixel type is BGRA.
        pub fn read_pixels(&self) -> Result<FramePixels> {
            self.call(|id, p| p.read_pixels(id))
        }

        /// Get display items of the last rendered frame that intercept the `point`.
        ///
        /// Returns all hits from front-to-back.
        pub fn hit_test(&self, point: LayoutPoint) -> Result<HitTestResult> {
            self.call(|id, p| p.hit_test(id, point))
        }

        /// Change the text anti-alias used in this renderer.
        pub fn set_text_aa(&self, aa: TextAntiAliasing) -> Result<()> {
            self.call(|id, p| p.set_text_aa(id, aa))
        }

        /// Render a new frame.
        pub fn render(&self, frame: FrameRequest) -> Result<()> {
            self.call(|id, p| p.render(id, frame))
        }

        /// Update the current frame and re-render it.
        pub fn render_update(&self, updates: DynamicProperties) -> Result<()> {
            self.call(|id, p| p.render_update(id, updates))
        }

        /// Add/remove/update resources such as images and fonts.
        pub fn update_resources(&self, updates: Vec<ResourceUpdate>) -> Result<()> {
            self.call(|id, p| p.update_resources(id, updates))
        }
    }

    event_args! {
        /// Arguments for the [`ViewProcessRespawnedEvent`].
        pub struct ViewProcessRespawnedArgs {

            ..

            /// Returns `true` for all widgets.
            fn concerns_widget(&self, _ctx: &mut WidgetContext) -> bool {
                true
            }
        }

    }

    event! {
        /// View Process crashed and respawned, resources may need to be rebuild.
        ///
        /// This event fires if the view-process crashed and was successfully
        pub ViewProcessRespawnedEvent: ViewProcessRespawnedArgs;
    }
}

/// Events directly from `winit` targeting the app windows.
///
/// These events get processed by [app extensions] to generate the events used in widgets, for example
/// the [`KeyboardManager`] uses the [`RawKeyInputEvent`] into focus targeted events.
///
/// # Synthetic Input
///
/// You can [`notify`] these events to fake hardware input, please be careful that you mimic the exact sequence a real
/// hardware would generate, [app extensions] can assume that the raw events are correct. The [`DeviceId`] for fake
/// input must be unique but constant for each distinctive *synthetic event source*.
///
/// [app extensions]: crate::app::AppExtension
/// [`KeyboardManager`]: crate::keyboard::KeyboardManager
/// [`RawKeyInputEvent`]: crate::app::raw_events::RawKeyInputEvent
/// [`notify`]: crate::event::Event::notify
/// [`DeviceId`]: crate::app::DeviceId
pub mod raw_events {
    use std::{path::PathBuf, time::Duration};

    use super::{
        raw_device_events::AxisId,
        view_process::{MonitorInfo, TextAntiAliasing},
        DeviceId,
    };
    use crate::{
        event::*,
        keyboard::{Key, KeyState, ModifiersState, ScanCode},
        mouse::{ButtonState, MouseButton, MultiClickConfig},
        units::{LayoutPoint, LayoutSize},
        window::{MonitorId, WindowId, WindowTheme},
    };

    event_args! {
        /// Arguments for the [`RawKeyInputEvent`].
        pub struct RawKeyInputArgs {
            /// Window that received the event.
            pub window_id: WindowId,

            /// Keyboard device that generated the event.
            pub device_id: DeviceId,

            /// Raw code of key.
            pub scan_code: ScanCode,

            /// If the key was pressed or released.
            pub state: KeyState,

            /// Symbolic name of [`scan_code`](Self::scan_code).
            pub key: Option<Key>,

            ..

            /// Returns `true` for all widgets in the [window](Self::window_id).
            fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
                ctx.path.window_id() == self.window_id
            }
        }

        /// Arguments for the [`RawModifiersChangedEvent`].
        pub struct RawModifiersChangedArgs {
            /// Window that received the event.
            pub window_id: WindowId,

            /// New modifiers state.
            pub modifiers: ModifiersState,

            ..

            /// Returns `true` for all widgets in the [window](Self::window_id).
            fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
                ctx.path.window_id() == self.window_id
            }
        }

        /// Arguments for the [`RawCharInputEvent`].
        pub struct RawCharInputArgs {
            /// Window that received the event.
            pub window_id: WindowId,

            /// Unicode character.
            pub character: char,

            ..

            /// Returns `true` for all widgets in the [window](Self::window_id).
            fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
                ctx.path.window_id() == self.window_id
            }
        }

        /// Arguments for the [`RawWindowFocusEvent`].
        pub struct RawWindowFocusArgs {
            /// Window that was focuses/blurred.
            pub window_id: WindowId,

            /// If the window received focus.
            pub focused: bool,

            ..

            /// Returns `true` for all widgets in the [window](Self::window_id).
            fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
                ctx.path.window_id() == self.window_id
            }
        }

        /// Arguments for the [`RawWindowMovedEvent`].
        pub struct RawWindowMovedArgs {
            /// Window that was moved.
            pub window_id: WindowId,

            /// Window top-left offset, including the system chrome.
            pub position: LayoutPoint,

            ..

            /// Returns `true` for all widgets in the [window](Self::window_id).
            fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
                ctx.path.window_id() == self.window_id
            }
        }

        /// Arguments for the [`RawWindowResizedEvent`].
        pub struct RawWindowResizedArgs {
            /// Window that was resized.
            pub window_id: WindowId,

            /// Window new size.
            pub size: LayoutSize,

            ..

            /// Returns `true` for all widgets in the [window](Self::window_id).
            fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
                ctx.path.window_id() == self.window_id
            }
        }

        /// Arguments for the [`RawWindowCloseRequestedEvent`].
        pub struct RawWindowCloseRequestedArgs {
            /// Window that was requested to close.
            pub window_id: WindowId,

            ..

            /// Returns `true` for all widgets in the [window](Self::window_id).
            fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
                ctx.path.window_id() == self.window_id
            }
        }

        /// Arguments for the [`RawWindowCloseEvent`].
        pub struct RawWindowCloseArgs {
            /// Window that was destroyed.
            pub window_id: WindowId,

            ..

            /// Returns `true` for all widgets.
            fn concerns_widget(&self, _ctx: &mut WidgetContext) -> bool {
                true
            }
        }

        /// Arguments for the [`RawDroppedFileEvent`].
        pub struct RawDroppedFileArgs {
            /// Window where it was dropped.
            pub window_id: WindowId,

            /// Path to file that was dropped.
            pub file: PathBuf,

            ..

            /// Returns `true` for all widgets in the [window](Self::window_id).
            fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
                ctx.path.window_id() == self.window_id
            }
        }

        /// Arguments for the [`RawHoveredFileEvent`].
        pub struct RawHoveredFileArgs {
            /// Window where it was dragged over.
            pub window_id: WindowId,

            /// Path to file that was dragged over the window.
            pub file: PathBuf,

            ..

            /// Returns `true` for all widgets in the [window](Self::window_id).
            fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
                ctx.path.window_id() == self.window_id
            }
        }

        /// Arguments for the [`RawHoveredFileCancelledEvent`].
        ///
        /// The file is the one that was last [hovered] into the window.
        ///
        /// [hovered]: RawHoveredFileEvent
        pub struct RawHoveredFileCancelledArgs {
            /// Window where the file was previously dragged over.
            pub window_id: WindowId,

            ..

            /// Returns `true` for all widgets in the [window](Self::window_id).
            fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
                ctx.path.window_id() == self.window_id
            }
        }

        /// Arguments for the [`RawCursorMovedEvent`].
        pub struct RawCursorMovedArgs {
            /// Window the cursor was moved over.
            pub window_id: WindowId,

            /// Device that generated this event.
            pub device_id: DeviceId,

            /// Position of the cursor over the window, (0, 0) is the top-left.
            pub position: LayoutPoint,

            ..

            /// Returns `true` for all widgets in the [window](Self::window_id).
            fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
                ctx.path.window_id() == self.window_id
            }
        }

        /// Arguments for the [`RawCursorEnteredEvent`] and [`RawCursorLeftEvent`].
        pub struct RawCursorArgs {
            /// Window the cursor entered or left.
            pub window_id: WindowId,

            /// Device that generated this event.
            pub device_id: DeviceId,

            ..

            /// Returns `true` for all widgets in the [window](Self::window_id).
            fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
                ctx.path.window_id() == self.window_id
            }
        }

        /// Arguments for the [`RawMouseWheelEvent`].
        pub struct RawMouseWheelArgs {
            /// Window that is hovered by the cursor.
            pub window_id: WindowId,

            /// Device that generated this event.
            pub device_id: DeviceId,

            // TODO

            ..

            /// Returns `true` for all widgets in the [window](Self::window_id).
            fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
                ctx.path.window_id() == self.window_id
            }
        }

        /// Arguments for the [`RawMouseInputEvent`].
        pub struct RawMouseInputArgs {
            /// Window that is hovered by the cursor.
            pub window_id: WindowId,

            /// Device that generated this event.
            pub device_id: DeviceId,

            /// If the button was pressed or released.
            pub state: ButtonState,

            /// What button was pressed or released.
            pub button: MouseButton,

            ..

            /// Returns `true` for all widgets in the [window](Self::window_id).
            fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
                ctx.path.window_id() == self.window_id
            }
        }

        /// Arguments for the [`RawTouchpadPressureEvent`].
        pub struct RawTouchpadPressureArgs {
            /// Window that is touched.
            pub window_id: WindowId,

            /// Device that generated this event.
            pub device_id: DeviceId,

            // TODO

            ..

            /// Returns `true` for all widgets in the [window](Self::window_id).
            fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
                ctx.path.window_id() == self.window_id
            }
        }

        /// Arguments for the [`RawAxisMotionEvent`].
        pub struct RawAxisMotionArgs {
            /// Window that received the event.
            pub window_id: WindowId,

            /// Device that generated the event.
            pub device_id: DeviceId,

            /// Analog axis.
            pub axis: AxisId,

            /// Motion amount.
            pub value: f64,

            ..

            /// Returns `true` for all widgets in the [window](Self::window_id).
            fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
                ctx.path.window_id() == self.window_id
            }
        }

        /// Arguments for the [`RawTouchEvent`].
        pub struct RawTouchArgs {
            /// Window that was touched.
            pub window_id: WindowId,

            /// Device that generated this event.
            pub device_id: DeviceId,

            // TODO

            ..

            /// Returns `true` for all widgets in the [window](Self::window_id).
            fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
                ctx.path.window_id() == self.window_id
            }
        }

        /// Arguments for the [`RawWindowScaleFactorChangedEvent`].
        pub struct RawWindowScaleFactorChangedArgs {
            /// Window for which the scale has changed.
            pub window_id: WindowId,

            /// New pixel scale factor.
            pub scale_factor: f32,

            ..

            /// Returns `true` for all widgets in the [window](Self::window_id).
            fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
                ctx.path.window_id() == self.window_id
            }
        }

        /// Arguments for the [`RawMonitorsChangedEvent`].
        pub struct RawMonitorsChangedArgs {
            /// Up-to-date monitors list.
            pub available_monitors: Vec<(MonitorId, MonitorInfo)>,

            ..

            /// Concerns all widgets.
            fn concerns_widget(&self, _ctx: &mut WidgetContext) -> bool {
                true
            }
        }

        /// Arguments for the [`RawWindowThemeChangedEvent`].
        pub struct RawWindowThemeChangedArgs {
            /// Window for which the theme was changed.
            pub window_id: WindowId,

            /// New theme.
            pub theme: WindowTheme,

            ..

            /// Returns `true` for all widgets in the [window](Self::window_id).
            fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
                ctx.path.window_id() == self.window_id
            }
        }

        /// [`RawFontChangedEvent`] arguments.
        pub struct RawFontChangedArgs {
            ..

            /// Concerns all widgets.
            fn concerns_widget(&self, _ctx: &mut WidgetContext) -> bool {
                true
            }
        }

        /// Arguments for the [`RawTextAaChangedEvent`].
        pub struct RawTextAaChangedArgs {
            /// The new anti-aliasing config.
            pub aa: TextAntiAliasing,

            ..

            /// Concerns all widgets.
            fn concerns_widget(&self, _ctx: &mut WidgetContext) -> bool {
                true
            }
        }

        /// Arguments for the [`RawMultiClickConfigChangedEvent`].
        pub struct RawMultiClickConfigChangedArgs {
            /// New config.
            pub config: MultiClickConfig,

            ..

            /// Concerns all widgets.
            fn concerns_widget(&self, _ctx: &mut WidgetContext) -> bool {
                true
            }
        }

        /// Arguments for the [`RawAnimationEnabledChangedEvent`].
        pub struct RawAnimationEnabledChangedArgs {
            /// If animation is enabled in the operating system.
            pub enabled: bool,

            ..

            /// Concerns all widgets.
            fn concerns_widget(&self, _ctx: &mut WidgetContext) -> bool {
                true
            }
        }

        /// Arguments for the [`RawKeyRepeatDelayChangedEvent`].
        pub struct RawKeyRepeatDelayChangedArgs {
            /// New delay.
            ///
            /// When the user holds a key pressed the system will generate a new key-press event
            /// every time this delay elapses. The real delay time depends on the hardware but it
            /// roughly matches this value.
            pub delay: Duration,

            ..

            /// Concerns all widgets.
            fn concerns_widget(&self, _ctx: &mut WidgetContext) -> bool {
                true
            }
        }
    }

    event! {
        /// A key press or release targeting a window.
        ///
        /// This event represents a key input directly from the operating system. It is processed
        /// by [`KeyboardManager`] to generate the [`KeyInputEvent`] that actually targets the focused widget.
        ///
        /// *See also the [module level documentation](self) for details of how you can fake this event*
        ///
        /// [`KeyboardManager`]: crate::keyboard::KeyboardManager
        /// [`KeyInputEvent`]: crate::keyboard::KeyInputEvent
        pub RawKeyInputEvent: RawKeyInputArgs;

        /// A modifier key press or release updated the state of the modifier keys.
        ///
        /// This event represents a key input directly from the operating system. It is processed
        /// by [`KeyboardManager`] to generate the keyboard events that are used in general.
        ///
        /// *See also the [module level documentation](self) for details of how you can fake this event*
        ///
        /// [`KeyboardManager`]: crate::keyboard::KeyboardManager
        pub RawModifiersChangedEvent: RawModifiersChangedArgs;

        /// A window received an Unicode character.
        pub RawCharInputEvent: RawCharInputArgs;

        /// A window received or lost focus.
        pub RawWindowFocusEvent: RawWindowFocusArgs;

        /// A window was moved.
        pub RawWindowMovedEvent: RawWindowMovedArgs;

        /// A window was resized.
        pub RawWindowResizedEvent: RawWindowResizedArgs;

        /// A window was requested to close.
        pub RawWindowCloseRequestedEvent: RawWindowCloseRequestedArgs;

        /// A window was destroyed.
        pub RawWindowCloseEvent: RawWindowCloseArgs;

        /// A file was drag-dropped on a window.
        pub RawDroppedFileEvent: RawDroppedFileArgs;

        /// A file was dragged over a window.
        ///
        /// If the file is dropped [`RawDroppedFileEvent`] will raise.
        pub RawHoveredFileEvent: RawHoveredFileArgs;

        /// A dragging file was moved away from the window or the operation was cancelled.
        ///
        /// The file is the last one that emitted a [`RawHoveredFileEvent`].
        pub RawHoveredFileCancelledEvent: RawHoveredFileCancelledArgs;

        /// Cursor pointer moved over a window.
        pub RawCursorMovedEvent: RawCursorMovedArgs;

        /// Cursor pointer started hovering a window.
        pub RawCursorEnteredEvent: RawCursorArgs;

        /// Cursor pointer stopped hovering a window.
        pub RawCursorLeftEvent: RawCursorArgs;

        /// Mouse wheel scrolled when the cursor was over a window.
        pub RawMouseWheelEvent: RawMouseWheelArgs;

        /// Mouse button was pressed or released when the cursor was over a window.
        pub RawMouseInputEvent: RawMouseInputArgs;

        /// Touchpad touched when the cursor was over a window.
        pub RawTouchpadPressureEvent: RawTouchpadPressureArgs;

        /// Motion on some analog axis send to a window.
        pub RawAxisMotionEvent: RawAxisMotionArgs;

        /// A window was touched.
        pub RawTouchEvent: RawTouchArgs;

        /// Pixel scale factor for a window changed.
        ///
        /// This can happen when the window is dragged to another monitor or if the user
        /// change the screen scaling configuration.
        pub RawWindowScaleFactorChangedEvent: RawWindowScaleFactorChangedArgs;

        /// Monitors added or removed.
        pub RawMonitorsChangedEvent: RawMonitorsChangedArgs;

        /// System theme changed for a window.
        pub RawWindowThemeChangedEvent: RawWindowThemeChangedArgs;

        /// Change in system text anti-aliasing config.
        pub RawTextAaChangedEvent: RawTextAaChangedArgs;

        /// Change in system text fonts, install or uninstall.
        pub RawFontChangedEvent: RawFontChangedArgs;

        /// Change in system "double-click" config.
        pub RawMultiClickConfigChangedEvent: RawMultiClickConfigChangedArgs;

        /// Change in system animation enabled config.
        pub RawAnimationEnabledChangedEvent: RawAnimationEnabledChangedArgs;

        /// Change in system key repeat interval config.
        pub RawKeyRepeatDelayChangedEvent: RawKeyRepeatDelayChangedArgs;
    }
}

/// Events directly from `winit` not targeting any windows.
///
/// These events get emitted only if the app [`enable_device_events`]. When enabled they
/// can be used like [`raw_events`].
///
/// [`enable_device_events`]: AppExtended::enable_device_events
pub mod raw_device_events {
    use super::DeviceId;
    use crate::{
        event::*,
        keyboard::{Key, KeyState, ScanCode},
        mouse::ButtonState,
    };

    pub use zero_ui_vp::{AxisId, ButtonId, MouseScrollDelta};

    event_args! {
        /// Arguments for [`DeviceAddedEvent`] and [`DeviceRemovedEvent`].
        pub struct DeviceArgs {
            /// Device that was added/removed.
            pub device_id: DeviceId,

            ..

            /// Returns `true` for all widgets.
            fn concerns_widget(&self, _ctx: &mut WidgetContext) -> bool {
                true
            }
        }

        /// Arguments for [`MouseMotionEvent`].
        pub struct MouseMotionArgs {
            /// Mouse device that generated the event.
            pub device_id: DeviceId,

            /// Motion (x, y) delta.
            pub delta: (f64, f64),

            ..

            /// Returns `true` for all widgets.
            fn concerns_widget(&self, _ctx: &mut WidgetContext) -> bool {
                true
            }
        }

        /// Arguments for [`MouseWheelEvent`].
        pub struct MouseWheelArgs {
            /// Mouse device that generated the event.
            pub device_id: DeviceId,

            /// Wheel motion delta, value be in pixels if the *wheel* is a touchpad.
            pub delta: MouseScrollDelta,

            ..

            /// Returns `true` for all widgets.
            fn concerns_widget(&self, _ctx: &mut WidgetContext) -> bool {
                true
            }
        }

        /// Arguments for [`MotionEvent`].
        pub struct MotionArgs {
            /// Device that generated the event.
            pub device_id: DeviceId,

            /// Analog axis.
            pub axis: AxisId,

            /// Motion amount.
            pub value: f64,

            ..

            /// Returns `true` for all widgets.
            fn concerns_widget(&self, _ctx: &mut WidgetContext) -> bool {
                true
            }
        }

        /// Arguments for the [`ButtonEvent`].
        pub struct ButtonArgs {
            /// Device that generated the event.
            pub device_id: DeviceId,

            /// Button raw id.
            pub button: ButtonId,

            /// If the button was pressed or released.
            pub state: ButtonState,

            ..

            /// Returns `true` for all widgets.
            fn concerns_widget(&self, _ctx: &mut WidgetContext) -> bool {
                true
            }
        }

        /// Arguments for the [`KeyEvent`].
        pub struct KeyArgs {
            /// Keyboard device that generated the event.
            pub device_id: DeviceId,

            /// Raw code of key.
            pub scan_code: ScanCode,

            /// If the key was pressed or released.
            pub state: KeyState,

            /// Symbolic name of [`scan_code`](Self::scan_code).
            pub key: Option<Key>,

            ..

            /// Returns `true` for all widgets.
            fn concerns_widget(&self, _ctx: &mut WidgetContext) -> bool {
                true
            }
        }

        /// Arguments for the [`TextEvent`].
        pub struct TextArgs {
            /// Device that generated the event.
            pub device_id: DeviceId,

            /// Character received.
            pub code_point: char,

            ..

            /// Returns `true` for all widgets.
            fn concerns_widget(&self, _ctx: &mut WidgetContext) -> bool {
                true
            }
        }
    }

    event! {
        /// A device event source was added/installed.
        pub DeviceAddedEvent: DeviceArgs;

        /// A device event source was removed/un-installed.
        pub DeviceRemovedEvent: DeviceArgs;

        /// Mouse device unfiltered move delta.
        pub MouseMotionEvent: MouseMotionArgs;

        /// Mouse device unfiltered wheel motion delta.
        pub MouseWheelEvent: MouseWheelArgs;

        /// Motion on some analog axis.
        ///
        /// This event will be reported for all arbitrary input devices that `winit` supports on this platform,
        /// including mouse devices. If the device is a mouse device then this will be reported alongside the [`MouseMotionEvent`].
        pub MotionEvent: MotionArgs;

        /// Button press/release from a device, probably a mouse.
        pub ButtonEvent: ButtonArgs;

        /// Keyboard device key press.
        pub KeyEvent: KeyArgs;

        /// Raw text input.
        pub TextEvent: TextArgs;
    }
}
