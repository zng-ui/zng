//! App windows manager.
use crate::{
    app::{self, AppExtended, AppExtension, AppProcess, EventLoopProxy, EventLoopWindowTarget, ShutdownRequestedArgs},
    context::*,
    event::*,
    profiler::profile_scope,
    render::{
        FrameBuilder, FrameHitInfo, FrameId, FrameInfo, FrameUpdate, NewFrameArgs, RenderSize, Renderer, RendererConfig, WidgetTransformKey,
    },
    service::WindowServicesVisitors,
    service::{AppService, WindowServices},
    text::Text,
    units::{LayoutPoint, LayoutRect, LayoutSize, PixelGrid, Point, Size},
    var::{BoxedVar, IntoVar, VarObj, VarsRead},
    UiNode, WidgetId,
};

use app::AppEvent;
use fnv::FnvHashMap;

use glutin::window::WindowBuilder;
use rayon::{ThreadPool, ThreadPoolBuilder};
use std::{cell::RefCell, mem, num::NonZeroU16, rc::Rc, sync::Arc};
use webrender::api::{Epoch, PipelineId, RenderApi};

pub use glutin::{event::WindowEvent, window::CursorIcon};

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

/// Extension trait, adds [`run_window`](AppRunWindowExt::run_window) to [`AppExtended`].
pub trait AppRunWindowExt {
    /// Runs the application event loop and requests a new window.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use zero_ui_core::app::App;
    /// # use zero_ui_core::window::AppRunWindowExt;
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
    /// # use zero_ui_core::window::Windows;
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
impl<E: AppExtension> AppRunWindowExt for AppExtended<E> {
    fn run_window(self, new_window: impl FnOnce(&mut AppContext) -> Window + 'static) {
        self.run(|ctx| {
            ctx.services.req::<Windows>().open(new_window);
        })
    }
}

/// Extension trait, adds [`open_window`](HeadlessAppOpenWindowExt::open_window) to [`HeadlessApp`](app::HeadlessApp).
pub trait HeadlessAppOpenWindowExt {
    /// Open a new headless window and returns the new window ID.
    fn open_window(&mut self, new_window: impl FnOnce(&mut AppContext) -> Window + 'static) -> WindowId;

    /// Cause the headless window to think it is focused in the screen.
    fn activate_window(&mut self, window_id: WindowId);
    /// Cause the headless window to think focus moved away from it.
    fn deactivate_window(&mut self, window_id: WindowId);

    /// Sends a close request, returns if the window was found and closed.
    fn close_window(&mut self, window_id: WindowId) -> bool;
}
impl HeadlessAppOpenWindowExt for app::HeadlessApp {
    fn open_window(&mut self, new_window: impl FnOnce(&mut AppContext) -> Window + 'static) -> WindowId {
        let listener = self.with_context(|ctx| ctx.services.req::<Windows>().open(new_window));
        let mut window_id = None;
        self.update_observed(|_, ctx| {
            if let Some(opened) = listener.updates(ctx.events).first() {
                window_id = Some(opened.window_id);
            }
        });
        let window_id = window_id.expect("window did not open");

        self.activate_window(window_id);

        window_id
    }

    fn activate_window(&mut self, window_id: WindowId) {
        let event = WindowEvent::Focused(true);
        self.on_window_event(window_id, &event);
        self.update();
    }

    fn deactivate_window(&mut self, window_id: WindowId) {
        let event = WindowEvent::Focused(false);
        self.on_window_event(window_id, &event);
        self.update();
    }

    fn close_window(&mut self, window_id: WindowId) -> bool {
        let (closing_ls, closed_ls) = self.with_context(|ctx| {
            let cls = ctx.events.listen::<WindowCloseRequestedEvent>();
            let cld = ctx.events.listen::<WindowCloseEvent>();
            (cls, cld)
        });

        let event = WindowEvent::CloseRequested;
        self.on_window_event(window_id, &event);

        let mut requested = false;
        let mut closed = false;

        self.update_observed(|_, ctx| {
            for a in closing_ls.updates(ctx.events) {
                requested |= a.window_id == window_id;
            }
            for a in closed_ls.updates(ctx.events) {
                closed |= a.window_id == window_id;
            }
        });

        assert_eq!(requested, closed);

        closed
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
        /// Id of window that was activated or deactivated.
        pub window_id: WindowId,

        /// If the window was activated in this event.
        pub activated: bool,

        /// If the window was deactivated because it closed.
        pub closed: bool,

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

                    let args = WindowIsActiveArgs::now(window_id, window.is_active, false);
                    self.notify_activation(args, ctx.events);
                }
            }
            WindowEvent::Resized(_) => {
                if let Some(window) = ctx.services.req::<Windows>().windows.get_mut(&window_id) {
                    let new_size = window.size();

                    ctx.updates.layout();
                    window.expect_layout_update();
                    window.resize_renderer();

                    // set the window size variable if it is not read-only.
                    let wn_ctx = window.context.borrow();
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
                    let wn_ctx = window.context.borrow();
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
                    window.resize_renderer();

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
            let mut ctx = window.context.borrow_mut();
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
        // The windows don't do a layout update unless they recorded
        // an update request for layout or render.

        // we need to detach the windows from the ctx, because the window needs it
        // to create a layout context. Services are not visible in the layout context
        // so this is fine.
        let mut windows = mem::take(&mut ctx.services.req::<Windows>().windows);
        for (_, window) in windows.iter_mut() {
            window.layout(ctx);
            window.render(ctx);
            window.render_update(ctx);
        }
        ctx.services.req::<Windows>().windows = windows;
    }

    fn on_new_frame_ready(&mut self, window_id: WindowId, ctx: &mut AppContext) {
        if let Some(window) = ctx.services.req::<Windows>().windows.get_mut(&window_id) {
            window.request_redraw(ctx.vars);
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
                let mut w_ctx = window.context.borrow_mut();
                error_println!("dropping `{:?} ({})` without closing events", id, w_ctx.root.title.get(ctx.vars));
                w_ctx.deinit(ctx);
            }
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

            let wn_ctx = w.context.clone();
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
                .map(|(_, w)| w.context.clone())
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
                let mut windows = mem::take(&mut ctx.services.req::<Windows>().windows);
                for (_, window) in windows.iter_mut() {
                    window.update_window_vars(ctx);
                }
                ctx.services.req::<Windows>().windows = windows;
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
                w.context.clone().borrow_mut().deinit(ctx);
                if w.is_active {
                    let args = WindowIsActiveArgs::now(w.id, false, true);
                    self.notify_activation(args, ctx.events);
                }
            }
        }

        let service = ctx.services.req::<Windows>();
        if service.shutdown_on_last_close && service.windows.is_empty() {
            ctx.services.req::<AppProcess>().shutdown();
        }
    }

    fn notify_activation(&self, args: WindowIsActiveArgs, events: &Events) {
        debug_assert!(!args.closed || (args.closed && !args.activated));

        self.window_is_active_changed.notify(events, args.clone());
        let specif_event = if args.activated {
            &self.window_activated
        } else {
            &self.window_deactivated
        };
        specif_event.notify(events, args);
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
    state: LazyStateMap,
    id: WidgetId,
    title: BoxedVar<Text>,
    start_position: StartPosition,
    position: BoxedVar<Point>,
    size: BoxedVar<Size>,
    auto_size: BoxedVar<AutoSize>,
    resizable: BoxedVar<bool>,
    visible: BoxedVar<bool>,
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
        visible: impl IntoVar<bool>,
        headless_config: WindowHeadlessConfig,
        child: impl UiNode,
    ) -> Self {
        Window {
            state: LazyStateMap::new(),
            id: root_id,
            title: title.into_var().boxed(),
            start_position: start_position.into(),
            position: position.into_var().boxed(),
            size: size.into_var().boxed(),
            auto_size: auto_size.into_var().boxed(),
            resizable: resizable.into_var().boxed(),
            visible: visible.into_var().boxed(),
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
    context: Rc<RefCell<OwnedWindowContext>>,

    window: Option<glutin::window::Window>,
    renderer: Option<RefCell<Renderer>>,

    mode: WindowMode,
    id: WindowId,

    first_draw: bool,
    frame_info: FrameInfo,

    is_active: bool,

    #[cfg(windows)]
    subclass_id: std::cell::Cell<usize>,

    headless_config: WindowHeadlessConfig,
    headless_position: LayoutPoint,
    headless_size: LayoutSize,

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

        let id;

        let window;
        let renderer;

        let headless_config = root.headless_config.clone();
        let headless_position;
        let headless_size;
        let renderless_event_sender;

        let renderer_config = RendererConfig {
            clear_color: None,
            workers: Some(ui_threads),
        };
        match mode {
            WindowMode::Headed => {
                headless_position = LayoutPoint::zero();
                headless_size = LayoutSize::zero();
                renderless_event_sender = None;

                let window_ = WindowBuilder::new()
                    .with_visible(false) // not visible until first render, to avoid flickering
                    .with_resizable(*root.resizable.get(ctx.vars))
                    .with_title(root.title.get(ctx.vars).to_owned())
                    .with_resizable(*root.auto_size.get(ctx.vars) != AutoSize::CONTENT);

                let event_loop = event_loop.headed_target().expect("AppContext is not headless but event_loop is");

                let r = Renderer::new_with_glutin(window_, &event_loop, renderer_config, move |args: NewFrameArgs| {
                    event_loop_proxy.send_event(AppEvent::NewFrameReady(WindowId::System(args.window_id.unwrap())))
                })
                .expect("failed to create a window renderer");

                renderer = Some(RefCell::new(r.0));

                let window_ = r.1;
                id = WindowId::System(window_.id());

                let pixel_factor = window_.scale_factor() as f32;

                // available size to calculate relative values in the initial position and size.
                let available_size = window_
                    .current_monitor()
                    .map(|m| {
                        let s = m.size();
                        if s.width == 0 {
                            // Web
                            LayoutSize::new(800.0, 600.0)
                        } else {
                            // Monitor
                            LayoutSize::new(s.width as f32 / pixel_factor, s.height as f32 / pixel_factor)
                        }
                    })
                    .unwrap_or_else(|| {
                        // No Monitor
                        LayoutSize::new(800.0, 600.0)
                    });

                let mut position = LayoutPoint::zero();
                let mut size = LayoutSize::zero();
                ctx.outer_layout_context(available_size, pixel_factor, id, root.id, |size_ctx| {
                    position = root.position.get(size_ctx.vars).to_layout(available_size, &size_ctx);
                    size = root.size.get(size_ctx.vars).to_layout(available_size, &size_ctx);
                });

                let fallback_pos = window_.outer_position().map(|p| (p.x, p.y)).unwrap_or_default();
                let fallback_size = window_.inner_size();

                let mut used_fallback = false;
                if !position.x.is_finite() {
                    position.x = fallback_pos.0 as f32 / pixel_factor;
                    used_fallback = true;
                }
                if !position.y.is_finite() {
                    position.y = fallback_pos.1 as f32 / pixel_factor;
                    used_fallback = true;
                }
                if used_fallback {
                    let _ = root.position.set(ctx.vars, position.to_tuple().into());
                }

                used_fallback = false;
                if !size.width.is_finite() {
                    size.width = fallback_size.width as f32 / pixel_factor;
                    used_fallback = true;
                }
                if !size.height.is_finite() {
                    size.height = fallback_size.height as f32 / pixel_factor;
                    used_fallback = true;
                }
                if used_fallback {
                    let _ = root.size.set(ctx.vars, size.to_tuple().into());
                }

                window = Some(window_);
            }
            headless => {
                window = None;
                renderless_event_sender = Some(event_loop_proxy.clone());

                id = WindowId::new_unique();

                let pixel_factor = headless_config.scale_factor;
                let available_size = headless_config.screen_size;

                let mut position = LayoutPoint::zero();
                let mut size = LayoutSize::zero();
                ctx.outer_layout_context(available_size, pixel_factor, id, root.id, |size_ctx| {
                    position = root.position.get(size_ctx.vars).to_layout(available_size, &size_ctx);
                    size = root.size.get(size_ctx.vars).to_layout(available_size, &size_ctx);
                });

                let mut used_fallback = false;
                if !position.x.is_finite() {
                    position.x = 0.0;
                    used_fallback = true;
                }
                if !position.y.is_finite() {
                    position.y = 0.0;
                    used_fallback = true;
                }
                if used_fallback {
                    let _ = root.size.set(ctx.vars, position.to_tuple().into());
                }
                headless_position = position;

                used_fallback = false;
                if !size.width.is_finite() {
                    size.width = available_size.width;
                    used_fallback = true;
                }
                if !size.height.is_finite() {
                    size.height = available_size.height;
                    used_fallback = true;
                }
                if used_fallback {
                    let _ = root.size.set(ctx.vars, size.to_tuple().into());
                }
                headless_size = size;

                renderer = if headless == WindowMode::HeadlessWithRenderer {
                    let size = RenderSize::new((size.width * pixel_factor) as i32, (size.height * pixel_factor) as i32);
                    Some(RefCell::new(
                        Renderer::new(size, pixel_factor, renderer_config, move |_| {
                            event_loop_proxy.send_event(AppEvent::NewFrameReady(id))
                        })
                        .expect("failed to create a headless renderer"),
                    ))
                } else {
                    None
                };
            }
        }

        let api = renderer.as_ref().map(|r| r.borrow().api().clone());

        // init window state and services.
        let (state, services) = ctx.new_window(id, mode, &api);

        let frame_info = FrameInfo::blank(id, root.id);

        OpenWindow {
            context: Rc::new(RefCell::new(OwnedWindowContext {
                window_id: id,
                mode,
                root_transform_key: WidgetTransformKey::new_unique(),
                state,
                services,
                root,
                api,
                update: UpdateDisplayRequest::Layout,
            })),
            window,
            renderer,
            id,
            headless_position,
            headless_size,
            headless_config,
            mode,
            first_draw: true,
            is_active: true, // just opened? TODO
            frame_info,
            renderless_event_sender,

            #[cfg(windows)]
            subclass_id: std::cell::Cell::new(0),
        }
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
        if let Some(window) = &self.window {
            let scale = window.scale_factor() as f32;
            let pos = window.outer_position().map(|p| (p.x, p.y)).unwrap_or_default();
            LayoutPoint::new(pos.0 as f32 / scale, pos.1 as f32 / scale)
        } else {
            self.headless_position
        }
    }

    /// Size of the window content.
    #[inline]
    pub fn size(&self) -> LayoutSize {
        if let Some(window) = &self.window {
            let scale = window.scale_factor() as f32;
            let size = window.inner_size();
            LayoutSize::new(size.width as f32 / scale, size.height as f32 / scale)
        } else {
            self.headless_size
        }
    }

    /// Scale factor used by this window, all `Layout*` values are scaled by this value by the renderer.
    #[inline]
    pub fn scale_factor(&self) -> f32 {
        if let Some(window) = &self.window {
            window.scale_factor() as f32
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
        if let Some(renderer) = &self.renderer {
            let results = renderer.borrow().hit_test(point);
            FrameHitInfo::new(self.id(), self.frame_info.frame_id(), point, results)
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
        let max_rect = LayoutRect::from_size(self.size());
        let rect = rect.intersection(&max_rect).unwrap_or_default();
        let dpi = self.scale_factor();
        let rect = rect * dpi;

        let x = rect.origin.x as u32;
        let y = rect.origin.y as u32;
        let width = rect.size.width as u32;
        let height = rect.size.height as u32;

        if width == 0 || height == 0 {
            return ScreenshotData {
                pixels: vec![],
                width,
                height,
                dpi,
            };
        }

        if let Some(renderer) = &self.renderer {
            let pixels = renderer
                .borrow_mut()
                .read_pixels(x, y, width, height)
                .expect("failed to read pixels");

            let mut pixels_flipped = Vec::with_capacity(pixels.len());
            for v in (0..height as _).rev() {
                let s = 4 * v as usize * width as usize;
                let o = 4 * width as usize;
                pixels_flipped.extend_from_slice(&pixels[s..(s + o)]);
            }
            ScreenshotData {
                pixels: pixels_flipped,
                width,
                height,
                dpi,
            }
        } else {
            panic!("cannot screenshot in renderless mode")
        }
    }

    /// Manually flags layout to actually update on the next call.
    ///
    /// This is required for updates generated outside of this window but that affect this window.
    fn expect_layout_update(&mut self) {
        self.context.borrow_mut().update |= UpdateDisplayRequest::Layout;
    }

    /// Update from/to variables that affect the window.
    fn update_window_vars(&mut self, app_ctx: &mut AppContext) {
        let ctx = self.context.borrow();
        if let Some(window) = &self.window {
            // title
            if let Some(title) = ctx.root.title.get_new(app_ctx.vars) {
                window.set_title(title);
            }

            // auto-size
            if let Some(&auto_size) = ctx.root.auto_size.get_new(app_ctx.vars) {
                app_ctx.updates.layout();

                if auto_size == AutoSize::CONTENT {
                    window.set_resizable(false);
                } else {
                    // TODO is there a way to disable resize in only one dimension?
                    window.set_resizable(*ctx.root.resizable.get(app_ctx.vars));
                }
            }

            if ctx.root.position.is_new(app_ctx.vars) || ctx.root.size.is_new(app_ctx.vars) {
                let vars = app_ctx.vars;
                self.outer_layout_context(ctx.root.id, app_ctx, |layout_ctx| {
                    // position
                    if let Some(&new_pos) = ctx.root.position.get_new(vars) {
                        let (curr_x, curr_y) = window.outer_position().map(|p| (p.x, p.y)).unwrap_or_default();

                        let mut new_pos = new_pos.to_layout(*layout_ctx.viewport_size, &layout_ctx);
                        let factor = layout_ctx.pixel_grid.get().scale_factor;

                        if !new_pos.x.is_finite() {
                            new_pos.x = curr_x as f32 / factor;
                        }
                        if !new_pos.y.is_finite() {
                            new_pos.y = curr_y as f32 / factor;
                        }

                        let new_x = (new_pos.x * factor) as i32;
                        let new_y = (new_pos.y * factor) as i32;

                        if new_x != curr_x || new_y != curr_y {
                            window.set_outer_position(glutin::dpi::PhysicalPosition::new(new_x, new_y));
                        }
                    }

                    // size
                    if let Some(&new_size) = ctx.root.size.get_new(vars) {
                        let curr_size = window.inner_size();

                        let mut new_size = new_size.to_layout(*layout_ctx.viewport_size, &layout_ctx);
                        let factor = layout_ctx.pixel_grid.get().scale_factor;

                        let auto_size = *ctx.root.auto_size.get(layout_ctx.vars);

                        if auto_size.contains(AutoSize::CONTENT_WIDTH) || !new_size.width.is_finite() {
                            new_size.width = curr_size.width as f32 / factor;
                        }
                        if auto_size.contains(AutoSize::CONTENT_HEIGHT) || !new_size.height.is_finite() {
                            new_size.height = curr_size.height as f32 / factor;
                        }

                        let new_size = glutin::dpi::PhysicalSize::new((new_size.width * factor) as u32, (new_size.height * factor) as u32);

                        if new_size != curr_size {
                            window.set_inner_size(new_size);
                        }
                    }
                });
            }

            // resizable
            if let Some(&resizable) = ctx.root.resizable.get_new(app_ctx.vars) {
                let auto_size = *ctx.root.auto_size.get(app_ctx.vars);
                window.set_resizable(resizable && auto_size != AutoSize::CONTENT);
            }

            // visibility
            if let Some(&vis) = ctx.root.visible.get_new(app_ctx.vars) {
                if !self.first_draw {
                    window.set_visible(vis);
                    if vis {
                        app_ctx.updates.layout();
                    }
                }
            }
        } else {
            if ctx.root.position.is_new(app_ctx.vars) || ctx.root.size.is_new(app_ctx.vars) {
                let mut h_pos = self.headless_position;
                let mut h_size = self.headless_size;
                let vars = app_ctx.vars;
                self.outer_layout_context(ctx.root.id, app_ctx, |layout_ctx| {
                    if let Some(position) = ctx.root.position.get_new(vars) {
                        h_pos = position.to_layout(*layout_ctx.viewport_size, &layout_ctx);
                    }

                    if let Some(size) = ctx.root.size.get_new(vars) {
                        h_size = size.to_layout(*layout_ctx.viewport_size, &layout_ctx);
                    }
                });
                self.headless_position = h_pos;
                self.headless_size = h_size;
            }

            if let Some(&vis) = ctx.root.visible.get_new(app_ctx.vars) {
                if !self.first_draw && vis {
                    app_ctx.updates.layout();
                }
            }
        }
    }

    /// [`LayoutContext`] for the window size and position relative values.
    fn outer_layout_context(&self, root_id: WidgetId, ctx: &mut AppContext, f: impl FnOnce(&mut LayoutContext)) {
        if let Some(window) = &self.window {
            let pixel_factor = window.scale_factor() as f32;
            let screen_size = window
                .current_monitor()
                .map(|m| {
                    let s = m.size();
                    if s.width == 0 {
                        // Web
                        LayoutSize::new(800.0, 600.0)
                    } else {
                        // Monitor
                        LayoutSize::new(s.width as f32 / pixel_factor, s.height as f32 / pixel_factor)
                    }
                })
                .unwrap_or_else(|| {
                    // No Monitor
                    LayoutSize::new(800.0, 600.0)
                });

            ctx.outer_layout_context(screen_size, pixel_factor, self.id, root_id, f);
        } else {
            ctx.outer_layout_context(
                self.headless_config.screen_size,
                self.headless_config.scale_factor,
                self.id,
                root_id,
                f,
            );
        }
    }

    /// Re-flow layout if a layout pass was required. If yes will
    /// flag a render required.
    fn layout(&mut self, ctx: &mut AppContext) {
        let mut w_ctx = self.context.borrow_mut();

        if w_ctx.update != UpdateDisplayRequest::Layout {
            return;
        }

        profile_scope!("window::layout");

        let auto_size = *w_ctx.root.auto_size.get(ctx.vars);

        let mut size = LayoutSize::zero();
        let scale_factor = self.scale_factor();

        w_ctx.root_layout(ctx, self.size(), scale_factor, |root, layout_ctx| {
            size = root.measure(layout_ctx, *layout_ctx.viewport_size);

            if !auto_size.contains(AutoSize::CONTENT_WIDTH) {
                size.width = layout_ctx.viewport_size.get().width;
            }
            if !auto_size.contains(AutoSize::CONTENT_HEIGHT) {
                size.height = layout_ctx.viewport_size.get().height;
            }

            root.arrange(layout_ctx, size);
        });

        if auto_size != AutoSize::DISABLED {
            if let Some(window) = &self.window {
                let factor = scale_factor;
                let size = glutin::dpi::PhysicalSize::new((size.width * factor) as u32, (size.height * factor) as u32);
                window.set_inner_size(size);
            } else {
                self.headless_size = size;
            }
        }

        w_ctx.update = UpdateDisplayRequest::Render;
    }

    /// Resize the renderer surface.
    ///
    /// Must be called when the window is resized and/or the scale factor changed.
    fn resize_renderer(&mut self) {
        let size = self.size();
        let scale = self.scale_factor();
        if let Some(renderer) = &mut self.renderer {
            let size = RenderSize::new((size.width * scale) as i32, (size.height * scale) as i32);
            renderer.get_mut().resize(size, scale).expect("failed to resize the renderer");
        }
    }

    /// Render a frame if one was required.
    fn render(&mut self, app_ctx: &mut AppContext) {
        let mut ctx = self.context.borrow_mut();

        if ctx.update != UpdateDisplayRequest::Render {
            return;
        }

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

        let pipeline_id = if let Some(renderer) = &self.renderer {
            renderer.borrow().pipeline_id()
        } else {
            PipelineId::dummy()
        };

        let mut frame = FrameBuilder::new(
            frame_id,
            ctx.window_id,
            pipeline_id,
            ctx.api.clone(),
            ctx.root.id,
            ctx.root_transform_key,
            size,
            self.scale_factor(),
        );

        ctx.root_render(app_ctx, |child, ctx| {
            child.render(ctx, &mut frame);
        });

        let (display_list_data, frame_info) = frame.finalize();

        self.frame_info = frame_info;

        if let Some(renderer) = &mut self.renderer {
            renderer.get_mut().render(display_list_data, frame_id);
        } else {
            // in renderless mode we only have the frame_info.
            self.renderless_event_sender
                .as_ref()
                .unwrap()
                .send_event(AppEvent::NewFrameReady(self.id));
        }
    }

    /// Render a frame update if one was required.
    fn render_update(&mut self, app_ctx: &mut AppContext) {
        let mut ctx = self.context.borrow_mut();

        if ctx.update != UpdateDisplayRequest::RenderUpdate {
            return;
        }

        ctx.update = UpdateDisplayRequest::None;

        let mut update = FrameUpdate::new(ctx.window_id, ctx.root.id, ctx.root_transform_key, self.frame_info.frame_id());

        ctx.root_render(app_ctx, |child, ctx| {
            child.render_update(ctx, &mut update);
        });

        let update = update.finalize();

        if !update.transforms.is_empty() || !update.floats.is_empty() {
            if let Some(renderer) = &mut self.renderer {
                renderer.get_mut().render_update(update);
            } else {
                // in renderless mode we only have the frame_info.
                self.renderless_event_sender
                    .as_ref()
                    .unwrap()
                    .send_event(AppEvent::NewFrameReady(self.id));
            }
        }
    }

    /// Notifies the OS to redraw the window, will receive WindowEvent::RedrawRequested
    /// from the OS after calling this.
    fn request_redraw(&mut self, vars: &VarsRead) {
        if let Some(window) = &self.window {
            if self.first_draw {
                self.first_draw = false;

                // apply start position.
                match self.context.borrow().root.start_position {
                    StartPosition::Default => {}
                    StartPosition::CenterScreen => {
                        let size = window.outer_size();
                        let screen_size = window
                            .current_monitor()
                            .map(|m| m.size())
                            .unwrap_or_else(|| glutin::dpi::PhysicalSize::new(0, 0));

                        let position = glutin::dpi::PhysicalPosition::new(
                            if size.width < screen_size.width {
                                (screen_size.width - size.width) / 2
                            } else {
                                0
                            },
                            if size.height < screen_size.height {
                                (screen_size.height - size.height) / 2
                            } else {
                                0
                            },
                        );

                        window.set_outer_position(position);
                    }
                    StartPosition::CenterOwner => {
                        // TODO, after window.owner is implemented.
                    }
                }

                self.redraw();

                // apply initial visibility.
                if *self.context.borrow().root.visible.get(vars) {
                    self.window.as_ref().unwrap().set_visible(true);
                }
            } else {
                window.request_redraw();
            }
        } else if self.renderer.is_some() {
            self.redraw();
        }
    }

    /// Redraws the last ready frame and swaps buffers.
    fn redraw(&mut self) {
        if let Some(renderer) = &mut self.renderer {
            profile_scope!("window::redraw");

            renderer.get_mut().present().expect("failed presenting frame");
        }
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
        if let Some(window) = &self.window {
            window.hwnd() as winapi::shared::windef::HWND
        } else {
            panic!("headless windows dont have a HWND");
        }
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
        // these need to be dropped in this order.
        let _ = self.renderer.take();
        let _ = self.window.take();
    }
}

/// Window screenshot image data.
pub struct ScreenshotData {
    /// RGBA8
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
        image::save_buffer(path, &self.pixels, self.width, self.height, image::ColorType::Rgba8)
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
            ctx.widget_context(root.id, &mut root.state, |ctx| {
                f(child, ctx);
            });
        })
    }

    fn root_layout(
        &mut self,
        ctx: &mut AppContext,
        window_size: LayoutSize,
        scale_factor: f32,
        f: impl FnOnce(&mut Box<dyn UiNode>, &mut LayoutContext),
    ) {
        let root = &mut self.root;
        ctx.window_context(self.window_id, self.mode, &mut self.state, &mut self.services, &self.api, |ctx| {
            let child = &mut root.child;
            ctx.layout_context(14.0, PixelGrid::new(scale_factor), window_size, root.id, &mut root.state, |ctx| {
                f(child, ctx);
            })
        });
    }

    fn root_render(&mut self, ctx: &mut AppContext, f: impl FnOnce(&mut Box<dyn UiNode>, &mut RenderContext)) {
        let root = &mut self.root;
        ctx.window_context(self.window_id, self.mode, &mut self.state, &mut self.services, &self.api, |ctx| {
            let child = &mut root.child;
            ctx.render_context(root.id, &root.state, |ctx| f(child, ctx))
        });
    }

    /// Call [`UiNode::init`](UiNode::init) in all nodes.
    pub fn init(&mut self, ctx: &mut AppContext) {
        profile_scope!("window::init");

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

#[cfg(test)]
mod headless_tests {
    use super::*;
    use crate::app::App;
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
        fn render(&self, _: &mut RenderContext, frame: &mut FrameBuilder) {
            frame.meta().set(FooMetaKey, true);
        }
    }
}
