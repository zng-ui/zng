//! App startup and app extension API.

pub mod raw_device_events;
pub mod raw_events;
pub mod view_process;

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
            let mut events = Vec::with_capacity(100);
            let mut layout = false;
            let mut render = false;

            let mut limit = 100_000;
            loop {
                limit -= 1;
                if limit == 0 {
                    panic!("update loop polled 100,000 times, probably stuck in an infinite loop");
                }

                let skip_timers =  update || !events.is_empty() || layout || render;
                let u = self.owned_ctx.apply_updates(skip_timers);
                let mut ctx = self.owned_ctx.borrow();

                self.wake_time = u.wake_time;
                update |= u.update;
                events.extend(u.events);
                layout |= u.layout;
                render |= u.render;

                if update {
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
                } else if !events.is_empty() {
                    // does events raised by extensions.

                    let _s = tracing::trace_span!("events").entered();

                    for event in events.drain(..) {
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
