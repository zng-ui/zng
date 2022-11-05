//! App startup and app extension API.

mod intrinsic;
pub mod raw_device_events;
pub mod raw_events;
pub mod view_process;

pub use intrinsic::*;

use crate::config::ConfigManager;
use crate::crate_util::{PanicPayload, ReceiverExt};
use crate::event::{event, event_args, EventUpdate};
use crate::image::ImageManager;
use crate::service::Services;
use crate::timer::Timers;
use crate::units::Deadline;
use crate::var::Vars;
use crate::window::WindowMode;
use crate::{context::*, widget_instance::WidgetId};
use crate::{
    focus::FocusManager,
    gesture::GestureManager,
    keyboard::KeyboardManager,
    mouse::MouseManager,
    text::FontManager,
    window::{WindowId, WindowManager},
};

use self::view_process::{ViewProcess, ViewProcessInitedArgs, VIEW_PROCESS_INITED_EVENT};
use once_cell::sync::Lazy;
use pretty_type_name::*;
use std::future::Future;
use std::marker::PhantomData;
use std::mem;
use std::path::PathBuf;
use std::sync::Arc;
use std::task::Waker;
use std::time::Duration;
use std::{
    any::{type_name, TypeId},
    fmt,
    time::Instant,
};

/// Error when the app connected to a sender/receiver channel has disconnected.
///
/// Contains the value that could not be send or `()` for receiver errors.
pub struct AppDisconnected<T>(pub T);
impl From<flume::RecvError> for AppDisconnected<()> {
    fn from(_: flume::RecvError) -> Self {
        AppDisconnected(())
    }
}
impl<T> From<flume::SendError<T>> for AppDisconnected<T> {
    fn from(e: flume::SendError<T>) -> Self {
        AppDisconnected(e.0)
    }
}
impl<T> fmt::Debug for AppDisconnected<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AppDisconnected<{}>", pretty_type_name::<T>())
    }
}
impl<T> fmt::Display for AppDisconnected<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "cannot send/receive because the app has disconnected")
    }
}
impl<T> std::error::Error for AppDisconnected<T> {}

/// Error when the app connected to a sender channel has disconnected or taken to long to respond.
pub enum TimeoutOrAppDisconnected {
    /// Connected app has not responded.
    Timeout,
    /// Connected app has disconnected.
    AppDisconnected,
}
impl From<flume::RecvTimeoutError> for TimeoutOrAppDisconnected {
    fn from(e: flume::RecvTimeoutError) -> Self {
        match e {
            flume::RecvTimeoutError::Timeout => TimeoutOrAppDisconnected::Timeout,
            flume::RecvTimeoutError::Disconnected => TimeoutOrAppDisconnected::AppDisconnected,
        }
    }
}
impl fmt::Debug for TimeoutOrAppDisconnected {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "TimeoutOrAppDisconnected::")?;
        }
        match self {
            TimeoutOrAppDisconnected::Timeout => write!(f, "Timeout"),
            TimeoutOrAppDisconnected::AppDisconnected => write!(f, "AppDisconnected"),
        }
    }
}
impl fmt::Display for TimeoutOrAppDisconnected {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TimeoutOrAppDisconnected::Timeout => write!(f, "failed send, timeout"),
            TimeoutOrAppDisconnected::AppDisconnected => write!(f, "cannot send because the app has disconnected"),
        }
    }
}
impl std::error::Error for TimeoutOrAppDisconnected {}

/// A future that receives a single message from a running [app](App).
pub struct RecvFut<'a, M>(flume::r#async::RecvFut<'a, M>);
impl<'a, M> From<flume::r#async::RecvFut<'a, M>> for RecvFut<'a, M> {
    fn from(f: flume::r#async::RecvFut<'a, M>) -> Self {
        Self(f)
    }
}
impl<'a, M> Future for RecvFut<'a, M> {
    type Output = Result<M, AppDisconnected<()>>;

    fn poll(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        match std::pin::Pin::new(&mut self.0).poll(cx) {
            std::task::Poll::Ready(r) => std::task::Poll::Ready(r.map_err(|_| AppDisconnected(()))),
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
/// A [`RAW_FRAME_RENDERED_EVENT`] is send when a frame finished rendering in the view-process.
///
/// ## 6 - Deinit
///
/// The [`deinit`] method is called once after an exit was requested and not cancelled. Exit is
/// requested using the [`AppProcess`] service, it causes an [`EXIT_REQUESTED_EVENT`] that can be cancelled, if it
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
/// [`RAW_FRAME_RENDERED_EVENT`]: raw_events::RAW_FRAME_RENDERED_EVENT
#[cfg_attr(doc_nightly, doc(notable_trait))]
pub trait AppExtension: 'static {
    /// Type id of this extension.
    fn id(&self) -> TypeId {
        TypeId::of::<Self>()
    }

    /// If this extension is the `app_extension_id` or dispatches to it.
    fn is_or_contain(&self, app_extension_id: TypeId) -> bool {
        self.id() == app_extension_id
    }

    /// Initializes this extension.
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
    fn enable_device_events(&self) -> bool {
        false
    }

    /// Called just before [`event_ui`](Self::event_ui).
    ///
    /// Extensions can handle this method to to intersect event updates before the UI.
    ///
    /// Note that this is not related to the `on_event_preview` properties, all UI events
    /// happen in `on_event_ui`.
    fn event_preview(&mut self, ctx: &mut AppContext, update: &mut EventUpdate) {
        let _ = (ctx, update);
    }

    /// Called just before [`event`](Self::event).
    ///
    /// Only extensions that generate windows must handle this method. The [`UiNode::event`](crate::widget_instance::UiNode::event)
    /// method is called here.
    fn event_ui(&mut self, ctx: &mut AppContext, update: &mut EventUpdate) {
        let _ = (ctx, update);
    }

    /// Called after every [`event_ui`](Self::event_ui).
    ///
    /// This is the general extensions event handler, it gives the chance for the UI to signal stop propagation.
    fn event(&mut self, ctx: &mut AppContext, update: &mut EventUpdate) {
        let _ = (ctx, update);
    }

    /// Called just before [`update_ui`](Self::update_ui).
    ///
    /// Extensions can handle this method to interact with updates before the UI.
    ///
    /// Note that this is not related to the `on_event_preview` properties, all UI events
    /// happen in `update_ui`.
    fn update_preview(&mut self, ctx: &mut AppContext) {
        let _ = ctx;
    }

    /// Called just before [`update`](Self::update).
    ///
    /// Only extensions that generate windows must handle this method. The [`UiNode::update`](crate::widget_instance::UiNode::update)
    /// method is called here.
    fn update_ui(&mut self, ctx: &mut AppContext, updates: &mut WidgetUpdates) {
        let _ = (ctx, updates);
    }

    /// Called after every [`update_ui`](Self::update_ui).
    ///
    /// This is the general extensions update, it gives the chance for
    /// the UI to signal stop propagation.
    fn update(&mut self, ctx: &mut AppContext) {
        let _ = ctx;
    }

    /// Called after every sequence of updates if layout was requested.
    fn layout(&mut self, ctx: &mut AppContext) {
        let _ = ctx;
    }

    /// Called after every sequence of updates and layout if render was requested.
    fn render(&mut self, ctx: &mut AppContext) {
        let _ = ctx;
    }

    /// Called when the application is exiting.
    ///
    /// Update requests and event notifications generated during this call are ignored,
    /// the extensions will be dropped after every extension received this call.
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
    fn update_ui_boxed(&mut self, ctx: &mut AppContext, updates: &mut WidgetUpdates);
    fn update_boxed(&mut self, ctx: &mut AppContext);
    fn event_preview_boxed(&mut self, ctx: &mut AppContext, update: &mut EventUpdate);
    fn event_ui_boxed(&mut self, ctx: &mut AppContext, update: &mut EventUpdate);
    fn event_boxed(&mut self, ctx: &mut AppContext, update: &mut EventUpdate);
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

    fn update_ui_boxed(&mut self, ctx: &mut AppContext, updates: &mut WidgetUpdates) {
        self.update_ui(ctx, updates);
    }

    fn update_boxed(&mut self, ctx: &mut AppContext) {
        self.update(ctx);
    }

    fn event_preview_boxed(&mut self, ctx: &mut AppContext, update: &mut EventUpdate) {
        self.event_preview(ctx, update);
    }

    fn event_ui_boxed(&mut self, ctx: &mut AppContext, update: &mut EventUpdate) {
        self.event_ui(ctx, update);
    }

    fn event_boxed(&mut self, ctx: &mut AppContext, update: &mut EventUpdate) {
        self.event(ctx, update);
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

    fn update_ui(&mut self, ctx: &mut AppContext, updates: &mut WidgetUpdates) {
        self.as_mut().update_ui_boxed(ctx, updates);
    }

    fn update(&mut self, ctx: &mut AppContext) {
        self.as_mut().update_boxed(ctx);
    }

    fn event_preview(&mut self, ctx: &mut AppContext, update: &mut EventUpdate) {
        self.as_mut().event_preview_boxed(ctx, update);
    }

    fn event_ui(&mut self, ctx: &mut AppContext, update: &mut EventUpdate) {
        self.as_mut().event_ui_boxed(ctx, update);
    }

    fn event(&mut self, ctx: &mut AppContext, update: &mut EventUpdate) {
        self.as_mut().event_boxed(ctx, update);
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

struct TraceAppExt<E: AppExtension>(E);
impl<E: AppExtension> AppExtension for TraceAppExt<E> {
    fn id(&self) -> TypeId {
        self.0.id()
    }

    fn is_or_contain(&self, app_extension_id: TypeId) -> bool {
        self.0.is_or_contain(app_extension_id)
    }

    fn init(&mut self, ctx: &mut AppContext) {
        let _span = UpdatesTrace::extension_span::<E>("init");
        self.0.init(ctx);
    }

    fn enable_device_events(&self) -> bool {
        self.0.enable_device_events()
    }

    fn event_preview(&mut self, ctx: &mut AppContext, update: &mut EventUpdate) {
        let _span = UpdatesTrace::extension_span::<E>("event_preview");
        self.0.event_preview(ctx, update);
    }

    fn event_ui(&mut self, ctx: &mut AppContext, update: &mut EventUpdate) {
        let _span = UpdatesTrace::extension_span::<E>("event_ui");
        self.0.event_ui(ctx, update);
    }

    fn event(&mut self, ctx: &mut AppContext, update: &mut EventUpdate) {
        let _span = UpdatesTrace::extension_span::<E>("event");
        self.0.event(ctx, update);
    }

    fn update_preview(&mut self, ctx: &mut AppContext) {
        let _span = UpdatesTrace::extension_span::<E>("update_preview");
        self.0.update_preview(ctx);
    }

    fn update_ui(&mut self, ctx: &mut AppContext, updates: &mut WidgetUpdates) {
        let _span = UpdatesTrace::extension_span::<E>("update_ui");
        self.0.update_ui(ctx, updates);
    }

    fn update(&mut self, ctx: &mut AppContext) {
        let _span = UpdatesTrace::extension_span::<E>("update");
        self.0.update(ctx);
    }

    fn layout(&mut self, ctx: &mut AppContext) {
        let _span = UpdatesTrace::extension_span::<E>("layout");
        self.0.layout(ctx);
    }

    fn render(&mut self, ctx: &mut AppContext) {
        let _span = UpdatesTrace::extension_span::<E>("render");
        self.0.render(ctx);
    }

    fn deinit(&mut self, ctx: &mut AppContext) {
        let _span = UpdatesTrace::extension_span::<E>("deinit");
        self.0.deinit(ctx);
    }

    fn boxed(self) -> Box<dyn AppExtensionBoxed>
    where
        Self: Sized,
    {
        Box::new(self)
    }
}

event_args! {
    /// Arguments for [`EXIT_REQUESTED_EVENT`].
    ///
    /// Requesting [`propagation().stop()`] on this event cancels the exit.
    ///
    /// [`propagation().stop()`]: crate::event::EventPropagationHandle::stop
    pub struct ExitRequestedArgs {
        ..
        /// Broadcast to all.
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.search_all()
        }
    }
}

event! {
    /// Cancellable event raised when app process exit is requested.
    ///
    /// App exit can be requested using the [`AppProcess`] service or the [`EXIT_CMD`], some extensions
    /// also request exit if some conditions are met, [`WindowManager`] requests it after the last window
    /// is closed for example.
    ///
    /// Requesting [`propagation().stop()`] on this event cancels the exit.
    ///
    /// [`propagation().stop()`]: crate::event::EventPropagationHandle::stop
    pub static EXIT_REQUESTED_EVENT: ExitRequestedArgs;
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
    pub fn is_running() -> bool {
        crate::var::Vars::instantiated() || crate::event::Events::instantiated()
    }

    /// Returns a [`WindowMode`] value that indicates if the app is headless, headless with renderer or headed.
    ///
    /// Note that specific windows can be in headless modes even if the app is headed.
    pub fn window_mode(services: &mut crate::service::Services) -> WindowMode {
        services
            .get::<crate::app::view_process::ViewProcess>()
            .map(|p| {
                if p.headless() {
                    WindowMode::HeadlessWithRenderer
                } else {
                    WindowMode::Headed
                }
            })
            .unwrap_or(WindowMode::Headless)
    }
}

fn assert_not_view_process() {
    if zero_ui_view_api::ViewConfig::from_env().is_some() {
        panic!("cannot start App in view-process");
    }
}

// In release mode we use generics tricks to compile all app extensions with
// static dispatch optimized to a direct call to the extension handle.
#[cfg(not(dyn_app_extension))]
impl App {
    /// Application without any extension.
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
    /// * [`ConfigManager`]
    /// * [`MouseManager`]
    /// * [`KeyboardManager`]
    /// * [`GestureManager`]
    /// * [`WindowManager`]
    /// * [`FontManager`]
    /// * [`FocusManager`]
    /// * [`ImageManager`]
    pub fn default() -> AppExtended<impl AppExtension> {
        App::blank()
            .extend(ConfigManager::default())
            .extend(MouseManager::default())
            .extend(KeyboardManager::default())
            .extend(GestureManager::default())
            .extend(WindowManager::default())
            .extend(FontManager::default())
            .extend(FocusManager::default())
            .extend(ImageManager::default())
    }
}

// In "dyn_app_extension" mode we use dynamic dispatch to reduce the number of types
// in the stack-trace and compile more quickly.
#[cfg(dyn_app_extension)]
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
    /// * [`ConfigManager`]
    /// * [`MouseManager`]
    /// * [`KeyboardManager`]
    /// * [`GestureManager`]
    /// * [`WindowManager`]
    /// * [`FontManager`]
    /// * [`FocusManager`]
    /// * [`ImageManager`]
    pub fn default() -> AppExtended<Vec<Box<dyn AppExtensionBoxed>>> {
        App::blank()
            .extend(ConfigManager::default())
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
///
/// See [`App`].
pub struct AppExtended<E: AppExtension> {
    extensions: E,
    view_process_exe: Option<PathBuf>,
}

#[cfg(dyn_app_extension)]
impl AppExtended<Vec<Box<dyn AppExtensionBoxed>>> {
    /// Includes an application extension.
    ///
    /// # Panics
    ///
    /// * `"app already extended with `{}`"` when the app is already [`extended_with`](AppExtended::extended_with) the
    /// extension type.
    pub fn extend<F: AppExtension>(mut self, extension: F) -> AppExtended<Vec<Box<dyn AppExtensionBoxed>>> {
        if self.extended_with::<F>() {
            panic!("app already extended with `{}`", type_name::<F>())
        }

        self.extensions.push(TraceAppExt(extension).boxed());

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

#[cfg(not(dyn_app_extension))]
impl<E: AppExtension> AppExtended<E> {
    /// Includes an application extension.
    ///
    /// # Panics
    ///
    /// * `"app already extended with `{}`"` when the app is already [`extended_with`](AppExtended::extended_with) the
    /// extension type.
    pub fn extend<F: AppExtension>(self, extension: F) -> AppExtended<impl AppExtension> {
        if self.extended_with::<F>() {
            panic!("app already extended with `{}`", type_name::<F>())
        }
        AppExtended {
            extensions: (self.extensions, TraceAppExt(extension)),
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
    /// This method only returns when the app has exited.
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
    extensions: (AppIntrinsic, E),

    device_events: bool,
    owned_ctx: OwnedAppContext,
    receiver: flume::Receiver<AppEvent>,

    loop_timer: LoopTimer,
    loop_monitor: LoopMonitor,

    pending_view_events: Vec<zero_ui_view_api::Event>,
    pending_view_frame_events: Vec<zero_ui_view_api::EventFrameRendered>,
    pending_app_events: Vec<EventUpdate>,
    pending_layout: bool,
    pending_render: bool,
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

        let device_events = extensions.enable_device_events();
        let process = AppIntrinsic::pre_init(&mut ctx, is_headed, with_renderer, view_process_exe, device_events);

        {
            let _s = tracing::debug_span!("extensions.init").entered();
            extensions.init(&mut ctx);
        }

        RunningApp {
            extensions: (process, extensions),

            device_events,
            owned_ctx,
            receiver,

            loop_timer: LoopTimer::default(),
            loop_monitor: LoopMonitor::default(),

            pending_view_events: Vec::with_capacity(100),
            pending_view_frame_events: Vec::with_capacity(5),
            pending_app_events: Vec::with_capacity(100),
            pending_layout: false,
            pending_render: false,
        }
    }

    /// If device events are enabled in this app.
    pub fn device_events(&self) -> bool {
        self.device_events
    }

    /// Exclusive borrow the app context.
    pub fn ctx(&mut self) -> AppContext {
        self.owned_ctx.borrow()
    }

    /// Borrow the [`Vars`] only.
    pub fn vars(&self) -> &Vars {
        self.owned_ctx.vars()
    }

    /// Borrow the [`Services`] only.
    pub fn services(&mut self) -> &mut Services {
        self.owned_ctx.services()
    }

    /// Notify an event directly to the app extensions.
    pub fn notify_event<O: AppEventObserver>(&mut self, mut update: EventUpdate, observer: &mut O) {
        let _scope = tracing::trace_span!("notify_event", event = update.event().name()).entered();

        let ctx = &mut self.owned_ctx.borrow();

        self.extensions.event_preview(ctx, &mut update);
        observer.event_preview(ctx, &mut update);
        update.call_pre_actions(ctx);

        self.extensions.event_ui(ctx, &mut update);
        observer.event_ui(ctx, &mut update);

        self.extensions.event(ctx, &mut update);
        observer.event(ctx, &mut update);
        update.call_pos_actions(ctx);
    }

    fn device_id(&mut self, id: zero_ui_view_api::DeviceId) -> DeviceId {
        ViewProcess::req(self.ctx().services).device_id(id)
    }

    /// Process a View Process event.
    fn on_view_event<O: AppEventObserver>(&mut self, ev: zero_ui_view_api::Event, observer: &mut O) {
        use raw_device_events::*;
        use raw_events::*;
        use zero_ui_view_api::Event;

        fn window_id(id: zero_ui_view_api::WindowId) -> WindowId {
            WindowId::from_raw(id)
        }

        match ev {
            Event::CursorMoved {
                window: w_id,
                device: d_id,
                coalesced_pos,
                position,
            } => {
                let args = RawCursorMovedArgs::now(window_id(w_id), self.device_id(d_id), coalesced_pos, position);
                self.notify_event(RAW_CURSOR_MOVED_EVENT.new_update(args), observer);
            }
            Event::CursorEntered {
                window: w_id,
                device: d_id,
            } => {
                let args = RawCursorArgs::now(window_id(w_id), self.device_id(d_id));
                self.notify_event(RAW_CURSOR_ENTERED_EVENT.new_update(args), observer);
            }
            Event::CursorLeft {
                window: w_id,
                device: d_id,
            } => {
                let args = RawCursorArgs::now(window_id(w_id), self.device_id(d_id));
                self.notify_event(RAW_CURSOR_LEFT_EVENT.new_update(args), observer);
            }
            Event::WindowChanged(c) => {
                let monitor_id = c.monitor.map(|(id, f)| {
                    let view = ViewProcess::req(self.ctx().services);
                    (view.monitor_id(id), crate::units::Factor(f))
                });
                let args = RawWindowChangedArgs::now(
                    window_id(c.window),
                    c.state,
                    c.position,
                    monitor_id,
                    c.size,
                    c.cause,
                    c.frame_wait_id,
                );
                self.notify_event(RAW_WINDOW_CHANGED_EVENT.new_update(args), observer);
            }
            Event::DroppedFile { window: w_id, file } => {
                let args = RawDroppedFileArgs::now(window_id(w_id), file);
                self.notify_event(RAW_DROPPED_FILE_EVENT.new_update(args), observer);
            }
            Event::HoveredFile { window: w_id, file } => {
                let args = RawHoveredFileArgs::now(window_id(w_id), file);
                self.notify_event(RAW_HOVERED_FILE_EVENT.new_update(args), observer);
            }
            Event::HoveredFileCancelled(w_id) => {
                let args = RawHoveredFileCancelledArgs::now(window_id(w_id));
                self.notify_event(RAW_HOVERED_FILE_CANCELLED_EVENT.new_update(args), observer);
            }
            Event::ReceivedCharacter(w_id, c) => {
                let args = RawCharInputArgs::now(window_id(w_id), c);
                self.notify_event(RAW_CHAR_INPUT_EVENT.new_update(args), observer);
            }
            Event::FocusChanged { prev, new } => {
                let args = RawWindowFocusArgs::now(prev.map(window_id), new.map(window_id));
                self.notify_event(RAW_WINDOW_FOCUS_EVENT.new_update(args), observer);
            }
            Event::KeyboardInput {
                window: w_id,
                device: d_id,
                scan_code,
                state,
                key,
            } => {
                let args = RawKeyInputArgs::now(window_id(w_id), self.device_id(d_id), scan_code, state, key);
                self.notify_event(RAW_KEY_INPUT_EVENT.new_update(args), observer);
            }

            Event::MouseWheel {
                window: w_id,
                device: d_id,
                delta,
                phase,
            } => {
                let args = RawMouseWheelArgs::now(window_id(w_id), self.device_id(d_id), delta, phase);
                self.notify_event(RAW_MOUSE_WHEEL_EVENT.new_update(args), observer);
            }
            Event::MouseInput {
                window: w_id,
                device: d_id,
                state,
                button,
            } => {
                let args = RawMouseInputArgs::now(window_id(w_id), self.device_id(d_id), state, button);
                self.notify_event(RAW_MOUSE_INPUT_EVENT.new_update(args), observer);
            }
            Event::TouchpadPressure {
                window: w_id,
                device: d_id,
                pressure,
                stage,
            } => {
                let args = RawTouchpadPressureArgs::now(window_id(w_id), self.device_id(d_id), pressure, stage);
                self.notify_event(RAW_TOUCHPAD_PRESSURE_EVENT.new_update(args), observer);
            }
            Event::AxisMotion(w_id, d_id, axis, value) => {
                let args = RawAxisMotionArgs::now(window_id(w_id), self.device_id(d_id), axis, value);
                self.notify_event(RAW_AXIS_MOTION_EVENT.new_update(args), observer);
            }
            Event::Touch(w_id, d_id, phase, pos, force, finger_id) => {
                let args = RawTouchArgs::now(window_id(w_id), self.device_id(d_id), phase, pos, force, finger_id);
                self.notify_event(RAW_TOUCH_EVENT.new_update(args), observer);
            }
            Event::ScaleFactorChanged {
                monitor: id,
                windows,
                scale_factor,
            } => {
                let view = ViewProcess::req(self.ctx().services);
                let monitor_id = view.monitor_id(id);
                let windows: Vec<_> = windows.into_iter().map(window_id).collect();
                let args = RawScaleFactorChangedArgs::now(monitor_id, windows, scale_factor);
                self.notify_event(RAW_SCALE_FACTOR_CHANGED_EVENT.new_update(args), observer);
            }
            Event::MonitorsChanged(monitors) => {
                let view = ViewProcess::req(self.ctx().services);
                let monitors: Vec<_> = monitors.into_iter().map(|(id, info)| (view.monitor_id(id), info)).collect();
                let args = RawMonitorsChangedArgs::now(monitors);
                self.notify_event(RAW_MONITORS_CHANGED_EVENT.new_update(args), observer);
            }
            Event::ColorSchemeChanged(w_id, scheme) => {
                let args = RawColorSchemeChangedArgs::now(window_id(w_id), scheme);
                self.notify_event(RAW_COLOR_SCHEME_CHANGED_EVENT.new_update(args), observer);
            }
            Event::WindowCloseRequested(w_id) => {
                let args = RawWindowCloseRequestedArgs::now(window_id(w_id));
                self.notify_event(RAW_WINDOW_CLOSE_REQUESTED_EVENT.new_update(args), observer);
            }
            Event::WindowOpened(w_id, data) => {
                let w_id = window_id(w_id);
                let (window, data) = ViewProcess::req(self.ctx().services).on_window_opened(w_id, data);
                let args = RawWindowOpenArgs::now(w_id, window, data);
                self.notify_event(RAW_WINDOW_OPEN_EVENT.new_update(args), observer);
            }
            Event::HeadlessOpened(w_id, data) => {
                let w_id = window_id(w_id);
                let (surface, data) = ViewProcess::req(self.ctx().services).on_headless_opened(w_id, data);
                let args = RawHeadlessOpenArgs::now(w_id, surface, data);
                self.notify_event(RAW_HEADLESS_OPEN_EVENT.new_update(args), observer);
            }
            Event::WindowOrHeadlessOpenError { id: w_id, error } => {
                let w_id = window_id(w_id);
                let args = RawWindowOrHeadlessOpenErrorArgs::now(w_id, error);
                self.notify_event(RAW_WINDOW_OR_HEADLESS_OPEN_ERROR_EVENT.new_update(args), observer);
            }
            Event::WindowClosed(w_id) => {
                let args = RawWindowCloseArgs::now(window_id(w_id));
                self.notify_event(RAW_WINDOW_CLOSE_EVENT.new_update(args), observer);
            }
            Event::ImageMetadataLoaded { image: id, size, ppi } => {
                let view = ViewProcess::req(self.ctx().services);
                if let Some(img) = view.on_image_metadata_loaded(id, size, ppi) {
                    let args = RawImageArgs::now(img);
                    self.notify_event(RAW_IMAGE_METADATA_LOADED_EVENT.new_update(args), observer);
                }
            }
            Event::ImagePartiallyLoaded {
                image: id,
                partial_size,
                ppi,
                opaque,
                partial_bgra8,
            } => {
                let view = ViewProcess::req(self.ctx().services);
                if let Some(img) = view.on_image_partially_loaded(id, partial_size, ppi, opaque, partial_bgra8) {
                    let args = RawImageArgs::now(img);
                    self.notify_event(RAW_IMAGE_PARTIALLY_LOADED_EVENT.new_update(args), observer);
                }
            }
            Event::ImageLoaded(image) => {
                let view = ViewProcess::req(self.ctx().services);
                if let Some(img) = view.on_image_loaded(image) {
                    let args = RawImageArgs::now(img);
                    self.notify_event(RAW_IMAGE_LOADED_EVENT.new_update(args), observer);
                }
            }
            Event::ImageLoadError { image: id, error } => {
                let view = ViewProcess::req(self.ctx().services);
                if let Some(img) = view.on_image_error(id, error) {
                    let args = RawImageArgs::now(img);
                    self.notify_event(RAW_IMAGE_LOAD_ERROR_EVENT.new_update(args), observer);
                }
            }
            Event::ImageEncoded { image: id, format, data } => {
                let view = ViewProcess::req(self.ctx().services);
                view.on_image_encoded(id, format, data)
            }
            Event::ImageEncodeError { image: id, format, error } => {
                let view = ViewProcess::req(self.ctx().services);
                view.on_image_encode_error(id, format, error);
            }
            Event::FrameImageReady {
                window: w_id,
                frame: frame_id,
                image: image_id,
                selection,
            } => {
                let view = ViewProcess::req(self.ctx().services);
                if let Some(img) = view.on_frame_image_ready(image_id) {
                    let args = RawFrameImageReadyArgs::now(img, window_id(w_id), frame_id, selection);
                    self.notify_event(RAW_FRAME_IMAGE_READY_EVENT.new_update(args), observer);
                }
            }

            // config events
            Event::FontsChanged => {
                let args = RawFontChangedArgs::now();
                self.notify_event(RAW_FONT_CHANGED_EVENT.new_update(args), observer);
            }
            Event::FontAaChanged(aa) => {
                let args = RawFontAaChangedArgs::now(aa);
                self.notify_event(RAW_FONT_AA_CHANGED_EVENT.new_update(args), observer);
            }
            Event::MultiClickConfigChanged(cfg) => {
                let args = RawMultiClickConfigChangedArgs::now(cfg);
                self.notify_event(RAW_MULTI_CLICK_CONFIG_CHANGED_EVENT.new_update(args), observer);
            }
            Event::AnimationsEnabledChanged(enabled) => {
                let args = RawAnimationsEnabledChangedArgs::now(enabled);
                self.notify_event(RAW_ANIMATIONS_ENABLED_CHANGED_EVENT.new_update(args), observer);
            }
            Event::KeyRepeatDelayChanged(delay) => {
                let args = RawKeyRepeatDelayChangedArgs::now(delay);
                self.notify_event(RAW_KEY_REPEAT_DELAY_CHANGED_EVENT.new_update(args), observer);
            }

            // `device_events`
            Event::DeviceAdded(d_id) => {
                let args = DeviceArgs::now(self.device_id(d_id));
                self.notify_event(DEVICE_ADDED_EVENT.new_update(args), observer);
            }
            Event::DeviceRemoved(d_id) => {
                let args = DeviceArgs::now(self.device_id(d_id));
                self.notify_event(DEVICE_REMOVED_EVENT.new_update(args), observer);
            }
            Event::DeviceMouseMotion { device: d_id, delta } => {
                let args = MouseMotionArgs::now(self.device_id(d_id), delta);
                self.notify_event(MOUSE_MOTION_EVENT.new_update(args), observer);
            }
            Event::DeviceMouseWheel { device: d_id, delta } => {
                let args = MouseWheelArgs::now(self.device_id(d_id), delta);
                self.notify_event(MOUSE_WHEEL_EVENT.new_update(args), observer);
            }
            Event::DeviceMotion { device: d_id, axis, value } => {
                let args = MotionArgs::now(self.device_id(d_id), axis, value);
                self.notify_event(MOTION_EVENT.new_update(args), observer);
            }
            Event::DeviceButton {
                device: d_id,
                button,
                state,
            } => {
                let args = ButtonArgs::now(self.device_id(d_id), button, state);
                self.notify_event(BUTTON_EVENT.new_update(args), observer);
            }
            Event::DeviceKey {
                device: d_id,
                scan_code,
                state,
                key,
            } => {
                let args = KeyArgs::now(self.device_id(d_id), scan_code, state, key);
                self.notify_event(KEY_EVENT.new_update(args), observer);
            }
            Event::DeviceText(d_id, c) => {
                let args = TextArgs::now(self.device_id(d_id), c);
                self.notify_event(TEXT_EVENT.new_update(args), observer);
            }

            // Others
            Event::Inited { .. } | Event::Disconnected(_) | Event::FrameRendered(_) => unreachable!(), // handled before coalesce.
        }
    }

    /// Process a [`Event::FrameRendered`] event.
    fn on_view_rendered_event<O: AppEventObserver>(&mut self, ev: zero_ui_view_api::EventFrameRendered, observer: &mut O) {
        debug_assert!(ev.window != 0);
        let window_id = WindowId::from_raw(ev.window);
        let view = ViewProcess::req(self.ctx().services);
        // view.on_frame_rendered(window_id); // already called in push_coalesce
        let image = ev.frame_image.map(|img| view.on_frame_image(img));
        let args = raw_events::RawFrameRenderedArgs::now(window_id, ev.frame, image);
        self.notify_event(raw_events::RAW_FRAME_RENDERED_EVENT.new_update(args), observer);
    }

    fn run_headed(mut self) {
        self.apply_updates(&mut ());
        self.apply_update_events(&mut ());
        let mut wait = false;
        loop {
            wait = match self.poll(wait, &mut ()) {
                ControlFlow::Poll => false,
                ControlFlow::Wait => true,
                ControlFlow::Exit => return,
            };
        }
    }

    fn push_coalesce<O: AppEventObserver>(&mut self, ev: AppEvent, observer: &mut O) {
        match ev {
            AppEvent::ViewEvent(ev) => match ev {
                zero_ui_view_api::Event::FrameRendered(ev) => {
                    if ev.window == 0 {
                        tracing::error!("ignored rendered event for invalid window id 0, {ev:?}");
                        return;
                    }

                    let window = WindowId::from_raw(ev.window);

                    // update ViewProcess immediately.
                    if let Some(vp) = self.ctx().services.get::<ViewProcess>() {
                        vp.on_frame_rendered(window);
                    }

                    #[cfg(debug_assertions)]
                    if self.pending_view_frame_events.iter().any(|e| e.window == ev.window) {
                        tracing::warn!("window `{window:?}` probably sent a frame request without awaiting renderer idle");
                    }

                    self.pending_view_frame_events.push(ev);
                }
                zero_ui_view_api::Event::Inited {
                    generation,
                    is_respawn,
                    available_monitors,
                    multi_click_config,
                    key_repeat_delay,
                    font_aa,
                    animations_enabled,
                } => {
                    // notify immediately.
                    if is_respawn {
                        ViewProcess::req(&mut self.ctx()).on_respawed(generation);

                        // discard pending events.
                        self.pending_app_events.clear();
                    }

                    let view = ViewProcess::req(self.ctx().services);
                    view.handle_inited(generation);

                    let monitors: Vec<_> = available_monitors
                        .into_iter()
                        .map(|(id, info)| (view.monitor_id(id), info))
                        .collect();
                    let args = ViewProcessInitedArgs::now(
                        generation,
                        is_respawn,
                        monitors,
                        multi_click_config,
                        key_repeat_delay,
                        font_aa,
                        animations_enabled,
                    );
                    self.notify_event(VIEW_PROCESS_INITED_EVENT.new_update(args), observer);
                }
                zero_ui_view_api::Event::Disconnected(gen) => {
                    // update ViewProcess immediately.
                    ViewProcess::req(self.ctx().services).handle_disconnect(gen);
                }
                ev => {
                    if let Some(last) = self.pending_view_events.last_mut() {
                        match last.coalesce(ev) {
                            Ok(()) => {}
                            Err(ev) => self.pending_view_events.push(ev),
                        }
                    } else {
                        self.pending_view_events.push(ev);
                    }
                }
            },
            AppEvent::Event(ev) => self.ctx().events.notify(ev.get()),
            AppEvent::Var => self.ctx().vars.receive_sended_modify(),
            AppEvent::Update(targets) => self.ctx().updates.recv_update_internal(targets),
            AppEvent::ResumeUnwind(p) => std::panic::resume_unwind(p),
        }
    }

    fn has_pending_updates(&mut self) -> bool {
        !self.pending_view_events.is_empty()
            || !self.pending_app_events.is_empty()
            || self.owned_ctx.has_pending_updates()
            || !self.receiver.is_empty()
    }

    fn poll<O: AppEventObserver>(&mut self, wait_app_event: bool, observer: &mut O) -> ControlFlow {
        #[cfg(dyn_app_extension)]
        let mut observer = observer.as_dyn();
        #[cfg(dyn_app_extension)]
        let observer = &mut observer;
        self.poll_impl(wait_app_event, observer)
    }
    fn poll_impl<O: AppEventObserver>(&mut self, wait_app_event: bool, observer: &mut O) -> ControlFlow {
        let mut disconnected = false;

        if wait_app_event {
            let idle = tracing::debug_span!("<idle>", ended_by = tracing::field::Empty).entered();

            let timer = if self.view_is_busy() { None } else { self.loop_timer.poll() };
            if let Some(time) = timer {
                match self.receiver.recv_deadline_sp(time) {
                    Ok(ev) => {
                        idle.record("ended_by", "event");
                        drop(idle);
                        self.push_coalesce(ev, observer)
                    }
                    Err(e) => match e {
                        flume::RecvTimeoutError::Timeout => {
                            idle.record("ended_by", "timeout");
                        }
                        flume::RecvTimeoutError::Disconnected => {
                            idle.record("ended_by", "disconnected");
                            disconnected = true
                        }
                    },
                }
            } else {
                match self.receiver.recv() {
                    Ok(ev) => {
                        idle.record("ended_by", "event");
                        drop(idle);
                        self.push_coalesce(ev, observer)
                    }
                    Err(e) => match e {
                        flume::RecvError::Disconnected => {
                            idle.record("ended_by", "disconnected");
                            disconnected = true
                        }
                    },
                }
            }
        }
        loop {
            match self.receiver.try_recv() {
                Ok(ev) => self.push_coalesce(ev, observer),
                Err(e) => match e {
                    flume::TryRecvError::Empty => break,
                    flume::TryRecvError::Disconnected => {
                        disconnected = true;
                        break;
                    }
                },
            }
        }
        if disconnected {
            panic!("app events channel disconnected");
        }

        if self.view_is_busy() {
            return ControlFlow::Wait;
        }

        // clear timers.
        let updated_timers = self.loop_timer.awake();
        if updated_timers {
            // tick timers and collect not elapsed timers.
            self.owned_ctx.update_timers(&mut self.loop_timer);
            self.apply_updates(observer);
        }

        let mut events = mem::take(&mut self.pending_view_events);
        for ev in events.drain(..) {
            self.on_view_event(ev, observer);
            self.apply_updates(observer);
        }
        debug_assert!(self.pending_view_events.is_empty());
        self.pending_view_events = events; // reuse capacity

        let mut events = mem::take(&mut self.pending_view_frame_events);
        for ev in events.drain(..) {
            self.on_view_rendered_event(ev, observer);
        }
        self.pending_view_frame_events = events;

        if self.has_pending_updates() {
            self.apply_updates(observer);
            self.apply_update_events(observer);
        }

        if self.view_is_busy() {
            return ControlFlow::Wait;
        }

        self.finish_frame(observer);

        self.owned_ctx.next_deadline(&mut self.loop_timer);

        if self.extensions.0.exit(self.owned_ctx.vars()) {
            ControlFlow::Exit
        } else if self.has_pending_updates() {
            ControlFlow::Poll
        } else {
            ControlFlow::Wait
        }
    }

    /// Does updates, collects pending update generated events and layout + render.
    fn apply_updates<O: AppEventObserver>(&mut self, observer: &mut O) {
        let _s = tracing::debug_span!("apply_updates").entered();

        let mut run = true;
        while run {
            run = self.loop_monitor.update(|| {
                let u = self.owned_ctx.apply_updates();

                Timers::notify(&mut self.owned_ctx.borrow());

                self.pending_app_events.extend(u.events);
                self.pending_layout |= u.layout;
                self.pending_render |= u.render;

                if !u.update {
                    return false;
                }

                let _s = tracing::debug_span!("extensions").entered();

                let ctx = &mut self.owned_ctx.borrow();

                self.extensions.update_preview(ctx);
                observer.update_preview(ctx);
                Updates::on_pre_updates(ctx);

                let mut wgt_updates = u.update_widgets;
                self.extensions.update_ui(ctx, &mut wgt_updates);
                observer.update_ui(ctx, &mut wgt_updates);

                self.extensions.update(ctx);
                observer.update(ctx);
                Updates::on_updates(ctx);

                true
            });
        }
    }

    // apply the current pending update generated events.
    fn apply_update_events<O: AppEventObserver>(&mut self, observer: &mut O) {
        let _s = tracing::debug_span!("apply_update_events").entered();

        loop {
            let events: Vec<_> = self.pending_app_events.drain(..).collect();
            if events.is_empty() {
                break;
            }
            for mut update in events {
                let _s = tracing::debug_span!("update_event", ?update).entered();

                let ctx = &mut self.owned_ctx.borrow();

                self.loop_monitor.maybe_trace(|| {
                    self.extensions.event_preview(ctx, &mut update);
                    observer.event_preview(ctx, &mut update);
                    update.call_pre_actions(ctx);

                    self.extensions.event_ui(ctx, &mut update);
                    observer.event_ui(ctx, &mut update);

                    self.extensions.event(ctx, &mut update);
                    observer.event(ctx, &mut update);
                    update.call_pos_actions(ctx);
                });

                self.apply_updates(observer);
            }
        }
    }

    fn view_is_busy(&mut self) -> bool {
        self.owned_ctx
            .borrow()
            .services
            .get::<ViewProcess>()
            .map(|vp| !vp.online() || vp.pending_frames() > 0)
            .unwrap_or(false)
    }

    // apply pending layout & render if the view-process is not already rendering.
    fn finish_frame<O: AppEventObserver>(&mut self, observer: &mut O) {
        debug_assert!(!self.view_is_busy());

        while mem::take(&mut self.pending_layout) {
            let _s = tracing::debug_span!("apply_layout").entered();

            let ctx = &mut self.owned_ctx.borrow();

            self.loop_monitor.maybe_trace(|| {
                self.extensions.layout(ctx);
                observer.layout(ctx);
            });

            self.apply_updates(observer);
        }

        if mem::take(&mut self.pending_render) {
            let _s = tracing::debug_span!("apply_render").entered();

            let ctx = &mut self.owned_ctx.borrow();

            self.extensions.render(ctx);
            observer.render(ctx);
        }

        self.loop_monitor.finish_frame();
    }
}
impl<E: AppExtension> Drop for RunningApp<E> {
    fn drop(&mut self) {
        let _s = tracing::debug_span!("extensions.deinit").entered();
        let mut ctx = self.owned_ctx.borrow();
        self.extensions.deinit(&mut ctx);
    }
}

#[cfg(dyn_app_extension)]
share_generics!(RunningApp<Box<dyn AppExtensionBoxed>>::start);

/// App main loop timer.
#[derive(Debug)]
pub(crate) struct LoopTimer {
    now: Instant,
    deadline: Option<Deadline>,
}
impl Default for LoopTimer {
    fn default() -> Self {
        Self {
            now: Instant::now(),
            deadline: None,
        }
    }
}
impl LoopTimer {
    /// Returns `true` if the `deadline` has elapsed, `false` if the `deadline` was
    /// registered for future waking.
    pub fn elapsed(&mut self, deadline: Deadline) -> bool {
        if deadline.0 <= self.now {
            true
        } else {
            self.register(deadline);
            false
        }
    }

    /// Register the future `deadline`.
    pub fn register(&mut self, deadline: Deadline) {
        if let Some(d) = &mut self.deadline {
            if deadline < *d {
                *d = deadline;
            }
        } else {
            self.deadline = Some(deadline)
        }
    }

    /// Get next recv deadline.
    pub(crate) fn poll(&mut self) -> Option<Deadline> {
        self.deadline
    }

    /// Maybe awake timer.
    pub(crate) fn awake(&mut self) -> bool {
        self.now = Instant::now();
        if let Some(d) = self.deadline {
            if d.0 <= self.now {
                self.deadline = None;
                return true;
            }
        }
        false
    }

    /// Awake timestamp.
    pub fn now(&self) -> Instant {
        self.now
    }
}

#[derive(Default)]
struct LoopMonitor {
    update_count: u16,
    skipped: bool,
    trace: Vec<UpdateTrace>,
}
impl LoopMonitor {
    /// Returns `false` if the loop should break.
    pub fn update(&mut self, update_once: impl FnOnce() -> bool) -> bool {
        self.update_count += 1;

        if self.update_count < 500 {
            update_once()
        } else if self.update_count < 1000 {
            UpdatesTrace::collect_trace(&mut self.trace, update_once)
        } else if self.update_count == 1000 {
            self.skipped = true;
            let trace = UpdatesTrace::format_trace(mem::take(&mut self.trace));
            tracing::error!(
                "updated 1000 times without rendering, probably stuck in an infinite loop\n\
                 will start skipping updates to render and poll system events\n\
                 top 20 most frequent update requests (in 500 cycles):\n\
                 {trace}\n\
                    you can use `UpdatesTraceUiNodeExt` to refine the trace"
            );
            false
        } else if self.update_count == 1500 {
            self.skipped = true;
            self.update_count = 1001;
            false
        } else {
            update_once()
        }
    }

    pub fn maybe_trace(&mut self, notify_once: impl FnOnce()) {
        if (500..1000).contains(&self.update_count) {
            UpdatesTrace::collect_trace(&mut self.trace, notify_once);
        } else {
            notify_once();
        }
    }

    pub fn finish_frame(&mut self) {
        if !self.skipped {
            self.skipped = false;
            self.update_count = 0;
            self.trace = vec![];
        }
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
impl ControlFlow {
    /// Assert that the value is [`ControlFlow::Wait`].
    #[track_caller]
    pub fn assert_wait(self) {
        assert_eq!(ControlFlow::Wait, self)
    }

    /// Assert that the value is [`ControlFlow::Exit`].
    #[track_caller]
    pub fn assert_exit(self) {
        assert_eq!(ControlFlow::Exit, self)
    }
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
    pub fn app_state(&self) -> StateMapRef<state_map::App> {
        self.app.owned_ctx.app_state()
    }

    /// Mutable app state.
    pub fn app_state_mut(&mut self) -> StateMapMut<state_map::App> {
        self.app.owned_ctx.app_state_mut()
    }

    /// If headless rendering is enabled.
    ///
    /// When enabled windows are still not visible but you can request [frame pixels]
    /// to get the frame image. Renderer is disabled by default in a headless app.
    ///
    /// Apps with render enabled can only be initialized in the main thread due to limitations of some operating systems,
    /// this means you cannot run a headless renderer in units tests.
    ///
    /// Note that [`UiNode::render`] is still called when a renderer is disabled and you can still
    /// query the latest frame from [`Windows::widget_tree`]. The only thing that
    /// is disabled is WebRender and the generation of frame textures.
    ///
    /// [frame pixels]: crate::window::Windows::frame_image
    /// [`UiNode::render`]: crate::widget_instance::UiNode::render
    /// [`Windows::widget_tree`]: crate::window::Windows::widget_tree
    pub fn renderer_enabled(&mut self) -> bool {
        self.ctx().services.get::<ViewProcess>().is_some()
    }

    /// Borrows the app context.
    pub fn ctx(&mut self) -> AppContext {
        self.app.ctx()
    }

    /// Borrow the [`Vars`] only.
    pub fn vars(&self) -> &Vars {
        self.app.vars()
    }

    /// Borrow the [`Services`] only.
    pub fn services(&mut self) -> &mut Services {
        self.app.services()
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
    pub fn update_observe_event(&mut self, on_event: impl FnMut(&mut AppContext, &EventUpdate), wait_app_event: bool) -> ControlFlow {
        struct Observer<F>(F);
        impl<F: FnMut(&mut AppContext, &EventUpdate)> AppEventObserver for Observer<F> {
            fn event(&mut self, ctx: &mut AppContext, update: &mut EventUpdate) {
                (self.0)(ctx, update);
            }
        }
        let mut observer = Observer(on_event);
        self.update_observed(&mut observer, wait_app_event)
    }

    /// Does updates with an [`AppEventObserver`].
    ///
    /// If `wait_app_event` is `true` the thread sleeps until at least one app event is received or a timer elapses,
    /// if it is `false` only responds to app events already in the buffer.
    pub fn update_observed<O: AppEventObserver>(&mut self, observer: &mut O, mut wait_app_event: bool) -> ControlFlow {
        loop {
            match self.app.poll(wait_app_event, observer) {
                ControlFlow::Poll => {
                    wait_app_event = false;
                    continue;
                }
                flow => return flow,
            }
        }
    }

    /// Execute the async `task` in the UI thread, updating the app until it finishes or the app shuts-down.
    ///
    /// Returns the task result if the app has not shut-down.
    pub fn run_task<R, F, T>(&mut self, task: T) -> Option<R>
    where
        R: 'static,
        F: Future<Output = R> + 'static,
        T: FnOnce(AppContextMut) -> F,
    {
        let mut task = self.ctx().async_task(task);

        let mut flow = self.update_observe(
            |ctx| {
                task.update(ctx);
            },
            false,
        );

        if task.update(&mut self.ctx()).is_some() {
            let r = task.into_result().ok();
            debug_assert!(r.is_some());
            return r;
        }

        while flow != ControlFlow::Exit {
            flow = self.update_observe(
                |ctx| {
                    task.update(ctx);
                },
                true,
            );

            match task.into_result() {
                Ok(r) => return Some(r),
                Err(t) => task = t,
            }
        }

        None
    }

    /// Requests and wait for app exit.
    ///
    /// Forces deinit if exit is cancelled.
    pub fn exit(mut self) {
        self.run_task(|ctx| async move {
            let req = ctx.with(|ctx| AppProcess::req(ctx.services).exit());
            req.wait_rsp(&ctx).await;
        });
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
    fn event_preview(&mut self, ctx: &mut AppContext, update: &mut EventUpdate) {
        let _ = (ctx, update);
    }

    /// Called just after [`AppExtension::event_ui`].
    fn event_ui(&mut self, ctx: &mut AppContext, update: &mut EventUpdate) {
        let _ = (ctx, update);
    }

    /// Called just after [`AppExtension::event`].
    fn event(&mut self, ctx: &mut AppContext, update: &mut EventUpdate) {
        let _ = (ctx, update);
    }

    /// Called just after [`AppExtension::update_preview`].
    fn update_preview(&mut self, ctx: &mut AppContext) {
        let _ = ctx;
    }

    /// Called just after [`AppExtension::update_ui`].
    fn update_ui(&mut self, ctx: &mut AppContext, updates: &mut WidgetUpdates) {
        let _ = (ctx, updates);
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

    /// Cast to dynamically dispatched observer, this can help avoid code bloat.
    ///
    /// The app methods that accept observers automatically use this method if the feature `"dyn_app_extension"` is active.
    fn as_dyn(&mut self) -> DynAppEventObserver
    where
        Self: Sized,
    {
        DynAppEventObserver(self)
    }
}
/// Nil observer, does nothing.
impl AppEventObserver for () {}

#[doc(hidden)]
pub struct DynAppEventObserver<'a>(&'a mut dyn AppEventObserverDyn);

trait AppEventObserverDyn {
    fn raw_event_dyn(&mut self, ctx: &mut AppContext, ev: &zero_ui_view_api::Event);
    fn event_preview_dyn(&mut self, ctx: &mut AppContext, update: &mut EventUpdate);
    fn event_ui_dyn(&mut self, ctx: &mut AppContext, update: &mut EventUpdate);
    fn event_dyn(&mut self, ctx: &mut AppContext, update: &mut EventUpdate);
    fn update_preview_dyn(&mut self, ctx: &mut AppContext);
    fn update_ui_dyn(&mut self, ctx: &mut AppContext, updates: &mut WidgetUpdates);
    fn update_dyn(&mut self, ctx: &mut AppContext);
    fn layout_dyn(&mut self, ctx: &mut AppContext);
    fn render_dyn(&mut self, ctx: &mut AppContext);
}
impl<O: AppEventObserver> AppEventObserverDyn for O {
    fn raw_event_dyn(&mut self, ctx: &mut AppContext, ev: &zero_ui_view_api::Event) {
        self.raw_event(ctx, ev)
    }

    fn event_preview_dyn(&mut self, ctx: &mut AppContext, update: &mut EventUpdate) {
        self.event_preview(ctx, update)
    }

    fn event_ui_dyn(&mut self, ctx: &mut AppContext, update: &mut EventUpdate) {
        self.event_ui(ctx, update)
    }

    fn event_dyn(&mut self, ctx: &mut AppContext, update: &mut EventUpdate) {
        self.event(ctx, update)
    }

    fn update_preview_dyn(&mut self, ctx: &mut AppContext) {
        self.update_preview(ctx)
    }

    fn update_ui_dyn(&mut self, ctx: &mut AppContext, updates: &mut WidgetUpdates) {
        self.update_ui(ctx, updates)
    }

    fn update_dyn(&mut self, ctx: &mut AppContext) {
        self.update(ctx)
    }

    fn layout_dyn(&mut self, ctx: &mut AppContext) {
        self.layout(ctx)
    }

    fn render_dyn(&mut self, ctx: &mut AppContext) {
        self.render(ctx)
    }
}
impl<'a> AppEventObserver for DynAppEventObserver<'a> {
    fn raw_event(&mut self, ctx: &mut AppContext, ev: &zero_ui_view_api::Event) {
        self.0.raw_event_dyn(ctx, ev)
    }

    fn event_preview(&mut self, ctx: &mut AppContext, update: &mut EventUpdate) {
        self.0.event_preview_dyn(ctx, update)
    }

    fn event_ui(&mut self, ctx: &mut AppContext, update: &mut EventUpdate) {
        self.0.event_ui_dyn(ctx, update)
    }

    fn event(&mut self, ctx: &mut AppContext, update: &mut EventUpdate) {
        self.0.event_dyn(ctx, update)
    }

    fn update_preview(&mut self, ctx: &mut AppContext) {
        self.0.update_preview_dyn(ctx)
    }

    fn update_ui(&mut self, ctx: &mut AppContext, updates: &mut WidgetUpdates) {
        self.0.update_ui_dyn(ctx, updates)
    }

    fn update(&mut self, ctx: &mut AppContext) {
        self.0.update_dyn(ctx)
    }

    fn layout(&mut self, ctx: &mut AppContext) {
        self.0.layout_dyn(ctx)
    }

    fn render(&mut self, ctx: &mut AppContext) {
        self.0.render_dyn(ctx)
    }

    fn as_dyn(&mut self) -> DynAppEventObserver {
        DynAppEventObserver(self.0)
    }
}

impl AppExtension for () {
    fn is_or_contain(&self, _: TypeId) -> bool {
        false
    }
}
impl<A: AppExtension, B: AppExtension> AppExtension for (A, B) {
    fn init(&mut self, ctx: &mut AppContext) {
        self.0.init(ctx);
        self.1.init(ctx);
    }

    fn is_or_contain(&self, app_extension_id: TypeId) -> bool {
        self.0.is_or_contain(app_extension_id) || self.1.is_or_contain(app_extension_id)
    }

    fn enable_device_events(&self) -> bool {
        self.0.enable_device_events() || self.1.enable_device_events()
    }

    fn update_preview(&mut self, ctx: &mut AppContext) {
        self.0.update_preview(ctx);
        self.1.update_preview(ctx);
    }

    fn update_ui(&mut self, ctx: &mut AppContext, updates: &mut WidgetUpdates) {
        self.0.update_ui(ctx, updates);
        self.1.update_ui(ctx, updates);
    }

    fn update(&mut self, ctx: &mut AppContext) {
        self.0.update(ctx);
        self.1.update(ctx);
    }

    fn layout(&mut self, ctx: &mut AppContext) {
        self.0.layout(ctx);
        self.1.layout(ctx);
    }

    fn render(&mut self, ctx: &mut AppContext) {
        self.0.render(ctx);
        self.1.render(ctx);
    }

    fn event_preview(&mut self, ctx: &mut AppContext, update: &mut EventUpdate) {
        self.0.event_preview(ctx, update);
        self.1.event_preview(ctx, update);
    }

    fn event_ui(&mut self, ctx: &mut AppContext, update: &mut EventUpdate) {
        self.0.event_ui(ctx, update);
        self.1.event_ui(ctx, update);
    }

    fn event(&mut self, ctx: &mut AppContext, update: &mut EventUpdate) {
        self.0.event(ctx, update);
        self.1.event(ctx, update);
    }

    fn deinit(&mut self, ctx: &mut AppContext) {
        self.1.deinit(ctx);
        self.0.deinit(ctx);
    }
}

#[cfg(dyn_app_extension)]
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

    fn update_ui(&mut self, ctx: &mut AppContext, updates: &mut WidgetUpdates) {
        for ext in self {
            ext.update_ui(ctx, updates);
        }
    }

    fn update(&mut self, ctx: &mut AppContext) {
        for ext in self {
            ext.update(ctx);
        }
    }

    fn event_preview(&mut self, ctx: &mut AppContext, update: &mut EventUpdate) {
        for ext in self {
            ext.event_preview(ctx, update);
        }
    }

    fn event_ui(&mut self, ctx: &mut AppContext, update: &mut EventUpdate) {
        for ext in self {
            ext.event_ui(ctx, update);
        }
    }

    fn event(&mut self, ctx: &mut AppContext, update: &mut EventUpdate) {
        for ext in self {
            ext.event(ctx, update);
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
    Event(crate::event::EventUpdateMsg),
    /// Notify [`Vars`](crate::var::Vars).
    Var,
    /// Do an update cycle.
    Update(Vec<WidgetId>),
    /// Resume a panic in the app thread.
    ResumeUnwind(PanicPayload),
}

/// A sender that can awake apps and insert events into the main loop.
///
/// A Clone of the sender is available in [`Updates::sender`].
///
/// [`Updates::sender`]: crate::context::Updates::sender
#[derive(Clone)]
pub struct AppEventSender(flume::Sender<AppEvent>);
impl AppEventSender {
    pub(crate) fn new() -> (Self, flume::Receiver<AppEvent>) {
        let (sender, receiver) = flume::unbounded();
        (Self(sender), receiver)
    }

    fn send_app_event(&self, event: AppEvent) -> Result<(), AppDisconnected<AppEvent>> {
        self.0.send(event)?;
        Ok(())
    }

    fn send_view_event(&self, event: zero_ui_view_api::Event) -> Result<(), AppDisconnected<AppEvent>> {
        self.0.send(AppEvent::ViewEvent(event))?;
        Ok(())
    }

    /// Causes an update cycle to happen in the app.
    pub fn send_update(&self, targets: Vec<WidgetId>) -> Result<(), AppDisconnected<()>> {
        UpdatesTrace::log_update();
        self.send_app_event(AppEvent::Update(targets)).map_err(|_| AppDisconnected(()))
    }

    /// Causes an update cycle that only affects app extensions to happen in the app.
    ///
    /// This is the equivalent of calling [`send_update`] with an empty vec.
    ///
    /// [`send_update`]: Self::send_update
    pub fn send_ext_update(&self) -> Result<(), AppDisconnected<()>> {
        UpdatesTrace::log_update();
        self.send_update(vec![])
    }

    /// [`VarSender`](crate::var::VarSender) util.
    pub(crate) fn send_var(&self) -> Result<(), AppDisconnected<()>> {
        self.send_app_event(AppEvent::Var).map_err(|_| AppDisconnected(()))
    }

    /// [`EventSender`](crate::event::EventSender) util.
    pub(crate) fn send_event(&self, event: crate::event::EventUpdateMsg) -> Result<(), AppDisconnected<crate::event::EventUpdateMsg>> {
        self.send_app_event(AppEvent::Event(event)).map_err(|e| match e.0 {
            AppEvent::Event(ev) => AppDisconnected(ev),
            _ => unreachable!(),
        })
    }

    /// Resume a panic in the app thread.
    pub fn send_resume_unwind(&self, payload: PanicPayload) -> Result<(), AppDisconnected<PanicPayload>> {
        self.send_app_event(AppEvent::ResumeUnwind(payload)).map_err(|e| match e.0 {
            AppEvent::ResumeUnwind(p) => AppDisconnected(p),
            _ => unreachable!(),
        })
    }

    /// Create an [`Waker`] that causes a [`send_update`](Self::send_update).
    pub fn waker(&self, targets: Vec<WidgetId>) -> Waker {
        Arc::new(AppWaker(self.0.clone(), targets)).into()
    }

    /// Create an unbound channel that causes an extension update for each message received.
    pub fn ext_channel<T>(&self) -> (AppExtSender<T>, AppExtReceiver<T>) {
        let (sender, receiver) = flume::unbounded();

        (
            AppExtSender {
                update: self.clone(),
                sender,
            },
            AppExtReceiver {
                _not_send: PhantomData,
                receiver,
            },
        )
    }

    /// Create aa bounded channel that causes an extension update for each message received.
    pub fn ext_channel_bounded<T>(&self, cap: usize) -> (AppExtSender<T>, AppExtReceiver<T>) {
        let (sender, receiver) = flume::bounded(cap);

        (
            AppExtSender {
                update: self.clone(),
                sender,
            },
            AppExtReceiver {
                _not_send: PhantomData,
                receiver,
            },
        )
    }
}

struct AppWaker(flume::Sender<AppEvent>, Vec<WidgetId>);
impl std::task::Wake for AppWaker {
    fn wake(self: std::sync::Arc<Self>) {
        let _ = match std::sync::Arc::try_unwrap(self) {
            Ok(w) => w.0.send(AppEvent::Update(w.1)),
            Err(arc) => arc.0.send(AppEvent::Update(arc.1.clone())),
        };
    }
}

/// Represents a channel sender that causes an extensions update for each value transferred.
///
/// A channel can be created using the [`AppEventSender::ext_channel`] method.
pub struct AppExtSender<T> {
    update: AppEventSender,
    sender: flume::Sender<T>,
}
impl<T> Clone for AppExtSender<T> {
    fn clone(&self) -> Self {
        Self {
            update: self.update.clone(),
            sender: self.sender.clone(),
        }
    }
}
impl<T: Send> AppExtSender<T> {
    /// Send an extension update and `msg`, blocks until the app receives the message.
    pub fn send(&self, msg: T) -> Result<(), AppDisconnected<T>> {
        match self.update.send_ext_update() {
            Ok(()) => self.sender.send(msg).map_err(|e| AppDisconnected(e.0)),
            Err(_) => Err(AppDisconnected(msg)),
        }
    }

    /// Send an extension update and `msg`, blocks until the app receives the message or `dur` elapses.
    pub fn send_timeout(&self, msg: T, dur: Duration) -> Result<(), TimeoutOrAppDisconnected> {
        match self.update.send_ext_update() {
            Ok(()) => self.sender.send_timeout(msg, dur).map_err(|e| match e {
                flume::SendTimeoutError::Timeout(_) => TimeoutOrAppDisconnected::Timeout,
                flume::SendTimeoutError::Disconnected(_) => TimeoutOrAppDisconnected::AppDisconnected,
            }),
            Err(_) => Err(TimeoutOrAppDisconnected::AppDisconnected),
        }
    }

    /// Send an extension update and `msg`, blocks until the app receives the message or `deadline` is reached.
    pub fn send_deadline(&self, msg: T, deadline: Instant) -> Result<(), TimeoutOrAppDisconnected> {
        match self.update.send_ext_update() {
            Ok(()) => self.sender.send_deadline(msg, deadline).map_err(|e| match e {
                flume::SendTimeoutError::Timeout(_) => TimeoutOrAppDisconnected::Timeout,
                flume::SendTimeoutError::Disconnected(_) => TimeoutOrAppDisconnected::AppDisconnected,
            }),
            Err(_) => Err(TimeoutOrAppDisconnected::AppDisconnected),
        }
    }
}

/// Represents a channel receiver in an app extension.
///
/// See [`AppExtReceiver`] for details.
pub struct AppExtReceiver<T> {
    _not_send: PhantomData<std::rc::Rc<T>>,
    receiver: flume::Receiver<T>,
}
impl<T> Clone for AppExtReceiver<T> {
    fn clone(&self) -> Self {
        Self {
            _not_send: PhantomData,
            receiver: self.receiver.clone(),
        }
    }
}
impl<T> AppExtReceiver<T> {
    /// Receive an update if any was send.
    ///
    /// Returns `Ok(msg)` if there was at least one message, or returns `Err(None)` if there was no update or
    /// returns `Err(AppExtSenderDisconnected)` if the connected sender was dropped.
    pub fn try_recv(&self) -> Result<T, Option<AppExtSenderDisconnected>> {
        self.receiver.try_recv().map_err(|e| match e {
            flume::TryRecvError::Empty => None,
            flume::TryRecvError::Disconnected => Some(AppExtSenderDisconnected),
        })
    }
}

/// Error when the app connected to a sender/receiver channel has disconnected.
///
/// Contains the value that could not be send or `()` for receiver errors.
#[derive(Debug)]
pub struct AppExtSenderDisconnected;
impl fmt::Display for AppExtSenderDisconnected {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "cannot receive because the sender disconnected")
    }
}
impl std::error::Error for AppExtSenderDisconnected {}

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
