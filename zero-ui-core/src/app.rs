//! App startup and app extension API.

use crate::context::*;
use crate::crate_util::PanicPayload;
use crate::event::{cancelable_event_args, event, AnyEventUpdate, EventUpdate, EventUpdateArgs, Events};
use crate::image::ImageManager;
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
use view_process::ViewProcessExt;

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
///
/// # App Loop
///
/// Methods in app extension are called in this synchronous order:
///
/// ## 1 - Init
///
/// The [`init`] method is called once at the start of the app. Extensions are initialized in the order then where *inserted* in the app.
///
/// ## 2 - Events
///
/// The [`event_preview`], [`event_ui`] and [`event`] methods are called in this order for each event message received. Events
/// received from other threads are buffered until the app is free and then are processed using these methods.
///
/// ## 3 - Updates
///
/// The [`update_preview`], [`update_ui`] and [`update`] methods are called in this order every time an [update is requested],
/// a sequence of events have processed, variables where assigned or timers elapsed. The app loops between [events] and [updates] until
/// no more updates or events are pending, if [layout] or [render] are requested they are deferred until a event-update cycle is complete.
///
/// # 4 - Layout
///
/// The [`layout`] method is called if during [init], [events] or [updates] a layout was requested, extensions should also remember which
/// unit requested layout, to avoid unnecessary work, for example the [`WindowManager`] remembers witch window requested layout.
///
/// If the [`layout`] call requests updates the app goes back to [updates], requests for render are again deferred.
///
/// # 5 - Render
///
/// The [`render`] method is called if during [init], [events], [updates] or [layout] a render was requested and no other
/// event, update or layout is pending. Extensions should identify which unit is pending a render or render update and generate
/// and send a display list or frame update.
///
/// This method does not block until the frame pixels are rendered, it covers only the creation of a frame request sent to the view-process.
/// A [`RawFrameRenderedEvent`] is send when a frame finished rendering in the view-process.
///
/// ## 6 - Deinit
///
/// The [`deinit`] method is called once after a shutdown was requested and not cancelled. Shutdowns are
/// requested using the [`AppProcess`] service, it causes a [`ShutdownRequestedEvent`] that can be cancelled, if it
/// is not cancelled the extensions are deinited and then dropped.
///
/// Deinit happens from the last inited extension first, so in reverse of init order, the [drop] happens in undefined order. Deinit is not called
/// if the app thread is unwinding from a panic, the extensions will just be dropped in this case.
///
/// # Resize Loop
///
/// The app enters a special loop when a window is resizing,
///
/// [`init`]: AppExtension::init
/// [`event_preview`]: AppExtension::event_preview
/// [`event_ui`]: AppExtension::event_ui
/// [`event`]: AppExtension::event
/// [`update_preview`]: AppExtension::update_preview
/// [`update_ui`]: AppExtension::update_ui
/// [`update`]: AppExtension::update
/// [`layout`]: AppExtension::layout
/// [`render`]: AppExtension::event
/// [`deinit`]: AppExtension::deinit
/// [drop]: Drop
/// [update is requested]: Updates::update
/// [init]: #1-init
/// [events]: #2-events
/// [updates]: #3-updates
/// [layout]: #3-layout
/// [render]: #5-render
/// [`RawFrameRenderedEvent`]: raw_events::RawFrameRenderedEvent
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

    /// Called after every sequence of updates if layout was requested.
    #[inline]
    fn layout(&mut self, ctx: &mut AppContext) {
        let _ = ctx;
    }

    /// Called after every sequence of updates and layout if render was requested.
    #[inline]
    fn render(&mut self, ctx: &mut AppContext) {
        let _ = ctx;
    }

    /// Called when the application is shutdown.
    ///
    /// Update requests and event notifications generated during this call are ignored,
    /// the extensions will be dropped after every extension received this call.
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
    fn layout_boxed(&mut self, ctx: &mut AppContext);
    fn render_boxed(&mut self, ctx: &mut AppContext);
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

    fn layout_boxed(&mut self, ctx: &mut AppContext) {
        self.layout(ctx);
    }

    fn render_boxed(&mut self, ctx: &mut AppContext) {
        self.render(ctx);
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

    fn layout(&mut self, ctx: &mut AppContext) {
        self.as_mut().layout_boxed(ctx);
    }

    fn render(&mut self, ctx: &mut AppContext) {
        self.as_mut().render_boxed(ctx);
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
    /// Arguments for [`ShutdownRequestedEvent`].
    pub struct ShutdownRequestedArgs {
        ..
        /// Always true.
        fn concerns_widget(&self, _: &mut WidgetContext) -> bool {
            true
        }
    }
}

event! {
    /// Cancellable event raised when app shutdown is requested.
    ///
    /// App shutdown can be requested using the [`AppProcess`] service, some extensions
    /// also request shutdown if some conditions are met, [`WindowManager`] requests shutdown
    /// after the last window is closed for example.
    ///
    /// Shutdown can be cancelled using the [`ShutdownRequestedArgs::cancel`] method.
    pub ShutdownRequestedEvent: ShutdownRequestedArgs;
}

/// Defines and runs an application.
///
/// # View Process
///
/// A view-process must be initialized before creating an app. Panics on `run` if there is
/// not view-process, also panics if the current process is executing as a view-process.
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

fn assert_not_view_process() {
    if zero_ui_view_api::ViewConfig::from_env().is_some() {
        panic!("cannot start App in view-process");
    }
}

// In release mode we use generics tricks to compile all app extensions with
// static dispatch optimized to a direct call to the extension handle.
#[cfg(not(debug_assertions))]
impl App {
    /// Application without any extension.
    #[inline]
    pub fn blank() -> AppExtended<()> {
        assert_not_view_process();
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
        assert_not_view_process();
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

/// Cancellation message of a [shutdown request].
///
/// [shutdown request]: AppProcess::shutdown
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
    /// The [`ShutdownRequestedEvent`] will be raised, and if not cancelled the app will shutdown.
    ///
    /// Returns a response variable that is updated once with the unit value [`ShutdownCancelled`]
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
    /// By the default the current executable is started again as a *View Process*, you can use
    /// two executables instead, by setting this value.
    ///
    /// Note that the `view_process_exe` must start a view server and both
    /// executables must be build using the same exact [`VERSION`].
    ///
    /// [`VERSION`]: zero_ui_view_api::VERSION  
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
        let app = RunningApp::start(self.extensions.boxed(), false, with_renderer, self.view_process_exe);

        HeadlessApp { app }
    }
}

/// Represents a running app controlled by an external event loop.
struct RunningApp<E: AppExtension> {
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

        let _s = tracing::debug_span!("App::start").entered();

        let (sender, receiver) = AppEventSender::new();

        let mut owned_ctx = OwnedAppContext::instance(sender);

        let mut ctx = owned_ctx.borrow();
        ctx.services.register(AppProcess::new(ctx.updates.sender()));

        let device_events = extensions.enable_device_events();

        if is_headed {
            debug_assert!(with_renderer);

            let view_evs_sender = ctx.updates.sender();
            let view_app = view_process::ViewProcess::start(view_process_exe, device_events, false, move |ev| {
                let _ = view_evs_sender.send_view_event(ev);
            });
            ctx.services.register(view_app);
        } else if with_renderer {
            let view_evs_sender = ctx.updates.sender();
            let renderer = view_process::ViewProcess::start(view_process_exe, false, true, move |ev| {
                let _ = view_evs_sender.send_view_event(ev);
            });
            ctx.services.register(renderer);
        }

        {
            let _s = tracing::debug_span!("extensions.init").entered();
            extensions.init(&mut ctx);
        }

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
        if let ControlFlow::Exit = self.update(&mut ()) {
            return;
        }
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
    pub fn notify_event<Ev: crate::event::Event, O: AppEventObserver>(&mut self, event: Ev, args: Ev::Args, observer: &mut O) {
        Self::notify_event_(&mut self.owned_ctx.borrow(), &mut self.extensions, event, args, observer);
        self.maybe_has_updates = true;
    }
    fn notify_event_<Ev: crate::event::Event, O: AppEventObserver>(
        ctx: &mut AppContext,
        extensions: &mut E,
        _event: Ev,
        args: Ev::Args,
        observer: &mut O,
    ) {
        let _scope = tracing::trace_span!("notify_event", event = type_name::<Ev>()).entered();

        let update = EventUpdate::<Ev>(args);

        extensions.event_preview(ctx, &update);
        observer.event_preview(ctx, &update);
        let update = update.boxed();
        Events::on_pre_events(ctx, &update);
        let update = EventUpdate::<Ev>(update.unbox_for::<Ev>().unwrap());

        extensions.event_ui(ctx, &update);
        observer.event_ui(ctx, &update);

        extensions.event(ctx, &update);
        observer.event(ctx, &update);
        Events::on_events(ctx, &update.boxed());
    }

    fn device_id(&mut self, id: zero_ui_view_api::DeviceId) -> DeviceId {
        self.ctx().services.req::<view_process::ViewProcess>().device_id(id)
    }

    /// Repeatedly sleeps-waits for app events until the control flow changes to something other than [`Poll`].
    ///
    /// This method also manages timers, awaking when a timer deadline elapses and causing an update cycle.
    ///
    /// [`Poll`]: ControlFlow::Poll
    #[inline]
    pub fn poll<O: AppEventObserver>(&mut self, observer: &mut O) -> ControlFlow {
        let mut flow = ControlFlow::Poll;

        while let ControlFlow::Poll = flow {
            let idle = tracing::trace_span!("<poll-idle>").entered();

            let ev = if let Some(timer) = self.wake_time {
                match self.receiver.recv_deadline(timer) {
                    Ok(ev) => ev,
                    Err(e) => match e {
                        flume::RecvTimeoutError::Timeout => {
                            // update timers
                            self.maybe_has_updates = true;
                            flow = self.update(observer);
                            continue;
                        }
                        flume::RecvTimeoutError::Disconnected => panic!("app events channel disconnected"),
                    },
                }
            } else {
                self.receiver.recv().expect("app events channel disconnected")
            };

            drop(idle);

            flow = if !matches!(ev, AppEvent::ViewEvent(view_process::Event::EventsCleared)) || self.receiver.is_empty() {
                // notify event, ignores `EventsCleared` if there is already more events in the channel.
                self.app_event(ev, observer)
            } else {
                ControlFlow::Poll
            };
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
        let _s = tracing::trace_span!("app_event").entered();

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
    fn view_event<O: AppEventObserver>(&mut self, ev: zero_ui_view_api::Event, observer: &mut O) -> ControlFlow {
        let _s = tracing::debug_span!("view_event", ?ev).entered();

        use raw_device_events::*;
        use raw_events::*;
        use zero_ui_view_api::Event;

        fn window_id(id: zero_ui_view_api::WindowId) -> WindowId {
            unsafe { WindowId::from_raw(id) }
        }

        match ev {
            Event::EventsCleared => {
                return self.update(observer);
            }
            Event::FrameRendered {
                window: w_id,
                frame: frame_id,
                frame_image,
                cursor_hits,
            } => {
                let image = frame_image.map(|img| {
                    let view = self.ctx().services.view_process();
                    view.on_frame_image(img)
                });
                let args = RawFrameRenderedArgs::now(window_id(w_id), frame_id, image, cursor_hits);
                self.notify_event(RawFrameRenderedEvent, args, observer);
                // `FrameRendered` is not followed by a `EventsCleared`.
                return self.update(observer);
            }
            Event::WindowResized { window: w_id, size, cause } => {
                let args = RawWindowResizedArgs::now(window_id(w_id), size, cause);
                self.notify_event(RawWindowResizedEvent, args, observer);
                // view-process blocks waiting for a frame on resize, so update as early
                // as possible to generate this frame
                return self.update(observer);
            }
            Event::WindowMoved {
                window: w_id,
                position,
                cause,
            } => {
                let args = RawWindowMovedArgs::now(window_id(w_id), position, cause);
                self.notify_event(RawWindowMovedEvent, args, observer);
            }
            Event::WindowStateChanged {
                window: w_id,
                state,
                cause,
            } => {
                let args = RawWindowStateChangedArgs::now(window_id(w_id), state, cause);
                self.notify_event(RawWindowStateChangedEvent, args, observer);
            }
            Event::DroppedFile { window: w_id, file } => {
                let args = RawDroppedFileArgs::now(window_id(w_id), file);
                self.notify_event(RawDroppedFileEvent, args, observer);
            }
            Event::HoveredFile { window: w_id, file } => {
                let args = RawHoveredFileArgs::now(window_id(w_id), file);
                self.notify_event(RawHoveredFileEvent, args, observer);
            }
            Event::HoveredFileCancelled(w_id) => {
                let args = RawHoveredFileCancelledArgs::now(window_id(w_id));
                self.notify_event(RawHoveredFileCancelledEvent, args, observer);
            }
            Event::ReceivedCharacter(w_id, c) => {
                let args = RawCharInputArgs::now(window_id(w_id), c);
                self.notify_event(RawCharInputEvent, args, observer);
            }
            Event::Focused { window: w_id, focused } => {
                let args = RawWindowFocusArgs::now(window_id(w_id), focused);
                self.notify_event(RawWindowFocusEvent, args, observer);
            }
            Event::KeyboardInput {
                window: w_id,
                device: d_id,
                scan_code,
                state,
                key,
            } => {
                let args = RawKeyInputArgs::now(window_id(w_id), self.device_id(d_id), scan_code, state, key);
                self.notify_event(RawKeyInputEvent, args, observer);
            }
            Event::ModifiersChanged { window: w_id, state } => {
                let args = RawModifiersChangedArgs::now(window_id(w_id), state);
                self.notify_event(RawModifiersChangedEvent, args, observer);
            }
            Event::CursorMoved {
                window: w_id,
                device: d_id,
                position,
                hit_test,
                frame,
            } => {
                let args = RawCursorMovedArgs::now(window_id(w_id), self.device_id(d_id), position, hit_test, frame);
                self.notify_event(RawCursorMovedEvent, args, observer);
            }
            Event::CursorEntered {
                window: w_id,
                device: d_id,
            } => {
                let args = RawCursorArgs::now(window_id(w_id), self.device_id(d_id));
                self.notify_event(RawCursorEnteredEvent, args, observer);
            }
            Event::CursorLeft {
                window: w_id,
                device: d_id,
            } => {
                let args = RawCursorArgs::now(window_id(w_id), self.device_id(d_id));
                self.notify_event(RawCursorLeftEvent, args, observer);
            }
            Event::MouseWheel {
                window: w_id,
                device: d_id,
                delta,
                phase,
            } => {
                // TODO
                let _ = (delta, phase);
                let args = RawMouseWheelArgs::now(window_id(w_id), self.device_id(d_id));
                self.notify_event(RawMouseWheelEvent, args, observer);
            }
            Event::MouseInput {
                window: w_id,
                device: d_id,
                state,
                button,
            } => {
                let args = RawMouseInputArgs::now(window_id(w_id), self.device_id(d_id), state, button);
                self.notify_event(RawMouseInputEvent, args, observer);
            }
            Event::TouchpadPressure {
                window: w_id,
                device: d_id,
                pressure,
                stage,
            } => {
                // TODO
                let _ = (pressure, stage);
                let args = RawTouchpadPressureArgs::now(window_id(w_id), self.device_id(d_id));
                self.notify_event(RawTouchpadPressureEvent, args, observer);
            }
            Event::AxisMotion(w_id, d_id, axis, value) => {
                let args = RawAxisMotionArgs::now(window_id(w_id), self.device_id(d_id), axis, value);
                self.notify_event(RawAxisMotionEvent, args, observer);
            }
            Event::Touch(w_id, d_id, phase, pos, force, finger_id) => {
                // TODO
                let _ = (phase, pos, force, finger_id);
                let args = RawTouchArgs::now(window_id(w_id), self.device_id(d_id));
                self.notify_event(RawTouchEvent, args, observer);
            }
            Event::ScaleFactorChanged {
                window: w_id,
                scale_factor,
            } => {
                let args = RawWindowScaleFactorChangedArgs::now(window_id(w_id), scale_factor);
                self.notify_event(RawWindowScaleFactorChangedEvent, args, observer);
            }
            Event::MonitorsChanged(monitors) => {
                let view = self.ctx().services.req::<view_process::ViewProcess>();
                let monitors: Vec<_> = monitors.into_iter().map(|(id, info)| (view.monitor_id(id), info)).collect();
                let args = RawMonitorsChangedArgs::now(monitors);
                self.notify_event(RawMonitorsChangedEvent, args, observer);
            }
            Event::WindowThemeChanged(w_id, theme) => {
                let args = RawWindowThemeChangedArgs::now(window_id(w_id), theme);
                self.notify_event(RawWindowThemeChangedEvent, args, observer);
            }
            Event::WindowCloseRequested(w_id) => {
                let args = RawWindowCloseRequestedArgs::now(window_id(w_id));
                self.notify_event(RawWindowCloseRequestedEvent, args, observer);
            }
            Event::WindowClosed(w_id) => {
                let args = RawWindowCloseArgs::now(window_id(w_id));
                self.notify_event(RawWindowCloseEvent, args, observer);
            }
            Event::ImageMetadataLoaded { image: id, size, ppi } => {
                let view = self.ctx().services.req::<view_process::ViewProcess>();
                if let Some(img) = view.on_image_metadata_loaded(id, size, ppi) {
                    let args = RawImageArgs::now(img);
                    self.notify_event(RawImageMetadataLoadedEvent, args, observer);
                }
            }
            Event::ImagePartiallyLoaded {
                image: id,
                partial_size,
                ppi,
                opaque,
                partial_bgra8,
            } => {
                let view = self.ctx().services.req::<view_process::ViewProcess>();
                if let Some(img) = view.on_image_partially_loaded(id, partial_size, ppi, opaque, partial_bgra8) {
                    let args = RawImageArgs::now(img);
                    self.notify_event(RawImagePartiallyLoadedEvent, args, observer);
                }
            }
            Event::ImageLoaded(image) => {
                let view = self.ctx().services.req::<view_process::ViewProcess>();
                if let Some(img) = view.on_image_loaded(image) {
                    let args = RawImageArgs::now(img);
                    self.notify_event(RawImageLoadedEvent, args, observer);
                }
            }
            Event::ImageLoadError { image: id, error } => {
                let view = self.ctx().services.req::<view_process::ViewProcess>();
                if let Some(img) = view.on_image_error(id, error) {
                    let args = RawImageArgs::now(img);
                    self.notify_event(RawImageLoadErrorEvent, args, observer);
                }
            }
            Event::ImageEncoded { image: id, format, data } => {
                let view = self.ctx().services.req::<view_process::ViewProcess>();
                view.on_image_encoded(id, format, data)
            }
            Event::ImageEncodeError { image: id, format, error } => {
                let view = self.ctx().services.req::<view_process::ViewProcess>();
                view.on_image_encode_error(id, format, error);
            }
            Event::FrameImageReady {
                window: w_id,
                frame: frame_id,
                image: image_id,
                selection,
            } => {
                let view = self.ctx().services.req::<view_process::ViewProcess>();
                if let Some(img) = view.on_frame_image_ready(image_id) {
                    let args = RawFrameImageReadyArgs::now(img, window_id(w_id), frame_id, selection);
                    self.notify_event(RawFrameImageReadyEvent, args, observer);
                }
            }

            // config events
            Event::FontsChanged => {
                let args = RawFontChangedArgs::now();
                self.notify_event(RawFontChangedEvent, args, observer);
            }
            Event::TextAaChanged(aa) => {
                let args = RawTextAaChangedArgs::now(aa);
                self.notify_event(RawTextAaChangedEvent, args, observer);
            }
            Event::MultiClickConfigChanged(cfg) => {
                let args = RawMultiClickConfigChangedArgs::now(cfg);
                self.notify_event(RawMultiClickConfigChangedEvent, args, observer);
            }
            Event::AnimationEnabledChanged(enabled) => {
                let args = RawAnimationEnabledChangedArgs::now(enabled);
                self.notify_event(RawAnimationEnabledChangedEvent, args, observer);
            }
            Event::KeyRepeatDelayChanged(delay) => {
                let args = RawKeyRepeatDelayChangedArgs::now(delay);
                self.notify_event(RawKeyRepeatDelayChangedEvent, args, observer);
            }

            // `device_events`
            Event::DeviceAdded(d_id) => {
                let args = DeviceArgs::now(self.device_id(d_id));
                self.notify_event(DeviceAddedEvent, args, observer);
            }
            Event::DeviceRemoved(d_id) => {
                let args = DeviceArgs::now(self.device_id(d_id));
                self.notify_event(DeviceRemovedEvent, args, observer);
            }
            Event::DeviceMouseMotion { device: d_id, delta } => {
                let args = MouseMotionArgs::now(self.device_id(d_id), delta);
                self.notify_event(MouseMotionEvent, args, observer);
            }
            Event::DeviceMouseWheel { device: d_id, delta } => {
                let args = MouseWheelArgs::now(self.device_id(d_id), delta);
                self.notify_event(MouseWheelEvent, args, observer);
            }
            Event::DeviceMotion { device: d_id, axis, value } => {
                let args = MotionArgs::now(self.device_id(d_id), axis, value);
                self.notify_event(MotionEvent, args, observer);
            }
            Event::DeviceButton {
                device: d_id,
                button,
                state,
            } => {
                let args = ButtonArgs::now(self.device_id(d_id), button, state);
                self.notify_event(ButtonEvent, args, observer);
            }
            Event::DeviceKey {
                device: d_id,
                scan_code,
                state,
                key,
            } => {
                let args = KeyArgs::now(self.device_id(d_id), scan_code, state, key);
                self.notify_event(KeyEvent, args, observer);
            }
            Event::DeviceText(d_id, c) => {
                let args = TextArgs::now(self.device_id(d_id), c);
                self.notify_event(TextEvent, args, observer);
            }

            // Other
            Event::Respawned(g) => {
                let args = view_process::ViewProcessRespawnedArgs::now(g);
                self.notify_event(view_process::ViewProcessRespawnedEvent, args, observer);
                // `FrameRendered` is not followed by a `EventsCleared`.
                return self.update(observer);
            }

            Event::Disconnected(gen) => {
                self.ctx().services.view_process().handle_disconnect(gen);
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
            let _s = tracing::debug_span!("update-cycle").entered();

            self.maybe_has_updates = false;

            let mut update = false;
            let mut layout = false;
            let mut render = false;

            let mut limit = 100_000;
            loop {
                limit -= 1;
                if limit == 0 {
                    panic!("update loop polled 100,000 times, probably stuck in an infinite loop");
                }

                let u = self.owned_ctx.apply_updates();
                let mut ctx = self.owned_ctx.borrow();

                self.wake_time = u.wake_time;
                update |= u.update;
                layout |= u.layout;
                render |= u.render;

                if !u.events.is_empty() {
                    // does events raised by extensions.

                    let _s = tracing::trace_span!("events").entered();

                    for event in u.events {
                        let _s = tracing::debug_span!("event", ?event).entered();

                        self.extensions.event_preview(&mut ctx, &event);
                        observer.event_preview(&mut ctx, &event);
                        Events::on_pre_events(&mut ctx, &event);

                        self.extensions.event_ui(&mut ctx, &event);
                        observer.event_ui(&mut ctx, &event);

                        self.extensions.event(&mut ctx, &event);
                        observer.event(&mut ctx, &event);
                        Events::on_events(&mut ctx, &event);
                    }
                } else if update {
                    // check shutdown.
                    if let Some(r) = ctx.services.app_process().take_requests() {
                        let _s = tracing::debug_span!("shutdown_requested").entered();

                        let args = ShutdownRequestedArgs::now();

                        Self::notify_event_(&mut ctx, &mut self.extensions, ShutdownRequestedEvent, args.clone(), observer);

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

                    // does general updates.
                    {
                        let _s = tracing::trace_span!("update").entered();

                        self.extensions.update_preview(&mut ctx);
                        observer.update_preview(&mut ctx);
                        Updates::on_pre_updates(&mut ctx);

                        self.extensions.update_ui(&mut ctx);
                        observer.update_ui(&mut ctx);

                        self.extensions.update(&mut ctx);
                        observer.update(&mut ctx);
                        Updates::on_updates(&mut ctx);
                    }

                    update = false;
                } else if layout {
                    let _s = tracing::trace_span!("layout").entered();

                    self.extensions.layout(&mut ctx);
                    observer.layout(&mut ctx);

                    layout = false;
                } else if render {
                    let _s = tracing::trace_span!("render").entered();

                    self.extensions.render(&mut ctx);
                    observer.render(&mut ctx);

                    render = false;
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
}
impl<E: AppExtension> Drop for RunningApp<E> {
    fn drop(&mut self) {
        let _s = tracing::debug_span!("extensions.deinit").entered();
        let mut ctx = self.owned_ctx.borrow();
        self.extensions.deinit(&mut ctx);
    }
}

/// Desired next step of app main loop.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[must_use = "methods that return `ControlFlow` expect to be inside a controlled loop"]
pub enum ControlFlow {
    /// Immediately try to receive more app events.
    Poll,
    /// Sleep until an app event is received.
    ///
    /// Note that a deadline might be set in case a timer is running.
    Wait,
    /// Exit the loop and drop the app.
    Exit,
}

/// A headless app controller.
///
/// Headless apps don't cause external side-effects like visible windows and don't listen to system events.
/// They can be used for creating apps like a command line app that renders widgets, or for creating integration tests.
pub struct HeadlessApp {
    app: RunningApp<Box<dyn AppExtensionBoxed>>,
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
    /// [frame pixels]: crate::window::Windows::frame_image
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

    /// If device events are enabled in this app.
    pub fn device_events(&self) -> bool {
        self.app.device_events()
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

/// Observer for [`HeadlessApp::update_observed`].
///
/// This works like a temporary app extension that runs only for the update call.
pub trait AppEventObserver {
    /// Called for each raw event received.
    fn raw_event(&mut self, ctx: &mut AppContext, ev: &zero_ui_view_api::Event) {
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

    /// Called just after [`AppExtension::layout`].
    fn layout(&mut self, ctx: &mut AppContext) {
        let _ = ctx;
    }

    /// Called just after [`AppExtension::render`].
    fn render(&mut self, ctx: &mut AppContext) {
        let _ = ctx;
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
    fn layout(&mut self, ctx: &mut AppContext) {
        self.0.layout(ctx);
        self.1.layout(ctx);
    }

    #[inline]
    fn render(&mut self, ctx: &mut AppContext) {
        self.0.render(ctx);
        self.1.render(ctx);
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
    fn deinit(&mut self, ctx: &mut AppContext) {
        self.1.deinit(ctx);
        self.0.deinit(ctx);
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

    fn layout(&mut self, ctx: &mut AppContext) {
        for ext in self {
            ext.layout(ctx);
        }
    }

    fn render(&mut self, ctx: &mut AppContext) {
        for ext in self {
            ext.render(ctx);
        }
    }

    fn deinit(&mut self, ctx: &mut AppContext) {
        for ext in self.iter_mut().rev() {
            ext.deinit(ctx);
        }
    }
}

/// App events.
#[derive(Debug)]
pub(crate) enum AppEvent {
    /// Event from the View Process.
    ViewEvent(zero_ui_view_api::Event),
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
    fn send_view_event(&self, event: zero_ui_view_api::Event) -> Result<(), AppShutdown<AppEvent>> {
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

unique_id_64! {
    /// Unique identifier of a device event source.
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
impl fmt::Debug for DeviceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            f.debug_struct("DeviceId")
                .field("id", &self.get())
                .field("sequential", &self.sequential())
                .finish()
        } else {
            write!(f, "DeviceId({})", self.sequential())
        }
    }
}
impl fmt::Display for DeviceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DeviceId({})", self.get())
    }
}

/// View process controller types.
pub mod view_process {
    use std::cell::Cell;
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::time::Duration;
    use std::{cell::RefCell, rc::Rc};
    use std::{fmt, rc};

    use linear_map::LinearMap;
    use once_cell::unsync::OnceCell;

    use super::DeviceId;
    use crate::mouse::MultiClickConfig;
    use crate::render::FrameId;
    use crate::service::Service;
    use crate::task::SignalOnce;
    use crate::units::{DipPoint, DipSize, Factor, Px, PxPoint, PxRect, PxSize};
    use crate::window::{MonitorId, WindowId};
    use crate::{event, event_args};
    use zero_ui_view_api::webrender_api::{
        DocumentId, FontInstanceKey, FontInstanceOptions, FontInstancePlatformOptions, FontKey, FontVariation, HitTestResult, IdNamespace,
        ImageKey, PipelineId,
    };
    pub use zero_ui_view_api::{
        bytes_channel, ByteBuf, CursorIcon, Event, EventCause, FrameRequest, FrameUpdateRequest, HeadlessRequest, ImageDataFormat,
        ImagePpi, IpcBytesReceiver, IpcBytesSender, IpcSharedMemory, MonitorInfo, Respawned, TextAntiAliasing, VideoMode, ViewProcessGen,
        WindowOpenData, WindowRequest, WindowState, WindowTheme,
    };
    use zero_ui_view_api::{
        Controller, DeviceId as ApiDeviceId, DocumentRequest, ImageId, ImageLoadedData, MonitorId as ApiMonitorId, WindowId as ApiWindowId,
    };

    type Result<T> = std::result::Result<T, Respawned>;

    struct EncodeRequest {
        image_id: ImageId,
        format: String,
        listeners: Vec<flume::Sender<std::result::Result<Arc<Vec<u8>>, EncodeError>>>,
    }

    /// Reference to the running View Process.
    ///
    /// This is the lowest level API, used for implementing fundamental services and is a service available
    /// in headed apps or headless apps with renderer.
    ///
    /// This is a strong reference to the view process. The process shuts down when all clones of this struct drops.
    #[derive(Service, Clone)]
    pub struct ViewProcess(Rc<RefCell<ViewApp>>);
    struct ViewApp {
        process: zero_ui_view_api::Controller,
        device_ids: LinearMap<ApiDeviceId, DeviceId>,
        monitor_ids: LinearMap<ApiMonitorId, MonitorId>,

        data_generation: ViewProcessGen,

        loading_images: Vec<rc::Weak<ImageConnection>>,
        frame_images: Vec<rc::Weak<ImageConnection>>,
        encoding_images: Vec<EncodeRequest>,
    }
    impl ViewApp {
        #[must_use = "if `true` all current WinId, DevId and MonId are invalid"]
        fn check_generation(&mut self) -> bool {
            let gen = self.process.generation();
            let invalid = gen != self.data_generation;
            if invalid {
                self.data_generation = gen;
                self.device_ids.clear();
                self.monitor_ids.clear();
            }
            invalid
        }
    }
    impl ViewProcess {
        /// Spawn the View Process.
        pub(super) fn start<F>(view_process_exe: Option<PathBuf>, device_events: bool, headless: bool, on_event: F) -> Self
        where
            F: FnMut(Event) + Send + 'static,
        {
            let _s = tracing::debug_span!("ViewProcess::start").entered();

            let process = zero_ui_view_api::Controller::start(view_process_exe, device_events, headless, on_event);
            Self(Rc::new(RefCell::new(ViewApp {
                data_generation: process.generation(),
                process,
                device_ids: LinearMap::default(),
                monitor_ids: LinearMap::default(),
                loading_images: vec![],
                encoding_images: vec![],
                frame_images: vec![],
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
        pub fn open_window(&self, config: WindowRequest) -> Result<(ViewWindow, WindowOpenData)> {
            let _s = tracing::debug_span!("ViewProcess.open_window").entered();

            let mut app = self.0.borrow_mut();
            let _ = app.check_generation();

            let id = config.id;
            let data = app.process.open_window(config)?;

            let win = ViewWindow(Rc::new(WindowConnection {
                id,
                app: self.0.clone(),
                id_namespace: data.id_namespace,
                pipeline_id: data.pipeline_id,
                document_id: data.document_id,
                generation: app.data_generation,
            }));
            Ok((win, data))
        }

        /// Open a headless renderer and associate it with the `window_id`.
        ///
        /// Note that no actual window is created, only the renderer, the use of window-ids to identify
        /// this renderer is only for convenience.
        pub fn open_headless(&self, config: HeadlessRequest) -> Result<ViewHeadless> {
            let _s = tracing::debug_span!("ViewProcess.open_headless").entered();

            let mut app = self.0.borrow_mut();

            let id = config.id;
            let data = app.process.open_headless(config)?;

            Ok(ViewHeadless(
                Rc::new(WindowConnection {
                    id,
                    app: self.0.clone(),
                    id_namespace: data.id_namespace,
                    pipeline_id: data.pipeline_id,
                    document_id: data.document_id,
                    generation: app.data_generation,
                }),
                data.document_id,
            ))
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

        /// Translate `DevId` to `DeviceId`, generates a device id if it was unknown.
        pub(super) fn device_id(&self, id: ApiDeviceId) -> DeviceId {
            *self.0.borrow_mut().device_ids.entry(id).or_insert_with(DeviceId::new_unique)
        }

        /// Translate `MonId` to `MonitorId`, generates a monitor id if it was unknown.
        pub(super) fn monitor_id(&self, id: ApiMonitorId) -> MonitorId {
            *self.0.borrow_mut().monitor_ids.entry(id).or_insert_with(MonitorId::new_unique)
        }

        /// Translate `MonitorId` to `MonId`.
        pub(super) fn monitor_id_back(&self, monitor_id: MonitorId) -> Option<ApiMonitorId> {
            self.0
                .borrow()
                .monitor_ids
                .iter()
                .find(|(_, app_id)| **app_id == monitor_id)
                .map(|(id, _)| *id)
        }

        /// Reopen the view-process, causing an [`Event::Respawned`].
        pub fn respawn(&self) {
            self.0.borrow_mut().process.respawn()
        }

        /// Causes a panic in the view-process to test respawn.
        #[cfg(debug_assertions)]
        pub fn crash_view_process(&self) {
            self.0.borrow_mut().process.crash().expect_err("expected Respawn error");
        }

        /// Handle an [`Event::Disconnected`].
        ///
        /// The process will exit if the view-process was killed by the user.
        pub fn handle_disconnect(&mut self, gen: ViewProcessGen) {
            self.0.borrow_mut().process.handle_disconnect(gen)
        }

        /// Gets the current view-process generation.
        pub fn generation(&self) -> ViewProcessGen {
            self.0.borrow().process.generation()
        }

        /// Send an image for decoding.
        ///
        /// This function returns immediately, the [`ViewImage`] will update when
        /// [`Event::ImageMetadataLoaded`], [`Event::ImageLoaded`] and [`Event::ImageLoadError`] events are received.
        pub fn add_image(&self, format: ImageDataFormat, data: IpcSharedMemory, max_decoded_size: u64) -> Result<ViewImage> {
            let mut app = self.0.borrow_mut();
            let id = app.process.add_image(format, data, max_decoded_size)?;
            let img = ViewImage(Rc::new(ImageConnection {
                id,
                generation: app.process.generation(),
                app: Some(self.0.clone()),
                size: Cell::new(PxSize::zero()),
                partial_size: Cell::new(PxSize::zero()),
                ppi: Cell::new(None),
                opaque: Cell::new(false),
                partial_bgra8: RefCell::new(None),
                bgra8: OnceCell::new(),
                done_signal: SignalOnce::new(),
            }));
            app.loading_images.push(Rc::downgrade(&img.0));
            Ok(img)
        }

        /// Starts sending an image for *progressive* decoding.
        ///
        /// This function returns immediately, the [`ViewImage`] will update when
        /// [`Event::ImageMetadataLoaded`], [`Event::ImagePartiallyLoaded`],
        /// [`Event::ImageLoaded`] and [`Event::ImageLoadError`] events are received.
        pub fn add_image_pro(&self, format: ImageDataFormat, data: IpcBytesReceiver, max_decoded_size: u64) -> Result<ViewImage> {
            let mut app = self.0.borrow_mut();
            let id = app.process.add_image_pro(format, data, max_decoded_size)?;
            let img = ViewImage(Rc::new(ImageConnection {
                id,
                generation: app.process.generation(),
                app: Some(self.0.clone()),
                size: Cell::new(PxSize::zero()),
                partial_size: Cell::new(PxSize::zero()),
                ppi: Cell::new(None),
                opaque: Cell::new(false),
                partial_bgra8: RefCell::new(None),
                bgra8: OnceCell::new(),
                done_signal: SignalOnce::new(),
            }));
            app.loading_images.push(Rc::downgrade(&img.0));
            Ok(img)
        }

        /// Returns a list of image decoders supported by the view-process backend.
        ///
        /// Each string is the lower-case file extension.
        pub fn image_decoders(&self) -> Result<Vec<String>> {
            self.0.borrow_mut().process.image_decoders()
        }

        /// Returns a list of image encoders supported by the view-process backend.
        ///
        /// Each string is the lower-case file extension.
        pub fn image_encoders(&self) -> Result<Vec<String>> {
            self.0.borrow_mut().process.image_encoders()
        }

        fn loading_image_index(&self, id: ImageId) -> Option<usize> {
            let mut app = self.0.borrow_mut();

            // cleanup
            app.loading_images.retain(|i| i.strong_count() > 0);

            app.loading_images.iter().position(|i| i.upgrade().unwrap().id == id)
        }

        pub(super) fn on_image_metadata_loaded(&self, id: ImageId, size: PxSize, ppi: ImagePpi) -> Option<ViewImage> {
            if let Some(i) = self.loading_image_index(id) {
                let app = self.0.borrow();
                let img = app.loading_images[i].upgrade().unwrap();
                img.size.set(size);
                img.ppi.set(ppi);
                Some(ViewImage(img))
            } else {
                None
            }
        }

        pub(super) fn on_image_partially_loaded(
            &self,
            id: ImageId,
            partial_size: PxSize,
            ppi: ImagePpi,
            opaque: bool,
            partial_bgra8: IpcSharedMemory,
        ) -> Option<ViewImage> {
            if let Some(i) = self.loading_image_index(id) {
                let app = self.0.borrow();
                let img = app.loading_images[i].upgrade().unwrap();
                img.partial_size.set(partial_size);
                img.ppi.set(ppi);
                img.opaque.set(opaque);
                *img.partial_bgra8.borrow_mut() = Some(partial_bgra8);
                Some(ViewImage(img))
            } else {
                None
            }
        }

        pub(super) fn on_image_loaded(&self, data: ImageLoadedData) -> Option<ViewImage> {
            if let Some(i) = self.loading_image_index(data.id) {
                let mut app = self.0.borrow_mut();
                let img = app.loading_images.swap_remove(i).upgrade().unwrap();
                img.size.set(data.size);
                img.partial_size.set(data.size);
                img.ppi.set(data.ppi);
                img.opaque.set(data.opaque);
                img.bgra8.set(Ok(data.bgra8)).unwrap();
                *img.partial_bgra8.borrow_mut() = None;
                img.done_signal.set();
                Some(ViewImage(img))
            } else {
                None
            }
        }

        pub(super) fn on_image_error(&self, id: ImageId, error: String) -> Option<ViewImage> {
            if let Some(i) = self.loading_image_index(id) {
                let mut app = self.0.borrow_mut();
                let img = app.loading_images.swap_remove(i).upgrade().unwrap();
                img.bgra8.set(Err(error)).unwrap();
                img.done_signal.set();
                Some(ViewImage(img))
            } else {
                None
            }
        }

        pub(crate) fn on_frame_image(&self, data: ImageLoadedData) -> ViewImage {
            let bgra8 = OnceCell::new();
            let _ = bgra8.set(Ok(data.bgra8));
            ViewImage(Rc::new(ImageConnection {
                id: data.id,
                generation: self.generation(),
                app: Some(self.0.clone()),
                size: Cell::new(data.size),
                partial_size: Cell::new(data.size),
                ppi: Cell::new(data.ppi),
                opaque: Cell::new(data.opaque),
                partial_bgra8: RefCell::new(None),
                bgra8,
                done_signal: SignalOnce::new_set(),
            }))
        }

        pub(super) fn on_frame_image_ready(&self, id: ImageId) -> Option<ViewImage> {
            let mut app = self.0.borrow_mut();

            // cleanup
            app.frame_images.retain(|i| i.strong_count() > 0);

            let i = app.frame_images.iter().position(|i| i.upgrade().unwrap().id == id);

            if let Some(i) = i {
                Some(ViewImage(app.frame_images.swap_remove(i).upgrade().unwrap()))
            } else {
                None
            }
        }

        pub(super) fn on_image_encoded(&self, id: ImageId, format: String, data: Vec<u8>) {
            self.on_image_encode_result(id, format, Ok(Arc::new(data)));
        }
        pub(super) fn on_image_encode_error(&self, id: ImageId, format: String, error: String) {
            self.on_image_encode_result(id, format, Err(EncodeError::Encode(error)));
        }
        fn on_image_encode_result(&self, id: ImageId, format: String, result: std::result::Result<Arc<Vec<u8>>, EncodeError>) {
            let mut app = self.0.borrow_mut();
            app.encoding_images.retain(move |r| {
                let done = r.image_id == id && r.format == format;
                if done {
                    for sender in &r.listeners {
                        let _ = sender.send(result.clone());
                    }
                }
                !done
            })
        }
    }

    struct ImageConnection {
        id: ImageId,
        generation: ViewProcessGen,
        app: Option<Rc<RefCell<ViewApp>>>,

        size: Cell<PxSize>,
        partial_size: Cell<PxSize>,
        ppi: Cell<ImagePpi>,
        opaque: Cell<bool>,

        partial_bgra8: RefCell<Option<IpcSharedMemory>>,
        bgra8: OnceCell<std::result::Result<IpcSharedMemory, String>>,

        done_signal: SignalOnce,
    }
    impl ImageConnection {
        fn alive(&self) -> bool {
            if let Some(app) = &self.app {
                self.generation == app.borrow().process.generation()
            } else {
                true
            }
        }
    }
    impl Drop for ImageConnection {
        fn drop(&mut self) {
            if let Some(app) = self.app.take() {
                let mut app = app.borrow_mut();
                if app.process.generation() == self.generation {
                    let _ = app.process.forget_image(self.id);
                }
            }
        }
    }

    /// Connection to an image loading or loaded in the View Process.
    ///
    /// This is a strong reference to the image connection. The image is removed from the View Process cache
    /// when all clones of this struct drops.
    #[derive(Clone)]
    pub struct ViewImage(Rc<ImageConnection>);
    impl PartialEq for ViewImage {
        fn eq(&self, other: &Self) -> bool {
            self.0.id == other.0.id && self.0.generation == other.0.generation
        }
    }
    impl Eq for ViewImage {}
    impl fmt::Debug for ViewImage {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("ViewImage")
                .field("loaded", &self.is_loaded())
                .field("error", &self.error())
                .field("size", &self.size())
                .field("dpi", &self.ppi())
                .field("opaque", &self.is_opaque())
                .field("generation", &self.generation())
                .field("alive", &self.alive())
                .finish_non_exhaustive()
        }
    }
    impl ViewImage {
        /// Image id.
        pub fn id(&self) -> ImageId {
            self.0.id
        }

        /// If the image does not actually exists in the view-process.
        pub fn is_dummy(&self) -> bool {
            self.0.app.is_none()
        }

        /// Returns `true` if the image has successfully decoded.
        #[inline]
        pub fn is_loaded(&self) -> bool {
            self.0.bgra8.get().map(|r| r.is_ok()).unwrap_or(false)
        }

        /// Returns `true` if the image is progressively decoding and has partially decoded.
        pub fn is_partially_loaded(&self) -> bool {
            self.0.partial_bgra8.borrow().is_some()
        }

        /// if [`error`] is `Some`.
        ///
        /// [`error`]: Self::error
        #[inline]
        pub fn is_error(&self) -> bool {
            self.0.bgra8.get().map(|r| r.is_err()).unwrap_or(false)
        }

        /// Returns the load error if one happened.
        #[inline]
        pub fn error(&self) -> Option<&str> {
            self.0.bgra8.get().and_then(|s| s.as_ref().err().map(|s| s.as_str()))
        }

        /// Returns the pixel size, or zero if is not loaded or error.
        #[inline]
        pub fn size(&self) -> PxSize {
            self.0.size.get()
        }

        /// Actual size of the current pixels.
        ///
        /// Can be different from [`size`] if the image is progressively decoding.
        ///
        /// [`size`]: Self::size
        #[inline]
        pub fn partial_size(&self) -> PxSize {
            self.0.partial_size.get()
        }

        /// Returns the "pixels-per-inch" metadata associated with the image, or `None` if not loaded or error or no
        /// metadata provided by decoder.
        #[inline]
        pub fn ppi(&self) -> ImagePpi {
            self.0.ppi.get()
        }

        /// Returns if the image is fully opaque.
        #[inline]
        pub fn is_opaque(&self) -> bool {
            self.0.opaque.get()
        }

        /// Copy the partially decoded pixels if the image is progressively decoding
        /// and has not finished decoding.
        pub fn partial_bgra8(&self) -> Option<Vec<u8>> {
            (*self.0.partial_bgra8.borrow()).as_ref().map(|r| r[..].to_vec())
        }

        /// Reference the decoded and pre-multiplied BGRA8 bytes of the image.
        ///
        /// Returns `None` until the image is fully loaded. Use [`partial_bgra8`] to copy
        /// partially decoded bytes.
        ///
        /// [`partial_bgra8`]: Self::partial_bgra8
        #[inline]
        pub fn bgra8(&self) -> Option<&[u8]> {
            self.0.bgra8.get().and_then(|r| r.as_ref().ok()).map(|m| &m[..])
        }

        /// Clone the reference to the inter-process shared memory that contains
        /// the image BGRA8 pixel buffer.
        pub fn shared_bgra8(&self) -> Option<IpcSharedMemory> {
            self.0.bgra8.get().and_then(|r| r.as_ref().ok()).cloned()
        }

        /// Returns the view-process generation on which the image is loaded.
        #[inline]
        pub fn generation(&self) -> ViewProcessGen {
            self.0.generation
        }

        /// Returns `true` if this window connection is still valid.
        ///
        /// The connection can be permanently lost in case the "view-process" respawns, in this
        /// case all methods will return [`Respawned`], and you must discard this connection and
        /// create a new one.
        #[inline]
        pub fn alive(&self) -> bool {
            self.0.alive()
        }

        /// Creates a [`WeakViewImage`].
        #[inline]
        pub fn downgrade(&self) -> WeakViewImage {
            WeakViewImage(Rc::downgrade(&self.0))
        }

        /// Create a dummy image in the loaded or error state.
        pub fn dummy(error: Option<String>) -> Self {
            let bgra8 = OnceCell::new();

            if let Some(e) = error {
                bgra8.set(Err(e)).unwrap();
            } else {
                // not zero-sized due to issue: TODO
                bgra8.set(Ok(IpcSharedMemory::from_byte(0, 1))).unwrap();
            }

            ViewImage(Rc::new(ImageConnection {
                id: 0,
                generation: 0,
                app: None,
                size: Cell::new(PxSize::zero()),
                partial_size: Cell::new(PxSize::zero()),
                ppi: Cell::new(None),
                opaque: Cell::new(true),
                partial_bgra8: RefCell::new(None),
                bgra8,
                done_signal: SignalOnce::new_set(),
            }))
        }

        /// Returns a future that awaits until this image is loaded or encountered an error.
        pub fn awaiter(&self) -> impl std::future::Future<Output = ()> + Send + Sync + 'static {
            self.0.done_signal.clone()
        }

        /// Tries to encode the image to the format.
        ///
        /// The `format` must be one of the [`image_encoders`] supported by the view-process backend.
        ///
        /// [`image_encoders`]: View::image_encoders.
        pub async fn encode(&self, format: String) -> std::result::Result<Arc<Vec<u8>>, EncodeError> {
            self.awaiter().await;

            if let Some(e) = self.error() {
                return Err(EncodeError::Encode(e.to_owned()));
            }

            if let Some(app) = &self.0.app {
                let mut app = app.borrow_mut();
                app.process.encode_image(self.0.id, format.clone())?;

                let (sender, receiver) = flume::bounded(1);
                if let Some(entry) = app
                    .encoding_images
                    .iter_mut()
                    .find(|r| r.image_id == self.0.id && r.format == format)
                {
                    entry.listeners.push(sender);
                } else {
                    app.encoding_images.push(EncodeRequest {
                        image_id: self.0.id,
                        format,
                        listeners: vec![sender],
                    });
                }
                drop(app);
                receiver.recv_async().await?
            } else {
                Err(EncodeError::Dummy)
            }
        }

        pub(crate) fn done_signal(&self) -> SignalOnce {
            self.0.done_signal.clone()
        }
    }

    /// Error returned by [`ViewImage::encode`].
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum EncodeError {
        /// Encode error.
        Encode(String),
        /// Attempted to encode dummy image.
        ///
        /// In a headless-app without renderer all images are dummy because there is no
        /// view-process backend running.
        Dummy,
        /// View-process respawned while waiting for encoded data.
        Respawned,
    }
    impl From<String> for EncodeError {
        fn from(e: String) -> Self {
            EncodeError::Encode(e)
        }
    }
    impl From<Respawned> for EncodeError {
        fn from(_: Respawned) -> Self {
            EncodeError::Respawned
        }
    }
    impl From<flume::RecvError> for EncodeError {
        fn from(_: flume::RecvError) -> Self {
            EncodeError::Respawned
        }
    }
    impl fmt::Display for EncodeError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                EncodeError::Encode(e) => write!(f, "{}", e),
                EncodeError::Dummy => write!(f, "cannot encode dummy image"),
                EncodeError::Respawned => write!(f, "{}", Respawned),
            }
        }
    }
    impl std::error::Error for EncodeError {}

    /// Connection to an image loading or loaded in the View Process.
    ///
    /// The image is removed from the View Process cache when all clones of [`ViewImage`] drops, but
    /// if there is another image pointer holding the image, this weak pointer can be upgraded back
    /// to a strong connection to the image.
    #[derive(Clone)]
    pub struct WeakViewImage(rc::Weak<ImageConnection>);
    impl WeakViewImage {
        /// Attempt to upgrade the weak pointer to the image to a full image.
        ///
        /// Returns `Some` if the is at least another [`ViewImage`] holding the image alive.
        #[inline]
        pub fn upgrade(&self) -> Option<ViewImage> {
            self.0.upgrade().map(ViewImage)
        }
    }

    struct WindowConnection {
        id: ApiWindowId,
        id_namespace: IdNamespace,
        pipeline_id: PipelineId,
        document_id: DocumentId,
        generation: ViewProcessGen,
        app: Rc<RefCell<ViewApp>>,
    }
    impl WindowConnection {
        fn alive(&self) -> bool {
            self.generation == self.app.borrow().process.generation()
        }

        fn call<R>(&self, f: impl FnOnce(ApiWindowId, &mut Controller) -> Result<R>) -> Result<R> {
            let mut app = self.app.borrow_mut();
            if app.check_generation() {
                Err(Respawned)
            } else {
                f(self.id, &mut app.process)
            }
        }
    }
    impl Drop for WindowConnection {
        fn drop(&mut self) {
            let mut app = self.app.borrow_mut();
            if self.generation == app.process.generation() {
                let _ = app.process.close_window(self.id);
            }
        }
    }

    /// Connection to a window open in the View Process.
    ///
    /// This is a strong reference to the window connection. The window closes when all clones of this struct drops.
    #[derive(Clone)]
    pub struct ViewWindow(Rc<WindowConnection>);
    impl PartialEq for ViewWindow {
        fn eq(&self, other: &Self) -> bool {
            self.0.id == other.0.id && self.0.generation == other.0.generation
        }
    }
    impl Eq for ViewWindow {}
    impl ViewWindow {
        /// Returns `true` if this window connection is still valid.
        ///
        /// The connection can be permanently lost in case the "view-process" respawns, in this
        /// case all methods will return [`Respawned`], and you must discard this connection and
        /// create a new one.
        #[inline]
        pub fn alive(&self) -> bool {
            self.0.alive()
        }

        /// Returns the view-process generation on which the window was open.
        #[inline]
        pub fn generation(&self) -> ViewProcessGen {
            self.0.generation
        }

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
        pub fn set_icon(&self, icon: Option<&ViewImage>) -> Result<()> {
            self.0.call(|id, p| {
                if let Some(icon) = icon {
                    if p.generation() == icon.0.generation {
                        p.set_icon(id, Some(icon.0.id))
                    } else {
                        Err(Respawned)
                    }
                } else {
                    p.set_icon(id, None)
                }
            })
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
            self.0.call(|id, p| p.set_parent(id, parent.map(WindowId::get), modal))
        }

        /// Set the window position.
        #[inline]
        pub fn set_position(&self, pos: DipPoint) -> Result<()> {
            self.0.call(|id, p| p.set_position(id, pos))
        }

        /// Set the window size.
        #[inline]
        pub fn set_size(&self, size: DipSize, frame: FrameRequest) -> Result<()> {
            self.0.call(|id, p| p.set_size(id, size, frame))
        }

        /// Set the window state.
        #[inline]
        pub fn set_state(&self, state: WindowState) -> Result<()> {
            self.0.call(|id, p| p.set_state(id, state))
        }

        /// Set video mode used in exclusive fullscreen.
        #[inline]
        pub fn set_video_mode(&self, mode: VideoMode) -> Result<()> {
            self.0.call(|id, p| p.set_video_mode(id, mode))
        }

        /// Set the window minimum size.
        #[inline]
        pub fn set_min_size(&self, size: DipSize) -> Result<()> {
            self.0.call(|id, p| p.set_min_size(id, size))
        }

        /// Set the window maximum size.
        #[inline]
        pub fn set_max_size(&self, size: DipSize) -> Result<()> {
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

        /// Sets if the headed window is in *capture-mode*. If `true` the resources used to capture
        /// a screenshot are kept in memory to be reused in the next screenshot capture.
        #[inline]
        pub fn set_capture_mode(&self, enabled: bool) -> Result<()> {
            self.0.call(|id, p| p.set_capture_mode(id, enabled))
        }

        /// Drop `self`.
        pub fn close(self) {
            drop(self)
        }
    }

    /// Connection to a headless surface/document open in the View Process.
    ///
    /// This is a strong reference to the window connection. The view is disposed when every reference drops.
    #[derive(Clone)]
    pub struct ViewHeadless(Rc<WindowConnection>, DocumentId);
    impl PartialEq for ViewHeadless {
        fn eq(&self, other: &Self) -> bool {
            self.0.id == other.0.id && self.0.generation == other.0.generation
        }
    }
    impl Eq for ViewHeadless {}
    impl ViewHeadless {
        /// Resize the headless surface.
        #[inline]
        pub fn set_size(&self, size: DipSize, scale_factor: Factor) -> Result<()> {
            let doc_id = self.1;
            self.0.call(|id, p| p.set_headless_size(id, doc_id, size, scale_factor.0))
        }

        /// Reference the window renderer.
        #[inline]
        pub fn renderer(&self) -> ViewRenderer {
            ViewRenderer(Rc::downgrade(&self.0))
        }

        /// Open a virtual headless surface that shares the renderer used by `self`.
        pub fn open_document(&self, size: DipSize, scale_factor: Factor) -> Result<ViewHeadless> {
            let c = self.0.call(|id, p| {
                p.open_document(DocumentRequest {
                    renderer: id,
                    scale_factor: scale_factor.0,
                    size,
                })
            })?;
            Ok(Self(Rc::clone(&self.0), c.document_id))
        }
    }

    /// Connection to a renderer in the View Process.
    ///
    /// This is only a weak reference, every method returns [`Respawned`] if the
    /// renderer has been dropped.
    ///
    /// [`Respawned`]: Respawned
    #[derive(Clone)]
    pub struct ViewRenderer(rc::Weak<WindowConnection>);
    impl PartialEq for ViewRenderer {
        fn eq(&self, other: &Self) -> bool {
            if let (Some(s), Some(o)) = (self.0.upgrade(), other.0.upgrade()) {
                s.id == o.id && s.generation == o.generation
            } else {
                false
            }
        }
    }
    impl ViewRenderer {
        fn call<R>(&self, f: impl FnOnce(ApiWindowId, &mut Controller) -> Result<R>) -> Result<R> {
            if let Some(c) = self.0.upgrade() {
                c.call(f)
            } else {
                Err(Respawned)
            }
        }

        /// Returns the view-process generation on which the renderer was created.
        pub fn generation(&self) -> Result<ViewProcessGen> {
            self.0.upgrade().map(|c| c.generation).ok_or(Respawned)
        }

        /// Returns `true` if the renderer is still alive.
        ///
        /// The renderer is dropped when the window closes or the view-process respawns.
        #[inline]
        pub fn alive(&self) -> bool {
            self.0.upgrade().map(|c| c.alive()).unwrap_or(false)
        }

        /// Pipeline ID.
        ///
        /// This value is cached locally (not an IPC call).
        #[inline]
        pub fn pipeline_id(&self) -> Result<PipelineId> {
            if let Some(c) = self.0.upgrade() {
                if c.alive() {
                    return Ok(c.pipeline_id);
                }
            }
            Err(Respawned)
        }

        /// Resource namespace.
        ///
        /// This value is cached locally (not an IPC call).
        #[inline]
        pub fn namespace_id(&self) -> Result<IdNamespace> {
            if let Some(c) = self.0.upgrade() {
                if c.alive() {
                    return Ok(c.id_namespace);
                }
            }
            Err(Respawned)
        }

        /// Document ID.
        ///
        /// This value is cached locally (not an IPC call).
        #[inline]
        pub fn document_id(&self) -> Result<DocumentId> {
            if let Some(c) = self.0.upgrade() {
                if c.alive() {
                    return Ok(c.document_id);
                }
            }
            Err(Respawned)
        }

        /// Use an image resource in the window renderer.
        ///
        /// Returns the image key.
        pub fn use_image(&self, image: &ViewImage) -> Result<ImageKey> {
            self.call(|id, p| {
                if p.generation() == image.0.generation {
                    p.use_image(id, image.0.id)
                } else {
                    Err(Respawned)
                }
            })
        }

        /// Replace the image resource in the window renderer.
        pub fn update_image_use(&mut self, key: ImageKey, image: &ViewImage) -> Result<()> {
            self.call(|id, p| {
                if p.generation() == image.0.generation {
                    p.update_image_use(id, key, image.0.id)
                } else {
                    Err(Respawned)
                }
            })
        }

        /// Delete the image resource in the window renderer.
        pub fn delete_image_use(&mut self, key: ImageKey) -> Result<()> {
            self.call(|id, p| p.delete_image_use(id, key))
        }

        /// Add a raw font resource to the window renderer.
        ///
        /// Returns the new font key.
        pub fn add_font(&self, bytes: Vec<u8>, index: u32) -> Result<FontKey> {
            self.call(|id, p| p.add_font(id, ByteBuf::from(bytes), index))
        }

        /// Delete the font resource in the window renderer.
        pub fn delete_font(&self, key: FontKey) -> Result<()> {
            self.call(|id, p| p.delete_font(id, key))
        }

        /// Add a font instance to the window renderer.
        ///
        /// Returns the new instance key.
        pub fn add_font_instance(
            &self,
            font_key: FontKey,
            glyph_size: Px,
            options: Option<FontInstanceOptions>,
            plataform_options: Option<FontInstancePlatformOptions>,
            variations: Vec<FontVariation>,
        ) -> Result<FontInstanceKey> {
            self.call(|id, p| p.add_font_instance(id, font_key, glyph_size, options, plataform_options, variations))
        }

        /// Delete the font instance.
        pub fn delete_font_instance(&self, key: FontInstanceKey) -> Result<()> {
            self.call(|id, p| p.delete_font_instance(id, key))
        }

        /// Gets the viewport size (window inner size).
        pub fn size(&self) -> Result<DipSize> {
            self.call(|id, p| p.size(id))
        }

        /// Gets the window scale factor.
        pub fn scale_factor(&self) -> Result<f32> {
            self.call(|id, p| p.scale_factor(id))
        }

        /// Create a new image resource from the current rendered frame.
        pub fn frame_image(&self) -> Result<ViewImage> {
            if let Some(c) = self.0.upgrade() {
                let id = c.call(|id, p| p.frame_image(id))?;
                Ok(Self::add_frame_image(&c.app, id))
            } else {
                Err(Respawned)
            }
        }

        /// Create a new image resource from a selection of the current rendered frame.
        pub fn frame_image_rect(&self, rect: PxRect) -> Result<ViewImage> {
            if let Some(c) = self.0.upgrade() {
                let id = c.call(|id, p| p.frame_image_rect(id, rect))?;
                Ok(Self::add_frame_image(&c.app, id))
            } else {
                Err(Respawned)
            }
        }

        fn add_frame_image(app: &Rc<RefCell<ViewApp>>, id: ImageId) -> ViewImage {
            if id == 0 {
                ViewImage::dummy(None)
            } else {
                let mut app_mut = app.borrow_mut();
                let img = ViewImage(Rc::new(ImageConnection {
                    id,
                    generation: app_mut.process.generation(),
                    app: Some(app.clone()),
                    size: Cell::new(PxSize::zero()),
                    partial_size: Cell::new(PxSize::zero()),
                    ppi: Cell::new(None),
                    opaque: Cell::new(false),
                    partial_bgra8: RefCell::new(None),
                    bgra8: OnceCell::new(),
                    done_signal: SignalOnce::new(),
                }));

                app_mut.loading_images.push(Rc::downgrade(&img.0));
                app_mut.frame_images.push(Rc::downgrade(&img.0));

                img
            }
        }

        /// Get display items of the last rendered frame that intercept the `point`.
        ///
        /// Returns all hits from front-to-back.
        pub fn hit_test(&self, point: PxPoint) -> Result<(FrameId, HitTestResult)> {
            self.call(|id, p| p.hit_test(id, point))
        }

        /// Change the text anti-alias used in this renderer.
        pub fn set_text_aa(&self, aa: TextAntiAliasing) -> Result<()> {
            self.call(|id, p| p.set_text_aa(id, aa))
        }

        /// Render a new frame.
        pub fn render(&self, frame: FrameRequest) -> Result<()> {
            let _s = tracing::debug_span!("ViewRenderer.render").entered();
            self.call(|id, p| p.render(id, frame))
        }

        /// Update the current frame and re-render it.
        pub fn render_update(&self, frame: FrameUpdateRequest) -> Result<()> {
            let _s = tracing::debug_span!("ViewRenderer.render_update").entered();
            self.call(|id, p| p.render_update(id, frame))
        }
    }

    event_args! {
        /// Arguments for the [`ViewProcessRespawnedEvent`].
        pub struct ViewProcessRespawnedArgs {
            /// New view-process generation
            pub generation: ViewProcessGen,

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
        view_process::{MonitorInfo, TextAntiAliasing, ViewImage, WindowState},
        DeviceId,
    };
    use crate::{
        event::*,
        keyboard::{Key, KeyState, ModifiersState, ScanCode},
        mouse::{ButtonState, MouseButton, MultiClickConfig},
        render::FrameId,
        units::{DipPoint, DipSize, Factor, PxRect},
        window::{EventCause, MonitorId, WindowId, WindowTheme},
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
            pub position: DipPoint,

            /// Who moved the window.
            pub cause: EventCause,

            ..

            /// Returns `true` for all widgets in the [window](Self::window_id).
            fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
                ctx.path.window_id() == self.window_id
            }
        }

        /// Arguments for the [`RawWindowStateChangedEvent`].
        pub struct RawWindowStateChangedArgs {
            /// The window.
            pub window_id: WindowId,

            /// New window state.
            pub state: WindowState,

            /// Who changed the state.
            pub cause: EventCause,

            ..

            /// Returns `true` for all widgets in the [window](Self::window_id).
            fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
                ctx.path.window_id() == self.window_id
            }
        }

        /// Arguments for the [`RawFrameRenderedEvent`].
        pub struct RawFrameRenderedArgs {
            /// Window that presents the rendered frame.
            pub window_id: WindowId,

            /// Frame tag.
            pub frame_id: FrameId,

            /// The frame pixels if it was requested when the frame request was sent to the view process.
            pub frame_image: Option<ViewImage>,

            /// Hit-test at the cursor position.
            pub cursor_hits: crate::render::webrender_api::HitTestResult,

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
            pub size: DipSize,

            /// Who resized the window.
            pub cause: EventCause,

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
            pub position: DipPoint,

            /// Raw hit-test.
            pub hit_test: crate::render::webrender_api::HitTestResult,

            /// Frame that was hit-test.
            pub frame_id: FrameId,

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
            pub scale_factor: Factor,

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

        /// Arguments for the image events.
        pub struct RawImageArgs {
            /// Image that changed.
            pub image: ViewImage,

            ..

            /// Concerns all widgets.
            fn concerns_widget(&self, _ctx: &mut WidgetContext) -> bool {
                true
            }
        }

        /// Arguments for the [`RawFrameImageReadyEvent`].
        pub struct RawFrameImageReadyArgs {
            /// Frame image that is ready.
            pub image: ViewImage,

            /// Window that was captured.
            pub window_id: WindowId,

            /// Frame that was captured.
            pub frame_id: FrameId,

            /// Area of the frame that was captured.
            pub area: PxRect,

            ..

            /// Concerns all widgets.
            fn concerns_widget(&self, _ctx: &mut WidgetContext) -> bool {
                true
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

        /// A window was maximized/minimized/restored.
        pub RawWindowStateChangedEvent: RawWindowStateChangedArgs;

        /// A frame finished rendering and was presented in a window.
        pub RawFrameRenderedEvent: RawFrameRenderedArgs;

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

        /// Image metadata loaded without errors.
        pub RawImageMetadataLoadedEvent: RawImageArgs;

        /// Progressively decoded image has decoded more pixels.
        pub RawImagePartiallyLoadedEvent: RawImageArgs;
        /// Image loaded without errors.
        pub RawImageLoadedEvent: RawImageArgs;

        /// Image failed to load.
        pub RawImageLoadErrorEvent: RawImageArgs;

        /// Image generated from a frame is ready for reading.
        pub RawFrameImageReadyEvent: RawFrameImageReadyArgs;
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

    pub use zero_ui_view_api::{AxisId, ButtonId, MouseScrollDelta};

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
