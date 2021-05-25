//! App startup and app extension API.

use crate::context::*;
use crate::event::{cancelable_event_args, EventEmitter, EventListener};
use crate::profiler::*;
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
use glutin::event::StartCause as GEventStartCause;
use glutin::event_loop::{
    ControlFlow, EventLoop as GEventLoop, EventLoopProxy as GEventLoopProxy, EventLoopWindowTarget as GEventLoopWindowTarget,
};
use std::{
    any::{type_name, TypeId},
    sync::atomic::AtomicBool,
};
use std::{fmt, mem};

pub use glutin::event::{DeviceEvent, DeviceId, ElementState};

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

    /// Called when the OS sends a global device event.
    #[inline]
    fn on_device_event(&mut self, device_id: DeviceId, event: &DeviceEvent, ctx: &mut AppContext) {
        let _ = (device_id, event, ctx);
    }

    /// Called when the OS sends an event to a window.
    #[inline]
    fn on_window_event(&mut self, window_id: WindowId, event: &WindowEvent, ctx: &mut AppContext) {
        let _ = (window_id, event, ctx);
    }

    /// Called just before [`update_ui`](Self::update_ui).
    ///
    /// Extensions can handle this method to interact with updates before the UI.
    ///
    /// Note that this is not related to the `on_event_preview` properties, all UI events
    /// happen in `update_ui`.
    #[inline]
    fn update_preview(&mut self, update: UpdateRequest, ctx: &mut AppContext) {
        let _ = (update, ctx);
    }

    /// Called just before [`update`](Self::update).
    ///
    /// Only extensions that generate windows must handle this method. The [`UiNode::update`](super::UiNode::update)
    /// and [`UiNode::update_hp`](super::UiNode::update_hp) are called here.
    #[inline]
    fn update_ui(&mut self, update: UpdateRequest, ctx: &mut AppContext) {
        let _ = (update, ctx);
    }

    /// Called after every [`update_ui`](Self::update_ui).
    ///
    /// This is the general extensions update, it gives the chance for
    /// the UI to signal stop propagation.
    #[inline]
    fn update(&mut self, update: UpdateRequest, ctx: &mut AppContext) {
        let _ = (update, ctx);
    }

    /// Called after every sequence of updates if display update was requested.
    #[inline]
    fn update_display(&mut self, update: UpdateDisplayRequest, ctx: &mut AppContext) {
        let _ = (update, ctx);
    }

    /// Called when a new frame is ready to be presented.
    #[inline]
    fn on_new_frame_ready(&mut self, window_id: WindowId, ctx: &mut AppContext) {
        let _ = (window_id, ctx);
    }

    /// Called when the OS sends a request for re-drawing the last frame.
    #[inline]
    fn on_redraw_requested(&mut self, window_id: WindowId, ctx: &mut AppContext) {
        let _ = (window_id, ctx);
    }

    /// Called when a shutdown was requested.
    #[inline]
    fn on_shutdown_requested(&mut self, args: &ShutdownRequestedArgs, ctx: &mut AppContext) {
        let _ = (args, ctx);
    }

    /// Called when the application is shutting down.
    ///
    /// Update requests generated during this call are ignored.
    #[inline]
    fn deinit(&mut self, ctx: &mut AppContext) {
        let _ = ctx;
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

/// In release mode we use generics tricks to compile all app extensions with
/// static dispatch optimized to a direct call to the extension handle.
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

/// In debug mode we use dynamic dispatch to reduce the number of types
/// in the stack-trace and compile more quickly.
#[cfg(debug_assertions)]
impl App {
    /// Application without any extension.
    pub fn blank() -> AppExtended<Vec<Box<dyn AppExtension>>> {
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
    pub fn default() -> AppExtended<Vec<Box<dyn AppExtension>>> {
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
    shutdown_requests: Vec<EventEmitter<ShutdownCancelled>>,
    update_notifier: UpdateNotifier,
}
impl AppProcess {
    /// New app process service
    pub fn new(update_notifier: UpdateNotifier) -> Self {
        AppProcess {
            shutdown_requests: Vec::new(),
            update_notifier,
        }
    }

    /// Register a request for process shutdown in the next update.
    ///
    /// Returns an event listener that is updated once with the unit value [`ShutdownCancelled`]
    /// if the shutdown operation is cancelled.
    pub fn shutdown(&mut self) -> EventListener<ShutdownCancelled> {
        let emitter = EventEmitter::response();
        self.shutdown_requests.push(emitter.clone());
        self.update_notifier.update();
        emitter.into_listener()
    }

    fn take_requests(&mut self) -> Vec<EventEmitter<ShutdownCancelled>> {
        mem::take(&mut self.shutdown_requests)
    }
}
///Returns if should shutdown
fn shutdown(shutdown_requests: Vec<EventEmitter<ShutdownCancelled>>, ctx: &mut AppContext, ext: &mut impl AppExtension) -> bool {
    if shutdown_requests.is_empty() {
        return false;
    }
    let args = ShutdownRequestedArgs::now();
    ext.on_shutdown_requested(&args, ctx);
    if args.cancel_requested() {
        for c in shutdown_requests {
            c.notify(ctx.events, ShutdownCancelled);
        }
    }
    !args.cancel_requested()
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
            EventLoopInner::Glutin(el) => EventLoopProxy(EventLoopProxyInner::Glutin(el.create_proxy())),
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
    Glutin(GEventLoopProxy<AppEvent>),
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
            EventLoopProxyInner::Glutin(_) => false,
        }
    }

    /// Send an [`AppEvent`] to the app.
    pub fn send_event(&self, event: AppEvent) {
        match &self.0 {
            EventLoopProxyInner::Glutin(elp) => elp.send_event(event).unwrap(),
            EventLoopProxyInner::Headless(sender) => sender.send(event).unwrap(),
        }
    }
}

#[cfg(debug_assertions)]
impl AppExtended<Vec<Box<dyn AppExtension>>> {
    /// Includes an application extension.
    ///
    /// # Panics
    /// * `"app already extended with `{}`"` when the app is already [`extended_with`](AppExtended::extended_with) the
    /// extension type.
    #[inline]
    pub fn extend<F: AppExtension>(self, extension: F) -> AppExtended<Vec<Box<dyn AppExtension>>> {
        if self.extended_with::<F>() {
            panic!("app already extended with `{}`", type_name::<F>())
        }

        let mut extensions = self.extensions;
        extensions.push(Box::new(extension));

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

    /// Runs the application event loop calling `start` once at the beginning.
    ///
    /// # Panics
    ///
    /// Panics if not called by the main thread. This means you cannot run an app in unit tests, use a headless
    /// app without renderer for that. The main thread is required by some operating systems and OpenGL.
    #[inline]
    pub fn run(self, start: impl FnOnce(&mut AppContext)) -> ! {
        if !is_main_thread::is_main_thread().unwrap_or(true) {
            panic!("can only init headed app in the main thread")
        }
        if HEADED_APP_RUNNING.swap(true, std::sync::atomic::Ordering::AcqRel) {
            panic!("only one headed app is allowed per process")
        }
        if App::is_running() {
            panic!("only one app is allowed per thread")
        }

        #[cfg(feature = "app_profiler")]
        register_thread_with_profiler();

        profile_scope!("app::run");

        let event_loop = EventLoop::new(false);

        let mut extensions = self.extensions;

        let mut owned_ctx = OwnedAppContext::instance(event_loop.create_proxy());

        let mut init_ctx = owned_ctx.borrow_init();
        init_ctx.services.register(AppProcess::new(init_ctx.updates.notifier().clone()));
        extensions.init(&mut init_ctx);

        let mut in_sequence = false;
        let mut sequence_update = UpdateDisplayRequest::None;

        start(&mut owned_ctx.borrow(event_loop.window_target()));

        event_loop.run_headed(move |event, event_loop, control_flow| {
            profile_scope!("app::event");

            if let ControlFlow::Poll = &control_flow {
                // Poll is the initial value but we want to use
                // Wait by default. This should only happen once
                // because we don't use Poll at all.
                *control_flow = ControlFlow::Wait;
            }

            let mut event_update = UpdateRequest::default();
            match event {
                GEvent::NewEvents(cause) => {
                    if let GEventStartCause::ResumeTimeReached { .. } = cause {
                        // we assume only timers set WaitUntil.
                        let ctx = owned_ctx.borrow(event_loop);
                        if let Some(resume) = ctx.sync.update_timers(ctx.events) {
                            *control_flow = ControlFlow::WaitUntil(resume);
                        } else {
                            *control_flow = ControlFlow::Wait;
                        }
                    }
                    in_sequence = true;
                }

                GEvent::WindowEvent { window_id, event } => {
                    profile_scope!("app::on_window_event");
                    extensions.on_window_event(window_id.into(), &event, &mut owned_ctx.borrow(event_loop));
                }
                GEvent::UserEvent(AppEvent::NewFrameReady(window_id)) => {
                    profile_scope!("app::on_new_frame_ready");
                    extensions.on_new_frame_ready(window_id, &mut owned_ctx.borrow(event_loop));
                }
                GEvent::UserEvent(AppEvent::Update) => {
                    event_update = owned_ctx.take_request();
                }
                GEvent::DeviceEvent { device_id, event } => {
                    profile_scope!("app::on_device_event");
                    extensions.on_device_event(device_id, &event, &mut owned_ctx.borrow(event_loop));
                }

                GEvent::MainEventsCleared => {
                    in_sequence = false;
                }

                GEvent::RedrawRequested(window_id) => {
                    profile_scope!("app::on_redraw_requested");
                    extensions.on_redraw_requested(window_id.into(), &mut owned_ctx.borrow(event_loop))
                }

                #[cfg(feature = "app_profiler")]
                GEvent::LoopDestroyed => {
                    crate::profiler::write_profile("app_profile.json", true);
                }

                _ => {}
            }

            // changes to this loop must be copied to the `HeadlessApp::update` loop.
            let mut limit = UPDATE_LIMIT;
            loop {
                let ((mut update, display), wake) = owned_ctx.apply_updates();

                update |= mem::take(&mut event_update);
                sequence_update |= display;
                if let Some(until) = wake {
                    *control_flow = ControlFlow::WaitUntil(until);
                }

                if update.update || update.update_hp {
                    {
                        profile_scope!("app::update");
                        let mut ctx = owned_ctx.borrow(event_loop);
                        let shutdown_requests = ctx.services.req::<AppProcess>().take_requests();
                        if shutdown(shutdown_requests, &mut ctx, &mut extensions) {
                            *control_flow = ControlFlow::Exit;
                            return;
                        }
                        extensions.update_preview(update, &mut ctx);
                        ctx.events.on_pre_events(&mut ctx);
                        extensions.update_ui(update, &mut ctx);
                        extensions.update(update, &mut ctx);
                        ctx.events.on_events(&mut ctx);
                    }
                } else {
                    break;
                }

                limit -= 1;
                if limit == 0 {
                    panic!("immediate update loop reached limit of `{}` repeats", UPDATE_LIMIT)
                }
            }

            if !in_sequence && sequence_update.is_some() {
                profile_scope!("app::update_display");
                extensions.update_display(sequence_update, &mut owned_ctx.borrow(event_loop));
                sequence_update = UpdateDisplayRequest::None;
            }
        })
    }

    /// Initializes extensions in headless mode and returns an [`HeadlessApp`].
    ///
    /// # Tests
    ///
    /// If called in a test (`cfg(test)`) this blocks until no other instance of [`HeadlessApp`] and
    /// [`TestWidgetContext`] are running in the current thread.
    #[inline]
    pub fn run_headless(self) -> HeadlessApp {
        if App::is_running() {
            if cfg!(any(test, doc, feature = "pub_test")) {
                panic!("only one app or `TestWidgetContext` is allowed per thread")
            } else {
                panic!("only one app is allowed per thread")
            }
        }

        #[cfg(feature = "app_profiler")]
        let profile_scope = {
            register_thread_with_profiler();
            ProfileScope::new("app::run_headless")
        };

        let event_loop = EventLoop::new(true);

        let mut owned_ctx = OwnedAppContext::instance(event_loop.create_proxy());

        let mut extensions = self.extensions;

        let mut init_ctx = owned_ctx.borrow_init();
        init_ctx.services.register(AppProcess::new(init_ctx.updates.notifier().clone()));
        extensions.init(&mut init_ctx);

        HeadlessApp {
            event_loop,
            extensions: Box::new(extensions),
            owned_ctx,
            control_flow: ControlFlow::Wait,

            #[cfg(feature = "app_profiler")]
            _pf: profile_scope,
        }
    }
}

const UPDATE_LIMIT: u32 = 100_000;

/// Raw events generated by the app.
#[derive(Debug)]
pub enum AppEvent {
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
    extensions: Box<dyn AppExtension>,
    owned_ctx: OwnedAppContext,
    control_flow: ControlFlow,
    #[cfg(feature = "app_profiler")]
    _pf: ProfileScope,
}
impl HeadlessApp {
    /// Headless state.
    ///
    /// Can be accessed in a context using [`HeadlessInfo`].
    pub fn headless_state(&self) -> &StateMap {
        self.owned_ctx.headless_state().unwrap()
    }

    /// Mutable headless state.
    pub fn headless_state_mut(&mut self) -> &mut StateMap {
        self.owned_ctx.headless_state_mut().unwrap()
    }

    /// App state.
    pub fn app_state(&self) -> &StateMap {
        self.owned_ctx.app_state()
    }

    /// Mutable app state.
    pub fn app_state_mut(&mut self) -> &mut StateMap {
        self.owned_ctx.app_state_mut()
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
    /// When enabled windows are still not visible but you can request [screenshots](crate::window::OpenWindow::screenshot)
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
    pub fn on_device_event(&mut self, device_id: DeviceId, event: &DeviceEvent) {
        profile_scope!("headless_app::on_device_event");
        self.extensions
            .on_device_event(device_id, event, &mut self.owned_ctx.borrow(self.event_loop.window_target()));
    }

    /// Notifies extensions of a [window event](WindowEvent).
    pub fn on_window_event(&mut self, window_id: WindowId, event: &WindowEvent) {
        profile_scope!("headless_app::on_device_event");
        self.extensions
            .on_window_event(window_id, event, &mut self.owned_ctx.borrow(self.event_loop.window_target()));
    }

    /// Pushes an [app event](AppEvent).
    pub fn on_app_event(&mut self, event: AppEvent) {
        profile_scope!("headless_app::on_app_event");
        self.event_loop.create_proxy().send_event(event);
    }

    /// Runs a custom action in the headless app context.
    pub fn with_context<R>(&mut self, action: impl FnOnce(&mut AppContext) -> R) -> R {
        profile_scope!("headless_app::with_context");
        action(&mut self.owned_ctx.borrow(self.event_loop.window_target()))
    }

    /// Does updates until no more updates are requested.
    ///
    /// If `wait_app_event` is `true` the thread sleeps until at least one app event is received,
    /// if it is `false` only responds to app events already in the buffer.
    #[inline]
    pub fn update(&mut self, wait_app_event: bool) -> ControlFlow {
        self.update_observe_all(|_, _| {}, |_, _| {}, |_, _| {}, |_, _| {}, wait_app_event)
    }

    /// Does updates with a callback called after the extensions update listeners.
    ///
    /// If `wait_app_event` is `true` the thread sleeps until at least one app event is received,
    /// if it is `false` only responds to app events already in the buffer.
    pub fn update_observe(&mut self, on_update: impl FnMut(UpdateRequest, &mut AppContext), wait_app_event: bool) -> ControlFlow {
        self.update_observe_all(|_, _| {}, |_, _| {}, |_, _| {}, on_update, wait_app_event)
    }

    /// Does updates with a callback called after the extensions respond to a new frame ready.
    ///
    /// If `wait_app_event` is `true` the thread sleeps until at least one app event is received,
    /// if it is `false` only responds to app events already in the buffer.
    pub fn update_observe_frame(&mut self, on_new_frame_ready: impl FnMut(WindowId, &mut AppContext), wait_app_event: bool) -> ControlFlow {
        self.update_observe_all(on_new_frame_ready, |_, _| {}, |_, _| {}, |_, _| {}, wait_app_event)
    }

    /// Does updates injecting update listeners after the extension listeners.
    ///
    /// If `wait_app_event` is `true` the thread sleeps until at least one app event is received,
    /// if it is `false` only responds to app events already in the buffer.
    pub fn update_observe_all(
        &mut self,
        mut on_new_frame_ready: impl FnMut(WindowId, &mut AppContext),
        mut on_update_preview: impl FnMut(UpdateRequest, &mut AppContext),
        mut on_update_ui: impl FnMut(UpdateRequest, &mut AppContext),
        mut on_update: impl FnMut(UpdateRequest, &mut AppContext),
        wait_app_event: bool,
    ) -> ControlFlow {
        if let ControlFlow::Exit = self.control_flow {
            return ControlFlow::Exit;
        }

        let mut event_update = UpdateRequest::default();

        for event in self.event_loop.take_headless_app_events(wait_app_event) {
            match event {
                AppEvent::NewFrameReady(window_id) => {
                    let mut ctx = &mut self.owned_ctx.borrow(self.event_loop.window_target());
                    self.extensions.on_new_frame_ready(window_id, &mut ctx);
                    on_new_frame_ready(window_id, &mut ctx)
                }
                AppEvent::Update => {
                    event_update |= self.owned_ctx.take_request();
                }
            }
        }

        let mut sequence_update = UpdateDisplayRequest::None;

        // this loop implementation is kept in sync with the one in `AppExtended::run`.
        let mut limit = UPDATE_LIMIT;
        loop {
            let ((mut update, display), wake) = self.owned_ctx.apply_updates();

            update |= mem::take(&mut event_update);
            sequence_update |= display;
            if let Some(until) = wake {
                self.control_flow = ControlFlow::WaitUntil(until);
            }

            if update.update || update.update_hp {
                {
                    profile_scope!("headless_app::update");
                    let mut ctx = self.owned_ctx.borrow(self.event_loop.window_target());
                    let shutdown_requests = ctx.services.req::<AppProcess>().take_requests();
                    if shutdown(shutdown_requests, &mut ctx, &mut self.extensions) {
                        self.control_flow = ControlFlow::Exit;
                        return ControlFlow::Exit;
                    }
                    self.extensions.update_preview(update, &mut ctx);
                    on_update_preview(update, &mut ctx);
                    ctx.events.on_pre_events(&mut ctx);

                    self.extensions.update_ui(update, &mut ctx);
                    on_update_ui(update, &mut ctx);

                    self.extensions.update(update, &mut ctx);
                    on_update(update, &mut ctx);
                    ctx.events.on_events(&mut ctx);
                }
            } else {
                break;
            }

            limit -= 1;
            if limit == 0 {
                panic!("immediate update loop reached limit of `{}` repeats", UPDATE_LIMIT)
            }
        }

        if sequence_update.is_some() {
            profile_scope!("headless_app::update_display");
            self.extensions
                .update_display(sequence_update, &mut self.owned_ctx.borrow(self.event_loop.window_target()));
        }

        self.control_flow
    }

    /// [`ControlFlow`] after the last update.
    #[inline]
    pub fn control_flow(&self) -> ControlFlow {
        self.control_flow // TODO better merge
    }
}

state_key! {
    /// If render is enabled in [headless mode](AppExtended::run_headless).
    ///
    /// See [`HeadlessApp::enable_renderer`] for for details.
    pub struct HeadlessRendererEnabledKey: bool;
}

#[cfg(not(debug_assertions))]
impl AppExtension for () {
    #[inline]
    fn is_or_contain(&self, _: TypeId) -> bool {
        false
    }
}

#[cfg(not(debug_assertions))]
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
    fn on_device_event(&mut self, device_id: DeviceId, event: &DeviceEvent, ctx: &mut AppContext) {
        self.0.on_device_event(device_id, event, ctx);
        self.1.on_device_event(device_id, event, ctx);
    }

    #[inline]
    fn on_window_event(&mut self, window_id: WindowId, event: &WindowEvent, ctx: &mut AppContext) {
        self.0.on_window_event(window_id, event, ctx);
        self.1.on_window_event(window_id, event, ctx);
    }

    #[inline]
    fn on_new_frame_ready(&mut self, window_id: WindowId, ctx: &mut AppContext) {
        self.0.on_new_frame_ready(window_id, ctx);
        self.1.on_new_frame_ready(window_id, ctx);
    }

    #[inline]
    fn update_preview(&mut self, update: UpdateRequest, ctx: &mut AppContext) {
        self.0.update_preview(update, ctx);
        self.1.update_preview(update, ctx);
    }

    #[inline]
    fn update_ui(&mut self, update: UpdateRequest, ctx: &mut AppContext) {
        self.0.update_ui(update, ctx);
        self.1.update_ui(update, ctx);
    }

    #[inline]
    fn update(&mut self, update: UpdateRequest, ctx: &mut AppContext) {
        self.0.update(update, ctx);
        self.1.update(update, ctx);
    }

    #[inline]
    fn update_display(&mut self, update: UpdateDisplayRequest, ctx: &mut AppContext) {
        self.0.update_display(update, ctx);
        self.1.update_display(update, ctx);
    }

    #[inline]
    fn on_redraw_requested(&mut self, window_id: WindowId, ctx: &mut AppContext) {
        self.0.on_redraw_requested(window_id, ctx);
        self.1.on_redraw_requested(window_id, ctx);
    }

    #[inline]
    fn on_shutdown_requested(&mut self, args: &ShutdownRequestedArgs, ctx: &mut AppContext) {
        self.0.on_shutdown_requested(args, ctx);
        self.1.on_shutdown_requested(args, ctx);
    }

    #[inline]
    fn deinit(&mut self, ctx: &mut AppContext) {
        self.0.deinit(ctx);
        self.1.deinit(ctx);
    }
}

#[cfg(debug_assertions)]
impl AppExtension for Vec<Box<dyn AppExtension>> {
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

    fn on_device_event(&mut self, device_id: DeviceId, event: &DeviceEvent, ctx: &mut AppContext) {
        for ext in self {
            ext.on_device_event(device_id, event, ctx);
        }
    }

    fn on_window_event(&mut self, window_id: WindowId, event: &WindowEvent, ctx: &mut AppContext) {
        for ext in self {
            ext.on_window_event(window_id, event, ctx);
        }
    }

    fn on_new_frame_ready(&mut self, window_id: WindowId, ctx: &mut AppContext) {
        for ext in self {
            ext.on_new_frame_ready(window_id, ctx);
        }
    }

    fn update_preview(&mut self, update: UpdateRequest, ctx: &mut AppContext) {
        for ext in self {
            ext.update_preview(update, ctx);
        }
    }

    fn update_ui(&mut self, update: UpdateRequest, ctx: &mut AppContext) {
        for ext in self {
            ext.update_ui(update, ctx);
        }
    }

    fn update(&mut self, update: UpdateRequest, ctx: &mut AppContext) {
        for ext in self {
            ext.update(update, ctx);
        }
    }

    fn update_display(&mut self, update: UpdateDisplayRequest, ctx: &mut AppContext) {
        for ext in self {
            ext.update_display(update, ctx);
        }
    }

    fn on_redraw_requested(&mut self, window_id: WindowId, ctx: &mut AppContext) {
        for ext in self {
            ext.on_redraw_requested(window_id, ctx);
        }
    }

    fn on_shutdown_requested(&mut self, args: &ShutdownRequestedArgs, ctx: &mut AppContext) {
        for ext in self {
            ext.on_shutdown_requested(args, ctx);
        }
    }

    fn deinit(&mut self, ctx: &mut AppContext) {
        for ext in self {
            ext.deinit(ctx);
        }
    }
}

impl AppExtension for Box<dyn AppExtension> {
    fn id(&self) -> TypeId {
        self.as_ref().id()
    }

    fn is_or_contain(&self, app_extension_id: TypeId) -> bool {
        self.as_ref().is_or_contain(app_extension_id)
    }

    fn init(&mut self, ctx: &mut AppInitContext) {
        self.as_mut().init(ctx);
    }

    fn on_device_event(&mut self, device_id: DeviceId, event: &DeviceEvent, ctx: &mut AppContext) {
        self.as_mut().on_device_event(device_id, event, ctx);
    }

    fn on_window_event(&mut self, window_id: WindowId, event: &WindowEvent, ctx: &mut AppContext) {
        self.as_mut().on_window_event(window_id, event, ctx);
    }

    fn update_preview(&mut self, update: UpdateRequest, ctx: &mut AppContext) {
        self.as_mut().update_preview(update, ctx);
    }

    fn update_ui(&mut self, update: UpdateRequest, ctx: &mut AppContext) {
        self.as_mut().update_ui(update, ctx);
    }

    fn update(&mut self, update: UpdateRequest, ctx: &mut AppContext) {
        self.as_mut().update(update, ctx);
    }

    fn update_display(&mut self, update: UpdateDisplayRequest, ctx: &mut AppContext) {
        self.as_mut().update_display(update, ctx);
    }

    fn on_new_frame_ready(&mut self, window_id: WindowId, ctx: &mut AppContext) {
        self.as_mut().on_new_frame_ready(window_id, ctx);
    }

    fn on_redraw_requested(&mut self, window_id: WindowId, ctx: &mut AppContext) {
        self.as_mut().on_redraw_requested(window_id, ctx);
    }

    fn on_shutdown_requested(&mut self, args: &ShutdownRequestedArgs, ctx: &mut AppContext) {
        self.as_mut().on_shutdown_requested(args, ctx);
    }

    fn deinit(&mut self, ctx: &mut AppContext) {
        self.as_mut().deinit(ctx);
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

        app.with_context(|ctx| {
            let render_enabled = ctx
                .headless
                .state()
                .and_then(|s| s.get::<HeadlessRendererEnabledKey>().copied())
                .unwrap_or_default();

            assert!(!render_enabled);
        });

        app.update(false);
    }

    #[test]
    pub fn new_window_with_render() {
        let mut app = App::default().run_headless();
        app.enable_renderer(true);
        assert!(app.renderer_enabled());

        app.with_context(|ctx| {
            let render_enabled = ctx
                .headless
                .state()
                .and_then(|s| s.get::<HeadlessRendererEnabledKey>().copied())
                .unwrap_or_default();

            assert!(render_enabled);
        });

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
