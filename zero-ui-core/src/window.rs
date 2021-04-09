//! App windows manager.
use crate::{
    app::{self, AppExtended, AppExtension, AppProcess, EventLoopProxy, EventLoopWindowTarget, ShutdownRequestedArgs},
    color::Rgba,
    context::*,
    event::*,
    profiler::profile_scope,
    render::{FrameBuilder, FrameHitInfo, FrameId, FrameInfo, FrameUpdate, WidgetTransformKey},
    service::WindowServicesVisitors,
    service::{AppService, WindowServices},
    text::Text,
    units::{LayoutPoint, LayoutRect, LayoutSize, PixelGrid, Point, Size},
    var::{BoxedLocalVar, BoxedVar, IntoVar, VarLocal, VarObj, Vars},
    UiNode, WidgetId,
};

use app::AppEvent;
use fnv::FnvHashMap;

use gleam::gl;

use glutin::{
    window::{Window as GlutinWindow, WindowBuilder},
    Api, ContextBuilder, GlRequest, NotCurrent, PossiblyCurrent, WindowedContext,
};
use rayon::{ThreadPool, ThreadPoolBuilder};
use std::{cell::RefCell, mem, num::NonZeroU16, rc::Rc, sync::Arc};
use webrender::api::{units, DocumentId, Epoch, HitTestFlags, PipelineId, RenderApi, RenderNotifier, Transaction};

pub use glutin::{event::WindowEvent, window::CursorIcon};

type HeadedEventLoopWindowTarget = glutin::event_loop::EventLoopWindowTarget<app::AppEvent>;
type CloseTogetherGroup = Option<NonZeroU16>;

unique_id! {
    /// Unique identifier of a headless window.
    ///
    /// See [`WindowId`] for more details.
    pub struct LogicalWindowId;
}

/// Unique identifier of a headed window or a headless window backed by a hidden system window.
///
/// See [`WindowId`] for more details.
pub type SystemWindowId = glutin::window::WindowId;

/// Unique identifier of a [`OpenWindow`].
///
/// Can be obtained from [`OpenWindow::id`] or [`WindowContext::window_id`] or [`WidgetContext::path`].
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum WindowId {
    /// The id for a *real* system window, this is the case for all windows in [headed mode](OpenWindow::mode)
    /// and also for headless windows with renderer enabled in compatibility mode, when a hidden window is used.
    System(SystemWindowId),
    /// The id for a headless window, when the window is not backed by a system window.
    Logical(LogicalWindowId),
}
impl WindowId {
    /// New unique [`Logical`](Self::Logical) window id.
    #[inline]
    pub fn new_unique() -> Self {
        WindowId::Logical(LogicalWindowId::new_unique())
    }
}
impl From<SystemWindowId> for WindowId {
    fn from(id: SystemWindowId) -> Self {
        WindowId::System(id)
    }
}
impl From<LogicalWindowId> for WindowId {
    fn from(id: LogicalWindowId) -> Self {
        WindowId::Logical(id)
    }
}

/// Extension trait, adds [`run_window`](AppRunWindow::run_window) to [`AppExtended`]
pub trait AppRunWindow {
    /// Runs the application event loop and requests a new window.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use zero_ui_core::app::App;
    /// # use zero_ui_core::window::AppRunWindow;
    /// # macro_rules! window { ($($tt:tt)*) => { todo!() } }
    /// App::default().run_window(|_| {
    ///     window! {
    ///         title = "Window 1";
    ///         content = text("Window 1");
    ///     }
    /// })   
    /// ```
    ///
    /// Which is a shortcut for:
    /// ```no_run
    /// # use zero_ui_core::app::App;
    /// # use zero_ui_core::window::{AppRunWindow, Windows};
    /// # macro_rules! window { ($($tt:tt)*) => { todo!() } }
    /// App::default().run(|ctx| {
    ///     ctx.services.req::<Windows>().open(|_| {
    ///         window! {
    ///             title = "Window 1";
    ///             content = text("Window 1");
    ///         }
    ///     });
    /// })   
    /// ```
    fn run_window(self, new_window: impl FnOnce(&mut AppContext) -> Window + 'static);
}

impl<E: AppExtension> AppRunWindow for AppExtended<E> {
    fn run_window(self, new_window: impl FnOnce(&mut AppContext) -> Window + 'static) {
        self.run(|ctx| {
            ctx.services.req::<Windows>().open(new_window);
        })
    }
}

event_args! {
    /// [`WindowOpenEvent`], [`WindowCloseEvent`] args.
    pub struct WindowEventArgs {
        /// Id of window that was opened or closed.
        pub window_id: WindowId,

        ..

        /// If the widget is in the same window.
        fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
            ctx.path.window_id() == self.window_id
        }
    }

    /// [`WindowIsActiveChangedEvent`], [`WindowActivatedEvent`], [`WindowDeactivatedEvent`] args.
    pub struct WindowIsActiveArgs {
        /// Id of window that was opened or closed.
        pub window_id: WindowId,

        /// If the window was activated in this event.
        pub activated: bool,

        ..

        /// If the widget is in the same window.
        fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
            ctx.path.window_id() == self.window_id
        }
    }

    /// [`WindowResizeEvent`] args.
    pub struct WindowResizeArgs {
        /// Window ID.
        pub window_id: WindowId,
        /// New window size.
        pub new_size: LayoutSize,

        ..

        /// If the widget is in the same window.
        fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
            ctx.path.window_id() == self.window_id
        }
    }

    /// [`WindowMoveEvent`] args.
    pub struct WindowMoveArgs {
        /// Window ID.
        pub window_id: WindowId,
        /// New window position.
        pub new_position: LayoutPoint,

        ..

        /// If the widget is in the same window.
        fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
            ctx.path.window_id() == self.window_id
        }
    }

    /// [`WindowScaleChangedEvent`] args.
    pub struct WindowScaleChangedArgs {
        /// Window ID.
        pub window_id: WindowId,
        /// New scale factor.
        pub new_scale_factor: f32,
        /// New window size, given by the OS.
        pub new_size: LayoutSize,

        ..

        /// If the widget is in the same window.
        fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
            ctx.path.window_id() == self.window_id
        }
    }
}
cancelable_event_args! {
    /// [`WindowCloseRequestedEvent`] args.
    pub struct WindowCloseRequestedArgs {
        /// Window ID.
        pub window_id: WindowId,
        group: CloseTogetherGroup,

        ..

        /// If the widget is in the same window.
        fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
            ctx.path.window_id() == self.window_id
        }
    }
}

event_hp! {
    /// Window resized event.
    pub WindowResizeEvent: WindowResizeArgs;

    /// Window moved event.
    pub WindowMoveEvent: WindowMoveArgs;
}

event! {
    /// New window event.
    pub WindowOpenEvent: WindowEventArgs;

    /// Window activated/deactivated event.
    pub WindowIsActiveChangedEvent: WindowIsActiveArgs;

    /// Window activated event.
    pub WindowActivatedEvent: WindowIsActiveArgs;

    /// Window deactivated event.
    pub WindowDeactivatedEvent: WindowIsActiveArgs;

    /// Window scale factor changed.
    pub WindowScaleChangedEvent: WindowScaleChangedArgs;

    /// Closing window event.
    pub WindowCloseRequestedEvent: WindowCloseRequestedArgs;

    /// Close window event.
    pub WindowCloseEvent: WindowEventArgs;
}

/// Application extension that manages windows.
///
/// # Events
///
/// Events this extension provides:
///
/// * [WindowOpenEvent]
/// * [WindowIsActiveChangedEvent]
/// * [WindowActivatedEvent]
/// * [WindowDeactivatedEvent]
/// * [WindowResizeEvent]
/// * [WindowMoveEvent]
/// * [WindowScaleChangedEvent]
/// * [WindowCloseRequestedEvent]
/// * [WindowCloseEvent]
///
/// # Services
///
/// Services this extension provides:
///
/// * [Windows]
pub struct WindowManager {
    event_loop_proxy: Option<EventLoopProxy>,
    ui_threads: Arc<ThreadPool>,
    window_open: EventEmitter<WindowEventArgs>,
    window_is_active_changed: EventEmitter<WindowIsActiveArgs>,
    window_activated: EventEmitter<WindowIsActiveArgs>,
    window_deactivated: EventEmitter<WindowIsActiveArgs>,
    window_resize: EventEmitter<WindowResizeArgs>,
    window_move: EventEmitter<WindowMoveArgs>,
    window_scale_changed: EventEmitter<WindowScaleChangedArgs>,
    window_closing: EventEmitter<WindowCloseRequestedArgs>,
    window_close: EventEmitter<WindowEventArgs>,
}

impl Default for WindowManager {
    fn default() -> Self {
        let ui_threads = Arc::new(
            ThreadPoolBuilder::new()
                .thread_name(|idx| format!("UI#{}", idx))
                .start_handler(|_| {
                    #[cfg(feature = "app_profiler")]
                    crate::profiler::register_thread_with_profiler();
                })
                .build()
                .unwrap(),
        );

        WindowManager {
            event_loop_proxy: None,
            ui_threads,
            window_open: WindowOpenEvent::emitter(),
            window_is_active_changed: WindowIsActiveChangedEvent::emitter(),
            window_activated: WindowActivatedEvent::emitter(),
            window_deactivated: WindowDeactivatedEvent::emitter(),
            window_resize: WindowResizeEvent::emitter(),
            window_move: WindowMoveEvent::emitter(),
            window_scale_changed: WindowScaleChangedEvent::emitter(),
            window_closing: WindowCloseRequestedEvent::emitter(),
            window_close: WindowCloseEvent::emitter(),
        }
    }
}

impl AppExtension for WindowManager {
    fn init(&mut self, r: &mut AppInitContext) {
        self.event_loop_proxy = Some(r.event_loop.clone());
        r.services.register(Windows::new(r.updates.notifier().clone()));
        r.events.register::<WindowOpenEvent>(self.window_open.listener());
        r.events
            .register::<WindowIsActiveChangedEvent>(self.window_is_active_changed.listener());
        r.events.register::<WindowActivatedEvent>(self.window_activated.listener());
        r.events.register::<WindowDeactivatedEvent>(self.window_deactivated.listener());
        r.events.register::<WindowResizeEvent>(self.window_resize.listener());
        r.events.register::<WindowMoveEvent>(self.window_move.listener());
        r.events.register::<WindowScaleChangedEvent>(self.window_scale_changed.listener());
        r.events.register::<WindowCloseRequestedEvent>(self.window_closing.listener());
        r.events.register::<WindowCloseEvent>(self.window_close.listener());
    }

    fn on_window_event(&mut self, window_id: WindowId, event: &WindowEvent, ctx: &mut AppContext) {
        match event {
            WindowEvent::Focused(focused) => {
                if let Some(window) = ctx.services.req::<Windows>().windows.get_mut(&window_id) {
                    window.is_active = *focused;

                    let args = WindowIsActiveArgs::now(window_id, window.is_active);
                    self.window_is_active_changed.notify(ctx.events, args.clone());
                    let specif_event = if window.is_active {
                        &self.window_activated
                    } else {
                        &self.window_deactivated
                    };
                    specif_event.notify(ctx.events, args);
                }
            }
            WindowEvent::Resized(_) => {
                if let Some(window) = ctx.services.req::<Windows>().windows.get_mut(&window_id) {
                    let new_size = window.size();

                    ctx.updates.layout();
                    window.expect_layout_update();
                    window.resize_next_render();

                    // set the window size variable if it is not read-only.
                    let wn_ctx = window.wn_ctx.borrow();
                    if !wn_ctx.root.size.is_read_only(ctx.vars) {
                        let new_size = Size::from((new_size.width, new_size.height));
                        let current_size = *wn_ctx.root.size.get(ctx.vars);
                        // the var can already be set if the user modified it to resize the window.
                        if current_size != new_size {
                            wn_ctx.root.size.set(ctx.vars, new_size).unwrap();
                        }
                    }

                    // raise window_resize
                    self.window_resize.notify(ctx.events, WindowResizeArgs::now(window_id, new_size));
                }
            }
            WindowEvent::Moved(_) => {
                if let Some(window) = ctx.services.req::<Windows>().windows.get_mut(&window_id) {
                    let new_position = window.position();

                    // set the window position variable if it is not read-only.
                    let wn_ctx = window.wn_ctx.borrow();
                    if !wn_ctx.root.position.is_read_only(ctx.vars) {
                        let new_position = Point::from((new_position.x, new_position.y));
                        let var = *wn_ctx.root.position.get(ctx.vars);
                        if new_position != var {
                            let _ = wn_ctx.root.position.set(ctx.vars, new_position);
                        }
                    }

                    // raise window_move
                    self.window_move.notify(ctx.events, WindowMoveArgs::now(window_id, new_position));
                }
            }
            WindowEvent::CloseRequested => {
                let wins = ctx.services.req::<Windows>();
                if wins.windows.contains_key(&window_id) {
                    wins.close_requests.insert(window_id, None);
                    ctx.updates.update();
                }
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                if let Some(window) = ctx.services.req::<Windows>().windows.get_mut(&window_id) {
                    ctx.updates.layout();
                    window.expect_layout_update();

                    self.window_scale_changed.notify(
                        ctx.events,
                        WindowScaleChangedArgs::now(window_id, *scale_factor as f32, window.size()),
                    );
                }
            }
            _ => {}
        }
    }

    fn visit_window_services(&mut self, visitors: &mut WindowServicesVisitors, ctx: &mut AppContext) {
        for window in ctx.services.req::<Windows>().windows() {
            let mut ctx = window.wn_ctx.borrow_mut();
            visitors.visit(ctx.window_id, &mut ctx.services);
        }
    }

    fn update_ui(&mut self, update: UpdateRequest, ctx: &mut AppContext) {
        self.update_open_close(ctx);
        self.update_pump(update, ctx);
        self.update_closing(update, ctx);
        self.update_close(update, ctx);
    }

    fn update_display(&mut self, _: UpdateDisplayRequest, ctx: &mut AppContext) {
        // Pump layout and render in all windows.
        // The windows don't do an update unless they recorded
        // an update request for layout or render.
        for (_, window) in ctx.services.req::<Windows>().windows.iter_mut() {
            window.layout();
            window.render();
            window.render_update();
        }
    }

    fn on_new_frame_ready(&mut self, window_id: WindowId, ctx: &mut AppContext) {
        if let Some(window) = ctx.services.req::<Windows>().windows.get_mut(&window_id) {
            window.request_redraw();
        }
    }

    fn on_redraw_requested(&mut self, window_id: WindowId, ctx: &mut AppContext) {
        if let Some(window) = ctx.services.req::<Windows>().windows.get_mut(&window_id) {
            window.redraw();
        }
    }

    fn on_shutdown_requested(&mut self, args: &ShutdownRequestedArgs, ctx: &mut AppContext) {
        if !args.cancel_requested() {
            let service = ctx.services.req::<Windows>();
            if service.shutdown_on_last_close {
                let windows: Vec<WindowId> = service.windows.keys().copied().collect();
                if !windows.is_empty() {
                    args.cancel();
                    service.close_together(windows).unwrap();
                }
            }
        }
    }

    fn deinit(&mut self, ctx: &mut AppContext) {
        let windows = mem::take(&mut ctx.services.req::<Windows>().windows);
        for (id, window) in windows {
            {
                let mut w_ctx = window.wn_ctx.borrow_mut();
                error_println!("dropping `{:?} ({})` without closing events", id, w_ctx.root.title.get_local());
                w_ctx.deinit(ctx);
            }
            window.deinit();
        }
    }
}

impl WindowManager {
    /// Respond to open/close requests.
    fn update_open_close(&mut self, ctx: &mut AppContext) {
        // respond to service requests
        let (open, close) = ctx.services.req::<Windows>().take_requests();

        for request in open {
            let w = OpenWindow::new(
                request.new,
                ctx,
                ctx.event_loop,
                self.event_loop_proxy.as_ref().unwrap().clone(),
                Arc::clone(&self.ui_threads),
            );

            let args = WindowEventArgs::now(w.id());

            let wn_ctx = w.wn_ctx.clone();
            let mut wn_ctx = wn_ctx.borrow_mut();
            ctx.services.req::<Windows>().windows.insert(args.window_id, w);
            wn_ctx.init(ctx);

            // notify the window requester
            request.notifier.notify(ctx.events, args.clone());

            // notify everyone
            self.window_open.notify(ctx.events, args.clone());
        }

        for (window_id, group) in close {
            self.window_closing
                .notify(ctx.events, WindowCloseRequestedArgs::now(window_id, group));
        }
    }

    /// Pump the requested update methods.
    fn update_pump(&mut self, update: UpdateRequest, ctx: &mut AppContext) {
        if update.update_hp || update.update {
            // detach context part so we can let a window content access its own window.
            let mut wn_ctxs: Vec<_> = ctx
                .services
                .req::<Windows>()
                .windows
                .iter_mut()
                .map(|(_, w)| w.wn_ctx.clone())
                .collect();

            // high-pressure pump.
            if update.update_hp {
                for wn_ctx in wn_ctxs.iter_mut() {
                    wn_ctx.borrow_mut().update_hp(ctx);
                }
            }

            // low-pressure pump.
            if update.update {
                for wn_ctx in wn_ctxs.iter_mut() {
                    wn_ctx.borrow_mut().update(ctx);
                }
            }

            // do window vars update.
            if update.update {
                for (_, window) in ctx.services.req::<Windows>().windows.iter_mut() {
                    window.update_window_vars(ctx.vars, ctx.updates);
                }
            }
        }
    }

    /// Respond to window_closing events.
    fn update_closing(&mut self, update: UpdateRequest, ctx: &mut AppContext) {
        if !update.update {
            return;
        }

        // close_together are canceled together
        let canceled_groups: Vec<_> = self
            .window_closing
            .updates(ctx.events)
            .iter()
            .filter_map(|c| {
                if c.cancel_requested() && c.group.is_some() {
                    Some(c.group)
                } else {
                    None
                }
            })
            .collect();

        let service = ctx.services.req::<Windows>();

        for closing in self.window_closing.updates(ctx.events) {
            if !closing.cancel_requested() && !canceled_groups.contains(&closing.group) {
                // not canceled and we can close the window.
                // notify close, the window will be deinit on
                // the next update.
                self.window_close.notify(ctx.events, WindowEventArgs::now(closing.window_id));

                for listener in service.close_listeners.remove(&closing.window_id).unwrap_or_default() {
                    listener.notify(ctx.events, CloseWindowResult::Close);
                }
            } else {
                // canceled notify operation listeners.

                for listener in service.close_listeners.remove(&closing.window_id).unwrap_or_default() {
                    listener.notify(ctx.events, CloseWindowResult::Cancel);
                }
            }
        }
    }

    /// Respond to window_close events.
    fn update_close(&mut self, update: UpdateRequest, ctx: &mut AppContext) {
        if !update.update {
            return;
        }

        for close in self.window_close.updates(ctx.events) {
            if let Some(w) = ctx.services.req::<Windows>().windows.remove(&close.window_id) {
                w.wn_ctx.clone().borrow_mut().deinit(ctx);
                w.deinit();
            }
        }

        let service = ctx.services.req::<Windows>();
        if service.shutdown_on_last_close && service.windows.is_empty() {
            ctx.services.req::<AppProcess>().shutdown();
        }
    }
}

/// Windows service.
#[derive(AppService)]
pub struct Windows {
    /// If shutdown is requested when there are no more windows open, `true` by default.
    pub shutdown_on_last_close: bool,

    windows: FnvHashMap<WindowId, OpenWindow>,

    open_requests: Vec<OpenWindowRequest>,
    close_requests: FnvHashMap<WindowId, CloseTogetherGroup>,
    next_group: u16,
    close_listeners: FnvHashMap<WindowId, Vec<EventEmitter<CloseWindowResult>>>,
    update_notifier: UpdateNotifier,
}

impl Windows {
    fn new(update_notifier: UpdateNotifier) -> Self {
        Windows {
            shutdown_on_last_close: true,
            open_requests: Vec::with_capacity(1),
            close_requests: FnvHashMap::default(),
            close_listeners: FnvHashMap::default(),
            next_group: 1,
            windows: FnvHashMap::default(),
            update_notifier,
        }
    }

    /// Requests a new window. Returns a listener that will update once when the window is opened.
    pub fn open(&mut self, new_window: impl FnOnce(&mut AppContext) -> Window + 'static) -> EventListener<WindowEventArgs> {
        let request = OpenWindowRequest {
            new: Box::new(new_window),
            notifier: EventEmitter::response(),
        };
        let notice = request.notifier.listener();
        self.open_requests.push(request);

        self.update_notifier.update();

        notice
    }

    /// Starts closing a window, the operation can be canceled by listeners of the
    /// [close requested event](WindowCloseRequestedEvent).
    ///
    /// Returns a listener that will update once with the result of the operation.
    pub fn close(&mut self, window_id: WindowId) -> Result<EventListener<CloseWindowResult>, WindowNotFound> {
        if self.windows.contains_key(&window_id) {
            let notifier = EventEmitter::response();
            let notice = notifier.listener();
            self.insert_close(window_id, None, notifier);
            self.update_notifier.update();
            Ok(notice)
        } else {
            Err(WindowNotFound(window_id))
        }
    }

    /// Requests closing multiple windows together, the operation can be canceled by listeners of the
    /// [close requested event](WindowCloseRequestedEvent). If canceled none of the windows are closed.
    ///
    /// Returns a listener that will update once with the result of the operation.
    pub fn close_together(
        &mut self,
        windows: impl IntoIterator<Item = WindowId>,
    ) -> Result<EventListener<CloseWindowResult>, WindowNotFound> {
        let windows = windows.into_iter();
        let mut buffer = Vec::with_capacity(windows.size_hint().0);
        {
            for id in windows {
                if !self.windows.contains_key(&id) {
                    return Err(WindowNotFound(id));
                }
                buffer.push(id);
            }
        }

        let set_id = NonZeroU16::new(self.next_group).unwrap();
        self.next_group += 1;

        let notifier = EventEmitter::response();

        for id in buffer {
            self.insert_close(id, Some(set_id), notifier.clone());
        }

        self.update_notifier.update();

        Ok(notifier.into_listener())
    }

    fn insert_close(&mut self, window_id: WindowId, set: CloseTogetherGroup, notifier: EventEmitter<CloseWindowResult>) {
        self.close_requests.insert(window_id, set);
        let listeners = self.close_listeners.entry(window_id).or_insert_with(Vec::new);
        listeners.push(notifier)
    }

    /// Reference an open window.
    #[inline]
    pub fn window(&self, window_id: WindowId) -> Result<&OpenWindow, WindowNotFound> {
        self.windows.get(&window_id).ok_or(WindowNotFound(window_id))
    }

    /// Iterate over all open windows.
    #[inline]
    pub fn windows(&self) -> impl Iterator<Item = &OpenWindow> {
        self.windows.values()
    }

    fn take_requests(&mut self) -> (Vec<OpenWindowRequest>, FnvHashMap<WindowId, CloseTogetherGroup>) {
        (mem::take(&mut self.open_requests), mem::take(&mut self.close_requests))
    }
}

struct OpenWindowRequest {
    new: Box<dyn FnOnce(&mut AppContext) -> Window>,
    notifier: EventEmitter<WindowEventArgs>,
}

/// Response message of [`close`](Windows::close) and [`close_together`](Windows::close_together).
#[derive(Debug, Eq, PartialEq)]
pub enum CloseWindowResult {
    /// Operation completed, all requested windows closed.
    Close,

    /// Operation canceled, no window closed.
    Cancel,
}

/// Window not found error.
#[derive(Debug)]
pub struct WindowNotFound(pub WindowId);
impl std::fmt::Display for WindowNotFound {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "window `{:?}` is not opened in `Windows` service", self.0)
    }
}
impl std::error::Error for WindowNotFound {}

/// Window configuration.
pub struct Window {
    meta: LazyStateMap,
    id: WidgetId,
    title: BoxedLocalVar<Text>,
    start_position: StartPosition,
    position: BoxedVar<Point>,
    size: BoxedVar<Size>,
    auto_size: BoxedLocalVar<AutoSize>,
    resizable: BoxedVar<bool>,
    clear_color: BoxedLocalVar<Rgba>,
    visible: BoxedLocalVar<bool>,
    headless_config: WindowHeadlessConfig,
    child: Box<dyn UiNode>,
}
impl Window {
    /// New window configuration.
    ///
    /// * `root_id` - Widget ID of `child`.
    /// * `title` - Window title, in the title-bar.
    /// * `start_position` - Position of the window when it first opens.
    /// * `position` - Position of the window, can be updated back by the window.
    /// * `size` - Size of the window, can be updated back by the window.
    /// * `auto_size` - If the window will auto-size to fit the `child`.
    /// * `resizable` - If the user can resize the window.
    /// * `clear_color` - Color used to clear a frame, works like a background color applied before `child`.
    /// * `visible` - If the window is visible, TODO diff. minimized.
    /// * `headless_config` - Extra config for the window when run in [headless mode](WindowMode::is_headless).
    /// * `child` - The root widget outermost node, the window sets-up the root widget using this and the `root_id`.
    #[allow(clippy::clippy::too_many_arguments)]
    pub fn new(
        root_id: WidgetId,
        title: impl IntoVar<Text>,
        start_position: impl Into<StartPosition>,
        position: impl IntoVar<Point>,
        size: impl IntoVar<Size>,
        auto_size: impl IntoVar<AutoSize>,
        resizable: impl IntoVar<bool>,
        clear_color: impl IntoVar<Rgba>,
        visible: impl IntoVar<bool>,
        headless_config: WindowHeadlessConfig,
        child: impl UiNode,
    ) -> Self {
        Window {
            meta: LazyStateMap::default(),
            id: root_id,
            title: title.into_local().boxed_local(),
            start_position: start_position.into(),
            position: position.into_var().boxed(),
            size: size.into_var().boxed(),
            auto_size: auto_size.into_local().boxed_local(),
            resizable: resizable.into_var().boxed(),
            clear_color: clear_color.into_local().boxed_local(),
            visible: visible.into_local().boxed_local(),
            headless_config,
            child: child.boxed(),
        }
    }
}

/// Configuration of a window in [headless mode](WindowMode::is_headless).
#[derive(Debug, Clone)]
pub struct WindowHeadlessConfig {
    /// The scale factor used for the headless layout and rendering.
    ///
    /// `1.0` by default.
    pub scale_factor: f32,

    /// Size of the imaginary monitor screen that contains the headless window.
    ///
    /// This is used to calculate relative lengths in the window size definition.
    ///
    /// `(1920.0, 1080.0)` by default.
    pub screen_size: LayoutSize,
}
impl Default for WindowHeadlessConfig {
    fn default() -> Self {
        WindowHeadlessConfig {
            scale_factor: 1.0,
            screen_size: LayoutSize::new(1920.0, 1080.0),
        }
    }
}

bitflags! {
    /// Window auto-size config.
    pub struct AutoSize: u8 {
        /// Does not automatically adjust size.
        const DISABLED = 0;
        /// Uses the content desired width.
        const CONTENT_WIDTH = 0b01;
        /// Uses the content desired height.
        const CONTENT_HEIGHT = 0b10;
        /// Uses the content desired width and height.
        const CONTENT = Self::CONTENT_WIDTH.bits | Self::CONTENT_HEIGHT.bits;
    }
}
impl_from_and_into_var! {
    /// Returns [`AutoSize::CONTENT`] if `content` is `true`, otherwise
    // returns [`AutoSize::DISABLED`].
    fn from(content: bool) -> AutoSize {
        if content {
            AutoSize::CONTENT
        } else {
            AutoSize::DISABLED
        }
    }
}

/// Window startup position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartPosition {
    /// Uses the value of the `position` property.
    Default,
    /// Centralizes the window in relation to the active screen.
    CenterScreen,
    /// Centralizes the window in relation to the parent window.
    CenterOwner,
}
impl Default for StartPosition {
    fn default() -> Self {
        Self::Default
    }
}

/// An open window.
pub struct OpenWindow {
    gl_ctx: RefCell<GlContext>,
    wn_ctx: Rc<RefCell<OwnedWindowContext>>,

    // Copy for when `wn_ctx` is borrowed.
    mode: WindowMode,
    id: WindowId,

    renderer: RendererState,
    pipeline_id: PipelineId,
    document_id: DocumentId,

    first_draw: bool,
    frame_info: FrameInfo,

    // document area visible in this window.
    doc_view: units::DeviceIntRect,
    // if [doc_view] changed and no render was called yet.
    doc_view_changed: bool,

    is_active: bool,

    #[cfg(windows)]
    subclass_id: std::cell::Cell<usize>,

    headless_config: WindowHeadlessConfig,
    headless_position: LayoutPoint,
    headless_size: LayoutSize,

    // used in renderless mode to notify new frames.
    renderless_event_sender: Option<EventLoopProxy>,
}

/// Mode of an [`OpenWindow`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowMode {
    /// Normal mode, shows a system window with content rendered.
    Headed,

    /// Headless mode, no system window and no renderer. The window does layout and calls [`UiNode::render`] but
    /// it does not actually generates frame textures.
    Headless,
    /// Headless mode, no visible system window but with a renderer. The window does everything a [`Headed`](WindowMode::Headed)
    /// window does, except presenting frame textures in a system window.
    HeadlessWithRenderer,
}
impl WindowMode {
    /// If is the [`Headed`](WindowMode::Headed) mode.
    #[inline]
    pub fn is_headed(self) -> bool {
        match self {
            WindowMode::Headed => true,
            WindowMode::Headless | WindowMode::HeadlessWithRenderer => false,
        }
    }

    /// If is the [`Headless`](WindowMode::Headed) or [`HeadlessWithRenderer`](WindowMode::Headed) modes.
    #[inline]
    pub fn is_headless(self) -> bool {
        match self {
            WindowMode::Headless | WindowMode::HeadlessWithRenderer => true,
            WindowMode::Headed => false,
        }
    }

    /// If is the [`Headed`](WindowMode::Headed) or [`HeadlessWithRenderer`](WindowMode::HeadlessWithRenderer) modes.
    #[inline]
    pub fn has_renderer(self) -> bool {
        match self {
            WindowMode::Headed | WindowMode::HeadlessWithRenderer => true,
            WindowMode::Headless => false,
        }
    }
}

impl OpenWindow {
    fn new(
        new_window: Box<dyn FnOnce(&mut AppContext) -> Window>,
        ctx: &mut AppContext,
        event_loop: EventLoopWindowTarget,
        event_loop_proxy: EventLoopProxy,
        ui_threads: Arc<ThreadPool>,
    ) -> Self {
        let root = new_window(ctx);

        // figure-out mode.
        let mode = if let Some(headless) = ctx.headless.state() {
            if headless.get(app::HeadlessRendererEnabledKey).copied().unwrap_or_default() {
                WindowMode::HeadlessWithRenderer
            } else {
                WindowMode::Headless
            }
        } else {
            WindowMode::Headed
        };

        // Init OpenGL context.
        let mut gl_ctx = match mode {
            WindowMode::Headed => {
                let window_builder = WindowBuilder::new()
                    .with_visible(false) // not visible until first render, to avoid flickering
                    .with_resizable(*root.resizable.get(ctx.vars))
                    .with_title(root.title.get(ctx.vars).to_owned())
                    .with_resizable(*root.auto_size.get(ctx.vars) != AutoSize::CONTENT);

                GlContext::new(window_builder, event_loop.headed_target().unwrap())
            }
            WindowMode::HeadlessWithRenderer => GlContext::new_headless(),
            WindowMode::Headless => GlContext::new_renderless(),
        };

        // set the user initial position, size & dpi.

        // dpi will be taken from monitor or headless config.
        let dpi_factor;

        // if the position and size property variables need to be set back during initialization.
        // this is the case when they are read-write and did not define the initial window size/pos.
        let set_position_var;
        let set_size_var;

        // initial size for WebRender.
        let device_init_size;

        // initial headless position and size.
        let headless_position;
        let headless_size;

        if mode.is_headed() {
            // init position, size & dpi in headed mode.

            // the `available_size` used to calculate relative lengths in the user pos/size values.
            // is the monitor size or (800, 600) fallback if there is not monitor plugged.
            let available_size;
            if let Some(monitor) = gl_ctx.window().current_monitor() {
                let size = monitor.size();
                let scale = monitor.scale_factor() as f32;
                available_size = LayoutSize::new(size.width as f32 * scale, size.height as f32 * scale);
                dpi_factor = scale;
            } else {
                #[cfg(debug_assertions)]
                warn_println!("no monitor found");
                available_size = LayoutSize::new(800.0, 600.0);
                dpi_factor = 1.0;
            };

            // the init position and size selected by the operating system.
            let system_init_pos = gl_ctx.window().outer_position().expect("only desktop windows are implemented");
            let system_init_size = gl_ctx.window().inner_size();

            // layout context for the user pos/size values.
            let layout_ctx = LayoutContext::new(12.0, available_size, PixelGrid::new(dpi_factor));

            // calculate the user pos/size in physical units.
            let user_init_pos = root.position.get(ctx.vars).to_layout(available_size, &layout_ctx);
            let user_init_size = root.size.get(ctx.vars).to_layout(available_size, &layout_ctx);
            // the use can set the variable but not the value (LAYOUT_ANY_SIZE)
            // in this case se fallback to the system selected value.
            let mut pos_used_fallback = false;
            let valid_init_pos = glutin::dpi::PhysicalPosition::new(
                if user_init_pos.x.is_finite() {
                    (user_init_pos.x * dpi_factor) as i32
                } else {
                    pos_used_fallback = true;
                    system_init_pos.x
                },
                if user_init_pos.y.is_finite() {
                    (user_init_pos.y * dpi_factor) as i32
                } else {
                    pos_used_fallback = true;
                    system_init_pos.y
                },
            );
            let mut size_used_fallback = false;
            let valid_init_size = glutin::dpi::PhysicalSize::new(
                if user_init_size.width.is_finite() {
                    (user_init_size.width * dpi_factor) as u32
                } else {
                    size_used_fallback = true;
                    system_init_size.width
                },
                if user_init_size.height.is_finite() {
                    (user_init_size.height * dpi_factor) as u32
                } else {
                    size_used_fallback = true;
                    system_init_size.height
                },
            );

            device_init_size = units::DeviceIntSize::new(valid_init_size.width as i32, valid_init_size.height as i32);

            // propagate pos & size
            //
            // we need to set the system size if all/some of the user values where valid ..
            if valid_init_pos != system_init_pos {
                gl_ctx.window().set_outer_position(valid_init_pos);
            }
            if valid_init_size != system_init_size {
                gl_ctx.window().set_inner_size(valid_init_size);
            }
            // .. and we need to set the user variables they are read/write and all/some of
            // the system values where used.
            set_position_var = pos_used_fallback && !root.position.is_read_only(ctx.vars);
            set_size_var = size_used_fallback && !root.position.is_read_only(ctx.vars);

            headless_position = LayoutPoint::zero();
            headless_size = LayoutSize::zero();
        } else {
            // init position, size & dpi in headless mode.

            dpi_factor = root.headless_config.scale_factor;
            let available_size = root.headless_config.screen_size;

            // values used for when the user size/pos values are LAYOUT_ANY_SIZE
            let fallback_pos = LayoutPoint::zero();
            let fallback_size = available_size;

            // layout context for the user pos/size values.
            let layout_ctx = LayoutContext::new(12.0, available_size, PixelGrid::new(dpi_factor));

            // calculate the user pos/size in physical units.
            let user_init_pos = root.position.get(ctx.vars).to_layout(available_size, &layout_ctx);
            let user_init_size = root.size.get(ctx.vars).to_layout(available_size, &layout_ctx);

            let mut used_fallback = false;

            let valid_pos = LayoutPoint::new(
                if user_init_pos.x.is_finite() {
                    user_init_pos.x
                } else {
                    used_fallback = true;
                    fallback_pos.x
                },
                if user_init_pos.y.is_finite() {
                    user_init_pos.y
                } else {
                    used_fallback = true;
                    fallback_pos.y
                },
            );
            headless_position = valid_pos;

            set_position_var = used_fallback && !root.position.is_read_only(ctx.vars);
            used_fallback = false;

            let valid_size = LayoutSize::new(
                if user_init_size.width.is_finite() {
                    user_init_size.width
                } else {
                    used_fallback = true;
                    fallback_size.width
                },
                if user_init_size.height.is_finite() {
                    user_init_size.height
                } else {
                    used_fallback = true;
                    fallback_size.height
                },
            );
            headless_size = valid_size;

            set_size_var = used_fallback && !root.size.is_read_only(ctx.vars);

            device_init_size = units::DeviceIntSize::new((valid_size.width * dpi_factor) as i32, (valid_size.height * dpi_factor) as i32);
        }

        let window_id = if mode.is_headed() {
            WindowId::System(gl_ctx.window().id())
        } else {
            WindowId::new_unique()
        };

        // initialize renderer.
        let api;
        let document_id;
        let pipeline_id;
        let renderer;
        let renderless_event_sender;

        if mode.has_renderer() {
            let clear_color = *root.clear_color.get(ctx.vars);
            let opts = webrender::RendererOptions {
                device_pixel_ratio: dpi_factor,
                clear_color: Some(clear_color.into()),
                workers: Some(ui_threads),
                // TODO expose more options to the user.
                ..webrender::RendererOptions::default()
            };

            renderless_event_sender = None;

            let notifier = Box::new(Notifier {
                window_id,
                event_loop: event_loop_proxy,
            });

            let (renderer_, sender) = webrender::Renderer::new(gl_ctx.gl().clone(), notifier, opts, None, device_init_size).unwrap();
            let api_ = Arc::new(sender.create_api());
            document_id = api_.add_document(device_init_size, 0);

            api = Some(api_);
            renderer = RendererState::Running(renderer_);
            pipeline_id = PipelineId(1, 0);
        } else {
            document_id = DocumentId::INVALID;
            api = None;
            renderer = RendererState::Renderless;
            pipeline_id = PipelineId::dummy();

            renderless_event_sender = Some(event_loop_proxy);
        }

        let (state, services) = ctx.new_window(window_id, mode, &api);

        if mode.has_renderer() {
            gl_ctx.make_not_current();
        }

        let frame_info = FrameInfo::blank(window_id, root.id);

        let w = OpenWindow {
            gl_ctx: RefCell::new(gl_ctx),

            headless_config: root.headless_config.clone(),

            wn_ctx: Rc::new(RefCell::new(OwnedWindowContext {
                mode,
                api,
                root,
                state,
                services,
                window_id,
                root_transform_key: WidgetTransformKey::new_unique(),
                update: UpdateDisplayRequest::Layout,
            })),

            mode,
            id: window_id,

            renderer,
            document_id,
            pipeline_id,

            first_draw: true,
            frame_info,

            doc_view: units::DeviceIntRect::from_size(device_init_size),
            doc_view_changed: false,
            is_active: true, // just opened it?

            headless_position,
            headless_size,

            renderless_event_sender,

            #[cfg(windows)]
            subclass_id: std::cell::Cell::new(0),
        };

        if set_position_var {
            // user did not set position, but variable is read-write,
            // so we update with the OS provided initial position.
            let LayoutPoint { x, y, .. } = w.position();
            w.wn_ctx.borrow().root.position.set(ctx.vars, (x, y).into()).unwrap();
        }
        if set_size_var {
            // user did not set size, but variable is read-write,
            // so we update with the OS provided initial size.
            let LayoutSize { width, height, .. } = w.size();
            w.wn_ctx.borrow().root.size.set(ctx.vars, (width, height).into()).unwrap();
        }

        w
    }

    /// Window mode.
    #[inline]
    pub fn mode(&self) -> WindowMode {
        self.mode
    }

    /// Window ID.
    #[inline]
    pub fn id(&self) -> WindowId {
        self.id
    }

    /// If the window is the foreground window.
    #[inline]
    pub fn is_active(&self) -> bool {
        self.is_active
    }

    /// Position of the window.
    #[inline]
    pub fn position(&self) -> LayoutPoint {
        if self.mode().is_headed() {
            let gl_ctx = self.gl_ctx.borrow();
            let wn = gl_ctx.window();
            let s = wn.scale_factor() as f32;
            let pos = wn.outer_position().expect("only desktop windows are supported");
            LayoutPoint::new(pos.x as f32 / s, pos.y as f32 / s)
        } else {
            self.headless_position
        }
    }

    /// Size of the window content.
    #[inline]
    pub fn size(&self) -> LayoutSize {
        if self.mode().is_headed() {
            let gl_ctx = self.gl_ctx.borrow();
            let wn = gl_ctx.window();
            let s = wn.scale_factor() as f32;
            let p_size = wn.inner_size();
            LayoutSize::new(p_size.width as f32 / s, p_size.height as f32 / s)
        } else {
            self.headless_size
        }
    }

    /// Scale factor used by this window, all `Layout*` values are scaled by this value by the renderer.
    #[inline]
    pub fn scale_factor(&self) -> f32 {
        if self.mode().is_headed() {
            self.gl_ctx.borrow().window().scale_factor() as f32
        } else {
            self.headless_config.scale_factor
        }
    }

    /// Pixel grid of this window, all `Layout*` values are aligned with this grid during layout.
    #[inline]
    pub fn pixel_grid(&self) -> PixelGrid {
        PixelGrid::new(self.scale_factor())
    }

    /// Hit-test the latest frame.
    ///
    /// # Renderless
    ///
    /// Hit-testing needs a renderer for pixel accurate results. In [renderless mode](Self::mode) a fallback
    /// layout based hit-testing algorithm is used, it probably generates different results.
    #[inline]
    pub fn hit_test(&self, point: LayoutPoint) -> FrameHitInfo {
        if let Some(api) = &self.wn_ctx.borrow().api {
            let r = api.hit_test(
                self.document_id,
                Some(self.pipeline_id),
                units::WorldPoint::new(point.x, point.y),
                HitTestFlags::all(),
            );

            FrameHitInfo::new(self.id(), self.frame_info.frame_id(), point, r)
        } else {
            unimplemented!("hit-test fallback for renderless mode not implemented");
        }
    }

    /// Latest frame info.
    pub fn frame_info(&self) -> &FrameInfo {
        &self.frame_info
    }

    /// Take a screenshot of the full window area.
    ///
    /// # Panics
    ///
    /// Panics if running in [renderless mode](Self::mode).
    pub fn screenshot(&self) -> ScreenshotData {
        self.screenshot_rect(LayoutRect::from_size(self.size()))
    }

    /// Take a screenshot of a window area.
    ///
    /// # Panics
    ///
    /// Panics if running in [renderless mode](Self::mode).
    pub fn screenshot_rect(&self, rect: LayoutRect) -> ScreenshotData {
        let mut gl_ctx = self.gl_ctx.borrow_mut();

        // calculate intersection with window in physical pixels.
        let (x, y, width, height, dpi) = {
            let dpi;
            let max_rect;
            let final_rect;

            if let Some(window) = gl_ctx.window_opt() {
                // headed mode.
                dpi = window.scale_factor() as f32;
                let max_size = window.inner_size();
                max_rect = LayoutRect::from_size(LayoutSize::new(max_size.width as f32, max_size.height as f32));
                let rect = rect * dpi;
                final_rect = rect.intersection(&max_rect).unwrap_or_default();
            } else {
                // headless mode.
                dpi = self.headless_config.scale_factor;
                let max_size = self.headless_size;
                max_rect = LayoutRect::from_size(max_size * dpi);
                let rect = rect * dpi;
                final_rect = rect.intersection(&max_rect).unwrap_or_default();
            }
            (
                final_rect.origin.x as u32,
                // read_pixels (0, 0) is the lower left corner.
                (max_rect.size.height - final_rect.origin.y - final_rect.size.height).max(0.0) as u32,
                final_rect.size.width as u32,
                final_rect.size.height as u32,
                dpi,
            )
        };

        if width == 0 || height == 0 {
            return ScreenshotData {
                pixels: vec![],
                width,
                height,
                dpi,
            };
        }

        gl_ctx.make_current();
        gl_ctx.swap_buffers();
        let gl = gl_ctx.gl();
        let pixels = gl.read_pixels(x as _, y as _, width as _, height as _, gl::RGB, gl::UNSIGNED_BYTE);

        gl_ctx.swap_buffers();

        let error = gl.get_error();
        if error != gl::NO_ERROR {
            panic!("read_pixels error: {:#x}", error)
        }
        gl_ctx.make_not_current();

        let mut pixels_flipped = Vec::with_capacity(pixels.len());
        for v in (0..height as _).rev() {
            let s = 3 * v as usize * width as usize;
            let o = 3 * width as usize;
            pixels_flipped.extend_from_slice(&pixels[s..(s + o)]);
        }

        ScreenshotData {
            pixels: pixels_flipped,
            width,
            height,
            dpi,
        }
    }

    /// Manually flags layout to actually update on the next call.
    ///
    /// This is required for updates generated outside of this window but that affect this window.
    fn expect_layout_update(&mut self) {
        self.wn_ctx.borrow_mut().update |= UpdateDisplayRequest::Layout;
    }

    fn update_window_vars(&mut self, vars: &Vars, updates: &mut Updates) {
        profile_scope!("window::update_window_vars");

        let gl_ctx = self.gl_ctx.borrow();
        let mut wn_ctx = self.wn_ctx.borrow_mut();

        if let Some(window) = gl_ctx.window_opt() {
            // headed update.

            // title
            if let Some(title) = wn_ctx.root.title.update_local(vars) {
                window.set_title(title);
            }

            // position
            if let Some(&new_pos) = wn_ctx.root.position.get_new(vars) {
                let current_pos = window.outer_position().expect("only desktop windows are supported");

                let layout_ctx = self.monitor_layout_ctx();
                let dpi_factor = layout_ctx.pixel_grid().scale_factor;
                let new_pos = new_pos.to_layout(layout_ctx.viewport_size(), &layout_ctx);

                let valid_pos = glutin::dpi::PhysicalPosition::new(
                    if new_pos.x.is_finite() {
                        (new_pos.x * dpi_factor) as i32
                    } else {
                        current_pos.x
                    },
                    if new_pos.y.is_finite() {
                        (new_pos.y * dpi_factor) as i32
                    } else {
                        current_pos.y
                    },
                );

                if valid_pos != current_pos {
                    // the position variable was changed to set the position size.
                    window.set_outer_position(valid_pos);
                }
            }

            // auto-size
            if wn_ctx.root.auto_size.update_local(vars).is_some() {
                updates.layout();
            }

            // size
            if let Some(&new_size) = wn_ctx.root.size.get_new(vars) {
                let current_size = window.inner_size();

                let layout_ctx = self.monitor_layout_ctx();
                let dpi_factor = layout_ctx.pixel_grid().scale_factor;
                let new_size = new_size.to_layout(layout_ctx.viewport_size(), &layout_ctx);

                let auto_size = *wn_ctx.root.auto_size.get_local();

                let valid_size = glutin::dpi::PhysicalSize::new(
                    if !auto_size.contains(AutoSize::CONTENT_WIDTH) && new_size.width.is_finite() {
                        (new_size.width * dpi_factor) as u32
                    } else {
                        current_size.width
                    },
                    if !auto_size.contains(AutoSize::CONTENT_HEIGHT) && new_size.height.is_finite() {
                        (new_size.height * dpi_factor) as u32
                    } else {
                        current_size.height
                    },
                );

                if auto_size == AutoSize::CONTENT {
                    window.set_resizable(false);
                } else {
                    // TODO disable resize in single dimension?
                    window.set_resizable(true);
                }

                if valid_size != current_size {
                    // the size var was changed to set the position size.
                    window.set_inner_size(valid_size);
                }
            }

            // resizable
            if let Some(&resizable) = wn_ctx.root.resizable.get_new(vars) {
                window.set_resizable(resizable);
            }

            // background_color
            if wn_ctx.root.clear_color.update_local(vars).is_some() {
                wn_ctx.update |= UpdateDisplayRequest::Render;
                updates.render();
            }

            // visibility
            if let Some(&vis) = wn_ctx.root.visible.update_local(vars) {
                if !self.first_draw {
                    window.set_visible(vis);
                    if vis {
                        updates.layout();
                    }
                }
            }
        } else {
            // headless update.

            wn_ctx.root.title.update_local(vars);
            wn_ctx.root.auto_size.update_local(vars);
            wn_ctx.root.clear_color.update_local(vars);

            let available_size = self.headless_config.screen_size;
            let layout_ctx = LayoutContext::new(14.0, available_size, PixelGrid::new(wn_ctx.root.headless_config.scale_factor));

            if let Some(size) = wn_ctx.root.size.get_new(vars) {
                self.headless_size = size.to_layout(available_size, &layout_ctx);
            }
            if let Some(pos) = wn_ctx.root.position.get_new(vars) {
                self.headless_position = pos.to_layout(available_size, &layout_ctx);
            }

            if let Some(&vis) = wn_ctx.root.visible.update_local(vars) {
                if !self.first_draw && vis {
                    updates.layout();
                }
            }
        }
    }

    fn monitor_layout_ctx(&self) -> LayoutContext {
        let monitor = self
            .gl_ctx
            .borrow()
            .window()
            .current_monitor()
            .expect("did not find current monitor");
        let size = monitor.size();
        let scale = monitor.scale_factor() as f32;
        let size = LayoutSize::new(size.width as f32 * scale, size.height as f32 * scale);
        // TODO font size
        LayoutContext::new(14.0, size, PixelGrid::new(scale))
    }

    fn layout_ctx(&self) -> LayoutContext {
        // TODO font size
        LayoutContext::new(14.0, self.size(), PixelGrid::new(self.scale_factor()))
    }

    /// Re-flow layout if a layout pass was required. If yes will
    /// flag a render required.
    fn layout(&mut self) {
        let mut ctx = self.wn_ctx.borrow_mut();

        if ctx.update == UpdateDisplayRequest::Layout {
            profile_scope!("window::layout");

            ctx.update = UpdateDisplayRequest::Render;

            let mut layout_ctx = self.layout_ctx();

            let mut available_size = ctx.root.child.measure(layout_ctx.viewport_size(), &mut layout_ctx);

            let auto_size = *ctx.root.auto_size.get_local();
            if !auto_size.contains(AutoSize::CONTENT_WIDTH) {
                available_size.width = layout_ctx.viewport_size().width;
            }
            if !auto_size.contains(AutoSize::CONTENT_HEIGHT) {
                available_size.height = layout_ctx.viewport_size().height;
            }

            ctx.root.child.arrange(available_size, &mut layout_ctx);

            if auto_size != AutoSize::DISABLED {
                let factor = layout_ctx.pixel_grid().scale_factor;
                let size = glutin::dpi::PhysicalSize::new((available_size.width * factor) as u32, (available_size.height * factor) as u32);
                self.gl_ctx.borrow().window().set_inner_size(size);
            }
        }
    }

    fn resize_next_render(&mut self) {
        let inner_size = self.gl_ctx.borrow().window().inner_size();
        let device_size = units::DeviceIntSize::new(inner_size.width as i32, inner_size.height as i32);
        self.doc_view = units::DeviceIntRect::from_size(device_size);
        self.doc_view_changed = true;
    }

    /// Render a frame if one was required.
    fn render(&mut self) {
        let mut ctx = self.wn_ctx.borrow_mut();

        if ctx.update == UpdateDisplayRequest::Render {
            profile_scope!("window::render");

            ctx.update = UpdateDisplayRequest::None;

            let frame_id = Epoch({
                let mut next = self.frame_info.frame_id().0.wrapping_add(1);
                if next == FrameId::invalid().0 {
                    next = next.wrapping_add(1);
                }
                next
            });

            let size = self.size();
            let clear_color = (*ctx.root.clear_color.get_local()).into();
            let mut frame = FrameBuilder::new(
                frame_id,
                ctx.window_id,
                self.pipeline_id,
                ctx.api.clone(),
                ctx.root.id,
                ctx.root_transform_key,
                size,
                self.scale_factor(),
                clear_color,
            );

            ctx.root.child.render(&mut frame);

            let (display_list_data, frame_info) = frame.finalize();

            self.frame_info = frame_info;

            if let Some(api) = &ctx.api {
                // send request if has a renderer.
                let mut txn = Transaction::new();
                txn.set_display_list(frame_id, Some(clear_color), size, display_list_data, true);
                txn.set_root_pipeline(self.pipeline_id);

                if self.doc_view_changed {
                    self.doc_view_changed = false;
                    txn.set_document_view(self.doc_view, self.scale_factor());
                }

                txn.generate_frame();
                api.send_transaction(self.document_id, txn);
            } else {
                // in renderless mode the frame is ready already.
                self.renderless_event_sender
                    .as_ref()
                    .unwrap()
                    .send_event(AppEvent::NewFrameReady(self.id));
            }
        }
    }

    /// Render a frame update if one was required.
    fn render_update(&mut self) {
        let mut ctx = self.wn_ctx.borrow_mut();

        if ctx.update == UpdateDisplayRequest::RenderUpdate {
            ctx.update = UpdateDisplayRequest::None;

            let mut update = FrameUpdate::new(ctx.window_id, ctx.root.id, ctx.root_transform_key, self.frame_info.frame_id());

            ctx.root.child.render_update(&mut update);

            let update = update.finalize();
            if !update.transforms.is_empty() || !update.floats.is_empty() {
                if let Some(api) = &ctx.api {
                    // send request if has a renderer.
                    let mut txn = Transaction::new();
                    txn.set_root_pipeline(self.pipeline_id);
                    txn.update_dynamic_properties(update);
                    txn.generate_frame();
                    api.send_transaction(self.document_id, txn);
                } else {
                    // in renderless mode the frame is updated already.
                    self.renderless_event_sender
                        .as_ref()
                        .unwrap()
                        .send_event(AppEvent::NewFrameReady(self.id));
                }
            }
        }
    }

    /// Notifies the OS to redraw the window, will receive WindowEvent::RedrawRequested
    /// from the OS after calling this.
    fn request_redraw(&mut self) {
        if self.first_draw {
            let gl_ctx = self.gl_ctx.borrow();
            let window = gl_ctx.window();

            match self.wn_ctx.borrow().root.start_position {
                StartPosition::Default => {}
                StartPosition::CenterScreen => {
                    let size = window.outer_size();
                    let available_size = window
                        .current_monitor()
                        .map(|m| m.size())
                        .unwrap_or_else(|| glutin::dpi::PhysicalSize::new(0, 0));
                    let position = glutin::dpi::PhysicalPosition::new(
                        if size.width < available_size.width {
                            (available_size.width - size.width) / 2
                        } else {
                            0
                        },
                        if size.height < available_size.height {
                            (available_size.height - size.height) / 2
                        } else {
                            0
                        },
                    );
                    window.set_outer_position(position)
                }
                StartPosition::CenterOwner => {
                    todo!()
                }
            }

            self.first_draw = false;
            drop(gl_ctx);

            // draws the first frame before showing
            // because we can still flash white here.
            self.redraw();

            // apply user initial visibility
            if *self.wn_ctx.borrow().root.visible.get_local() {
                self.gl_ctx.borrow().window().set_visible(true);
            }
        } else {
            self.gl_ctx.borrow().window().request_redraw();
        }
    }

    /// Redraws the last ready frame and swaps buffers.
    ///
    /// **`swap_buffers` Warning**: if you enabled vsync, this function will block until the
    /// next time the screen is refreshed. However drivers can choose to
    /// override your vsync settings, which means that you can't know in
    /// advance whether `swap_buffers` will block or not.
    fn redraw(&mut self) {
        profile_scope!("window::redraw");

        let mut context = self.gl_ctx.borrow_mut();
        context.make_current();

        let renderer = self.renderer.borrow_mut();
        renderer.update();

        let size = context.window().inner_size();
        let device_size = units::DeviceIntSize::new(size.width as i32, size.height as i32);

        renderer.render(device_size).unwrap();
        let _ = renderer.flush_pipeline_info();

        context.swap_buffers();
        context.make_not_current();
    }

    fn deinited(&self) -> bool {
        self.renderer.deinited()
    }

    fn deinit(mut self) {
        self.gl_ctx.borrow_mut().make_current();
        self.renderer.deinit();
        self.gl_ctx.borrow_mut().make_not_current();
    }
}

/// # Windows OS Only
#[cfg(windows)]
impl OpenWindow {
    /// Windows OS window handler.
    ///
    /// # See Also
    ///
    /// * [`Self::generate_subclass_id`]
    /// * [`Self::set_raw_windows_event_handler`]
    ///
    /// # Panics
    ///
    /// Panics in headless mode.
    #[inline]
    pub fn hwnd(&self) -> winapi::shared::windef::HWND {
        use glutin::platform::windows::WindowExtWindows;
        self.gl_ctx.borrow().window().hwnd() as winapi::shared::windef::HWND
    }

    /// Generate Windows OS subclasses id that is unique for this window.
    #[inline]
    pub fn generate_subclass_id(&self) -> winapi::shared::basetsd::UINT_PTR {
        self.subclass_id.replace(self.subclass_id.get() + 1)
    }

    /// Sets a window subclass that calls a raw event handler.
    ///
    /// Use this to receive Windows OS events not covered in [`WindowEvent`].
    ///
    /// Returns if adding a subclass handler succeeded.
    ///
    /// # Handler
    ///
    /// The handler inputs are the first 4 arguments of a [`SUBCLASSPROC`](https://docs.microsoft.com/en-us/windows/win32/api/commctrl/nc-commctrl-subclassproc).
    /// You can use closure capture to include extra data.
    ///
    /// The handler must return `Some(LRESULT)` to stop the propagation of a specific message.
    ///
    /// The handler is dropped after it receives the `WM_DESTROY` message.
    ///
    /// # Panics
    ///
    /// Panics in headless mode.
    pub fn set_raw_windows_event_handler<
        H: FnMut(
                winapi::shared::windef::HWND,
                winapi::shared::minwindef::UINT,
                winapi::shared::minwindef::WPARAM,
                winapi::shared::minwindef::LPARAM,
            ) -> Option<winapi::shared::minwindef::LRESULT>
            + 'static,
    >(
        &self,
        handler: H,
    ) -> bool {
        let hwnd = self.hwnd();
        let data = Box::new(handler);
        unsafe {
            winapi::um::commctrl::SetWindowSubclass(
                hwnd,
                Some(Self::subclass_raw_event_proc::<H>),
                self.generate_subclass_id(),
                Box::into_raw(data) as winapi::shared::basetsd::DWORD_PTR,
            ) != 0
        }
    }

    unsafe extern "system" fn subclass_raw_event_proc<
        H: FnMut(
                winapi::shared::windef::HWND,
                winapi::shared::minwindef::UINT,
                winapi::shared::minwindef::WPARAM,
                winapi::shared::minwindef::LPARAM,
            ) -> Option<winapi::shared::minwindef::LRESULT>
            + 'static,
    >(
        hwnd: winapi::shared::windef::HWND,
        msg: winapi::shared::minwindef::UINT,
        wparam: winapi::shared::minwindef::WPARAM,
        lparam: winapi::shared::minwindef::LPARAM,
        _id: winapi::shared::basetsd::UINT_PTR,
        data: winapi::shared::basetsd::DWORD_PTR,
    ) -> winapi::shared::minwindef::LRESULT {
        match msg {
            winapi::um::winuser::WM_DESTROY => {
                // last call and cleanup.
                let mut handler = Box::from_raw(data as *mut H);
                handler(hwnd, msg, wparam, lparam).unwrap_or_default()
            }

            msg => {
                let handler = &mut *(data as *mut H);
                if let Some(r) = handler(hwnd, msg, wparam, lparam) {
                    r
                } else {
                    winapi::um::commctrl::DefSubclassProc(hwnd, msg, wparam, lparam)
                }
            }
        }
    }
}

impl Drop for OpenWindow {
    fn drop(&mut self) {
        if !self.deinited() {
            error_println!("dropping window without calling `OpenWindow::deinit`");
        }
    }
}

/// Window screenshot image data.
pub struct ScreenshotData {
    /// RGB8
    pub pixels: Vec<u8>,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Dpi scale when the screenshot was taken.
    pub dpi: f32,
}
impl ScreenshotData {
    /// Encode and save the screenshot image.
    pub fn save(&self, path: impl AsRef<std::path::Path>) -> image::ImageResult<()> {
        image::save_buffer(path, &self.pixels, self.width, self.height, image::ColorType::Rgb8)
    }
}

struct OwnedWindowContext {
    window_id: WindowId,
    mode: WindowMode,
    root_transform_key: WidgetTransformKey,
    state: WindowState,
    services: WindowServices,
    root: Window,
    api: Option<Arc<RenderApi>>,
    update: UpdateDisplayRequest,
}

impl OwnedWindowContext {
    fn root_context(&mut self, ctx: &mut AppContext, f: impl FnOnce(&mut Box<dyn UiNode>, &mut WidgetContext)) -> UpdateDisplayRequest {
        let root = &mut self.root;

        ctx.window_context(self.window_id, self.mode, &mut self.state, &mut self.services, &self.api, |ctx| {
            let child = &mut root.child;
            ctx.widget_context(root.id, &mut root.meta, |ctx| {
                f(child, ctx);
            });
        })
    }

    /// Call [`UiNode::init`](UiNode::init) in all nodes.
    pub fn init(&mut self, ctx: &mut AppContext) {
        profile_scope!("window::init");

        self.root.title.init_local(ctx.vars);
        self.root.clear_color.init_local(ctx.vars);
        self.root.visible.init_local(ctx.vars);
        self.root.auto_size.init_local(ctx.vars);

        let update = self.root_context(ctx, |root, ctx| {
            ctx.updates.layout();

            root.init(ctx);
        });
        self.update |= update;
    }

    /// Call [`UiNode::update_hp`](UiNode::update_hp) in all nodes.
    pub fn update_hp(&mut self, ctx: &mut AppContext) {
        profile_scope!("window::update_hp");

        let update = self.root_context(ctx, |root, ctx| root.update_hp(ctx));
        self.update |= update;
    }

    /// Call [`UiNode::update`](UiNode::update) in all nodes.
    pub fn update(&mut self, ctx: &mut AppContext) {
        profile_scope!("window::update");

        // do UiNode updates
        let update = self.root_context(ctx, |root, ctx| root.update(ctx));
        self.update |= update;
    }

    /// Call [`UiNode::deinit`](UiNode::deinit) in all nodes.
    pub fn deinit(&mut self, ctx: &mut AppContext) {
        profile_scope!("window::deinit");
        self.root_context(ctx, |root, ctx| root.deinit(ctx));
    }
}

#[derive(Clone)]
struct Notifier {
    window_id: WindowId,
    event_loop: EventLoopProxy,
}
impl RenderNotifier for Notifier {
    fn clone(&self) -> Box<dyn RenderNotifier> {
        Box::new(Clone::clone(self))
    }

    fn wake_up(&self) {}

    fn new_frame_ready(&self, _: DocumentId, _scrolled: bool, _composite_needed: bool, _: Option<u64>) {
        self.event_loop.send_event(app::AppEvent::NewFrameReady(self.window_id));
    }
}

#[allow(clippy::large_enum_variant)]
enum RendererState {
    Running(webrender::Renderer),
    Deinited,
    Renderless,
}

impl RendererState {
    fn deinit(&mut self) {
        match mem::replace(self, RendererState::Deinited) {
            RendererState::Running(r) => r.deinit(),
            RendererState::Deinited => panic!("renderer already deinited"),
            RendererState::Renderless => {}
        }
    }

    fn borrow_mut(&mut self) -> &mut webrender::Renderer {
        match self {
            RendererState::Running(wr) => wr,
            RendererState::Deinited => panic!("cannot borrow deinited renderer"),
            RendererState::Renderless => panic!("cannot borrow, running in renderless mode"),
        }
    }

    fn deinited(&self) -> bool {
        matches!(self, RendererState::Deinited)
    }
}

enum GlContextState {
    Current(WindowedContext<PossiblyCurrent>),
    NotCurrent(WindowedContext<NotCurrent>),
    HeadlessCurrent(glutin::Context<PossiblyCurrent>),
    HeadlessNotCurrent(glutin::Context<NotCurrent>),
    Changing,
    Renderless,
}

struct GlContext {
    ctx: GlContextState,
    gl: Option<Rc<dyn gl::Gl>>,
}

impl GlContext {
    fn new(window_builder: WindowBuilder, event_loop: &HeadedEventLoopWindowTarget) -> Self {
        let context = ContextBuilder::new()
            .with_gl(GlRequest::GlThenGles {
                opengl_version: (3, 2),
                opengles_version: (3, 0),
            })
            .build_windowed(window_builder, &event_loop)
            .unwrap();

        let context = unsafe { context.make_current().expect("couldn't make `GlContext` current") };

        let gl = match context.get_api() {
            Api::OpenGl => unsafe { gl::GlFns::load_with(|symbol| context.get_proc_address(symbol) as *const _) },
            Api::OpenGlEs => unsafe { gl::GlesFns::load_with(|symbol| context.get_proc_address(symbol) as *const _) },
            Api::WebGl => panic!("WebGl is not supported"),
        };

        GlContext {
            ctx: GlContextState::Current(context),
            gl: Some(gl),
        }
    }

    fn new_headless() -> Self {
        if !is_main_thread::is_main_thread().unwrap_or(true) {
            panic!("can only init renderer in the main thread")
        }

        let context = ContextBuilder::new().with_gl(GlRequest::GlThenGles {
            opengl_version: (3, 2),
            opengles_version: (3, 0),
        });

        let event_loop = glutin::event_loop::EventLoop::new();
        let size = glutin::dpi::PhysicalSize::new(1, 1);

        #[cfg(unix)]
        let context = {
            use glutin::platform::unix::HeadlessContextExt;

            match context.build_surfaceless(&event_loop) {
                Ok(c) => c,
                Err(e) => {
                    error_println!("failed to build a surfaceless context, {}", e);

                    match context.build_headless(&event_loop, size) {
                        Ok(c) => c,
                        Err(e) => {
                            error_println!("failed to build a fallback headless context, {}", e);

                            match context.build_osmesa(size) {
                                Ok(c) => c,
                                Err(e) => {
                                    error_println!("failed to build a fallback osmesa context, {}", e);
                                    Self::new_headless_hidden_window_ctx(&event_loop)
                                }
                            }
                        }
                    }
                }
            }
        };

        #[cfg(not(unix))]
        let context = {
            match context.build_headless(&event_loop, size) {
                Ok(c) => c,
                Err(e) => {
                    error_println!("failed to build a headless context, {}", e);
                    Self::new_headless_hidden_window_ctx(&event_loop)
                }
            }
        };

        let context = unsafe { context.make_current().expect("couldn't make `GlContext` current") };

        let gl = match context.get_api() {
            Api::OpenGl => unsafe { gl::GlFns::load_with(|symbol| context.get_proc_address(symbol) as *const _) },
            Api::OpenGlEs => unsafe { gl::GlesFns::load_with(|symbol| context.get_proc_address(symbol) as *const _) },
            Api::WebGl => panic!("WebGl is not supported"),
        };

        GlContext {
            ctx: GlContextState::HeadlessCurrent(context),
            gl: Some(gl),
        }
    }
    fn new_headless_hidden_window_ctx(_event_loop: &glutin::event_loop::EventLoop<()>) -> glutin::Context<NotCurrent> {
        unimplemented!("headless hidden window fallback not implemented")
    }

    fn new_renderless() -> Self {
        GlContext {
            ctx: GlContextState::Renderless,
            gl: None,
        }
    }

    fn gl(&self) -> &Rc<dyn gl::Gl> {
        self.gl.as_ref().expect("no Gl in renderless mode")
    }

    fn window(&self) -> &GlutinWindow {
        match &self.ctx {
            GlContextState::Current(c) => c.window(),
            GlContextState::NotCurrent(c) => c.window(),
            GlContextState::HeadlessCurrent(_) | GlContextState::HeadlessNotCurrent(_) => panic!("no window in headless mode"),
            GlContextState::Changing => unreachable!(),
            GlContextState::Renderless => panic!("no window in renderless mode"),
        }
    }

    fn window_opt(&self) -> Option<&GlutinWindow> {
        match &self.ctx {
            GlContextState::Current(c) => Some(c.window()),
            GlContextState::NotCurrent(c) => Some(c.window()),
            _ => None,
        }
    }

    fn make_current(&mut self) {
        self.ctx = match std::mem::replace(&mut self.ctx, GlContextState::Changing) {
            GlContextState::Current(_) | GlContextState::HeadlessCurrent(_) => {
                panic!("`GlContext` already is current");
            }
            GlContextState::NotCurrent(c) => {
                let c = unsafe { c.make_current().expect("couldn't make `GlContext` current") };
                GlContextState::Current(c)
            }
            GlContextState::HeadlessNotCurrent(c) => {
                let c = unsafe { c.make_current().expect("couldn't make `GlContext` current") };
                GlContextState::HeadlessCurrent(c)
            }
            GlContextState::Changing => unreachable!(),
            GlContextState::Renderless => panic!("no Gl context in renderless mode"),
        }
    }

    fn make_not_current(&mut self) {
        self.ctx = match mem::replace(&mut self.ctx, GlContextState::Changing) {
            GlContextState::Current(c) => {
                let c = unsafe { c.make_not_current().expect("couldn't make `GlContext` not current") };
                GlContextState::NotCurrent(c)
            }
            GlContextState::HeadlessCurrent(c) => {
                let c = unsafe { c.make_not_current().expect("couldn't make `GlContext` not current") };
                GlContextState::HeadlessNotCurrent(c)
            }
            GlContextState::NotCurrent(_) | GlContextState::HeadlessNotCurrent(_) => {
                panic!("`GlContext` already is not current");
            }
            GlContextState::Changing => unreachable!(),
            GlContextState::Renderless => panic!("no Gl context in renderless mode"),
        }
    }

    fn swap_buffers(&self) {
        match &self.ctx {
            GlContextState::Current(c) => c.swap_buffers().expect("failed to swap buffers"),
            GlContextState::HeadlessCurrent(_) => {
                // no error because we may be using a hidden window fallback
                // and it is presented as a headless context by the API.
            }
            GlContextState::NotCurrent(_) | &GlContextState::HeadlessNotCurrent(_) => {
                panic!("can only swap buffers of current contexts");
            }
            GlContextState::Changing => unreachable!(),
            GlContextState::Renderless => panic!("cannot swap buffer in renderless mode"),
        };
    }
}

#[cfg(test)]
mod headless_tests {
    use super::*;
    use crate::app::App;
    use crate::color::colors;
    use crate::{impl_ui_node, UiNode};

    #[test]
    pub fn new_window_no_render() {
        let mut app = App::default().run_headless();
        assert!(!app.renderer_enabled());

        app.with_context(|ctx| {
            ctx.services.req::<Windows>().open(|_| test_window());
        });

        app.update();
    }

    #[test]
    #[should_panic(expected = "can only init renderer in the main thread")]
    pub fn new_window_with_render() {
        let mut app = App::default().run_headless();
        app.enable_renderer(true);
        assert!(app.renderer_enabled());

        app.with_context(|ctx| {
            ctx.services.req::<Windows>().open(|_| test_window());
        });

        app.update();
    }

    #[test]
    pub fn query_frame() {
        let mut app = App::default().run_headless();

        app.with_context(|ctx| {
            ctx.services.req::<Windows>().open(|_| test_window());
        });

        app.update();

        let events = app.take_app_events();

        assert!(events.iter().any(|ev| matches!(ev, AppEvent::NewFrameReady(_))));

        app.with_context(|ctx| {
            let wn = ctx.services.req::<Windows>().windows().next().unwrap();

            assert_eq!(wn.id(), wn.frame_info().window_id());

            let root = wn.frame_info().root();

            let expected = Some(true);
            let actual = root.meta().get(FooMetaKey).copied();
            assert_eq!(expected, actual);

            let expected = LayoutRect::new(LayoutPoint::zero(), LayoutSize::new(20.0, 10.0));
            let actual = *root.bounds();
            assert_eq!(expected, actual);
        })
    }

    fn test_window() -> Window {
        Window::new(
            WidgetId::new_unique(),
            "",
            StartPosition::Default,
            (0, 0),
            (20, 10),
            false,
            false,
            colors::RED,
            true,
            WindowHeadlessConfig::default(),
            SetFooMetaNode,
        )
    }

    state_key! {
        struct FooMetaKey: bool;
    }

    struct SetFooMetaNode;
    #[impl_ui_node(none)]
    impl UiNode for SetFooMetaNode {
        fn render(&self, frame: &mut FrameBuilder) {
            frame.meta().set(FooMetaKey, true);
        }
    }
}
