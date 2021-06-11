//! App windows manager.
use crate::{
    app::{self, AppExtended, AppExtension, AppProcess, EventLoopProxy, EventLoopWindowTarget, ShutdownRequestedArgs},
    context::*,
    event::*,
    profiler::profile_scope,
    render::*,
    service::Service,
    text::{Text, ToText},
    units::*,
    var::{response_done_var, response_var, var, IntoValue, RcVar, ResponderVar, ResponseVar, VarsRead},
    BoxedUiNode, UiNode, WidgetId,
};

use app::AppEvent;

use glutin::window::WindowBuilder;
use rayon::{ThreadPool, ThreadPoolBuilder};
use std::{
    cell::{Cell, RefCell},
    fmt, mem,
    rc::Rc,
    sync::Arc,
};
use webrender::api::{Epoch, PipelineId, RenderApi};

pub use glutin::{event::WindowEvent, window::CursorIcon};

unique_id! {
    /// Unique identifier of a headless window.
    ///
    /// See [`WindowId`] for more details.
    pub struct HeadlessWindowId;
}

/// Unique identifier of a headed window or a headless window backed by a hidden system window.
///
/// See [`WindowId`] for more details.
pub type SystemWindowId = glutin::window::WindowId;

/// Unique identifier of a [`OpenWindow`].
///
/// Can be obtained from [`OpenWindow::id`] or [`WindowContext::window_id`] or [`WidgetContext::path`].
#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub enum WindowId {
    /// The id for a *real* system window, this is the case for all windows in [headed mode](OpenWindow::mode)
    /// and also for headless windows with renderer enabled in compatibility mode, when a hidden window is used.
    System(SystemWindowId),
    /// The id for a headless window, when the window is not backed by a system window.
    Headless(HeadlessWindowId),
}
impl WindowId {
    /// New unique [`Headless`](Self::Headless) window id.
    #[inline]
    pub fn new_unique() -> Self {
        WindowId::Headless(HeadlessWindowId::new_unique())
    }
}
impl From<SystemWindowId> for WindowId {
    fn from(id: SystemWindowId) -> Self {
        WindowId::System(id)
    }
}
impl From<HeadlessWindowId> for WindowId {
    fn from(id: HeadlessWindowId) -> Self {
        WindowId::Headless(id)
    }
}
impl fmt::Debug for WindowId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WindowId::System(s) => {
                let window_id = format!("{:?}", s);
                let window_id_raw = window_id.trim_start_matches("WindowId(").trim_end_matches(')');
                if f.alternate() {
                    write!(f, "WindowId::System({})", window_id_raw)
                } else {
                    write!(f, "WindowId({})", window_id_raw)
                }
            }
            WindowId::Headless(s) => {
                if f.alternate() {
                    write!(f, "WindowId::Headless({})", s.get())
                } else {
                    write!(f, "WindowId({})", s.get())
                }
            }
        }
    }
}
impl fmt::Display for WindowId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WindowId::System(s) => {
                let window_id = format!("{:?}", s);
                let window_id_raw = window_id.trim_start_matches("WindowId(").trim_end_matches(')');
                write!(f, "WinId({})", window_id_raw)
            }
            WindowId::Headless(s) => {
                write!(f, "WinId({})", s.get())
            }
        }
    }
}

/// Extension trait, adds [`run_window`](AppRunWindowExt::run_window) to [`AppExtended`].
pub trait AppRunWindowExt {
    /// Runs the application event loop and requests a new window.
    ///
    /// The `new_window` argument is the [`WindowContext`] of the new window.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use zero_ui_core::app::App;
    /// # use zero_ui_core::window::AppRunWindowExt;
    /// # macro_rules! window { ($($tt:tt)*) => { todo!() } }
    /// App::default().run_window(|ctx| {
    ///     println!("starting app with window {:?}", ctx.window_id);
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
    ///     ctx.services.req::<Windows>().open(|ctx| {
    ///         println!("starting app with window {:?}", ctx.window_id);
    ///         window! {
    ///             title = "Window 1";
    ///             content = text("Window 1");
    ///         }
    ///     }, None);
    /// })   
    /// ```
    fn run_window(self, new_window: impl FnOnce(&mut WindowContext) -> Window + 'static) -> !;
}
impl<E: AppExtension> AppRunWindowExt for AppExtended<E> {
    fn run_window(self, new_window: impl FnOnce(&mut WindowContext) -> Window + 'static) -> ! {
        self.run(|ctx| {
            ctx.services.req::<Windows>().open(new_window, None);
        })
    }
}

/// Extension trait, adds [`open_window`](HeadlessAppWindowExt::open_window) to [`HeadlessApp`](app::HeadlessApp).
pub trait HeadlessAppWindowExt {
    /// Open a new headless window and returns the new window ID.
    ///
    /// The `new_window` argument is the [`WindowContext`] of the new window.
    ///
    /// Returns the [`WindowId`] of the new window.
    fn open_window(&mut self, new_window: impl FnOnce(&mut WindowContext) -> Window + 'static) -> WindowId;

    /// Cause the headless window to think it is focused in the screen.
    fn focus_window(&mut self, window_id: WindowId);
    /// Cause the headless window to think focus moved away from it.
    fn blur_window(&mut self, window_id: WindowId);

    /// Copy the current frame pixels of the window.
    fn frame_pixels(&mut self, window_id: WindowId) -> FramePixels;

    /// Sleeps until the next window frame is rendered, then returns the frame pixels.
    fn wait_frame(&mut self, window_id: WindowId) -> FramePixels;

    /// Sends a close request, returns if the window was found and closed.
    fn close_window(&mut self, window_id: WindowId) -> bool;
}
impl HeadlessAppWindowExt for app::HeadlessApp {
    fn open_window(&mut self, new_window: impl FnOnce(&mut WindowContext) -> Window + 'static) -> WindowId {
        let response = self.ctx().services.req::<Windows>().open(new_window, None);
        let mut window_id = None;
        while window_id.is_none() {
            self.update_observe(
                |ctx| {
                    if let Some(opened) = response.response_new(ctx.vars) {
                        window_id = Some(opened.window_id);
                    }
                },
                true,
            );
        }
        let window_id = window_id.unwrap();

        self.focus_window(window_id);

        window_id
    }

    fn focus_window(&mut self, window_id: WindowId) {
        let focused = self
            .ctx()
            .services
            .req::<Windows>()
            .windows()
            .iter()
            .find(|w| w.is_focused())
            .map(|w| w.id());

        if let Some(focused) = focused {
            // blur_window
            let event = WindowEvent::Focused(false);
            self.window_event(focused, &event);
        }
        let event = WindowEvent::Focused(true);
        self.window_event(window_id, &event);
        self.update(false);
    }

    fn blur_window(&mut self, window_id: WindowId) {
        let event = WindowEvent::Focused(false);
        self.window_event(window_id, &event);
        self.update(false);
    }

    fn wait_frame(&mut self, window_id: WindowId) -> FramePixels {
        // the current frame for comparison.
        let frame_id = self
            .ctx()
            .services
            .req::<Windows>()
            .window(window_id)
            .ok()
            .map(|w| w.frame_info().frame_id());

        loop {
            self.update(true);

            if let Ok(w) = self.ctx().services.req::<Windows>().window(window_id) {
                if Some(w.frame_info().frame_id()) != frame_id {
                    // is a new frame, get the pixels.
                    return w.frame_pixels();
                }
            }
        }
    }

    fn frame_pixels(&mut self, window_id: WindowId) -> FramePixels {
        self.ctx()
            .services
            .req::<Windows>()
            .window(window_id)
            .expect("window not found")
            .frame_pixels()
    }

    fn close_window(&mut self, window_id: WindowId) -> bool {
        let event = WindowEvent::CloseRequested;
        self.window_event(window_id, &event);

        let mut requested = false;
        let mut closed = false;

        self.update_observe_event(
            |_, args| {
                if let Some(args) = WindowCloseRequestedEvent::update(args) {
                    requested |= args.window_id == window_id;
                } else if let Some(args) = WindowCloseEvent::update(args) {
                    closed |= args.window_id == window_id;
                }
            },
            false,
        );

        assert_eq!(requested, closed);

        closed
    }
}

event_args! {
    /// [`WindowOpenEvent`], [`WindowCloseEvent`] args.
    pub struct WindowEventArgs {
        /// Id of window that was opened or closed.
        pub window_id: WindowId,

        /// `true` if the window opened, `false` if it closed.
        pub opened: bool,

        ..

        /// If the widget is in the same window.
        fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
            ctx.path.window_id() == self.window_id
        }
    }

    /// [`WindowFocusChangedEvent`], [`WindowFocusEvent`], [`WindowBlurEvent`] args.
    pub struct WindowIsFocusedArgs {
        /// Id of window that got or lost keyboard focus.
        pub window_id: WindowId,

        /// `true` if the window got focus, `false` if the window lost focus (blur).
        pub focused: bool,

        /// If the window was lost focus because it closed.
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

        ..

        /// If the widget is in the same window.
        fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
            ctx.path.window_id() == self.window_id
        }
    }
}

event! {
    /// Window resized event.
    pub WindowResizeEvent: WindowResizeArgs;

    /// Window moved event.
    pub WindowMoveEvent: WindowMoveArgs;

    /// New window event.
    pub WindowOpenEvent: WindowEventArgs;

    /// Window focus/blur event.
    pub WindowFocusChangedEvent: WindowIsFocusedArgs;

    /// Window got keyboard focus event.
    pub WindowFocusEvent: WindowIsFocusedArgs;

    /// Window lost keyboard event.
    pub WindowBlurEvent: WindowIsFocusedArgs;

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
/// * [WindowFocusChangedEvent]
/// * [WindowFocusEvent]
/// * [WindowBlurEvent]
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
        }
    }
}

impl AppExtension for WindowManager {
    fn init(&mut self, r: &mut AppInitContext) {
        self.event_loop_proxy = Some(r.event_loop.clone());
        r.services.register(Windows::new(r.updates.sender().clone()));
    }

    fn window_event(&mut self, ctx: &mut AppContext, window_id: WindowId, event: &WindowEvent) {
        match event {
            WindowEvent::Focused(focused) => {
                if let Some(window) = ctx.services.req::<Windows>().windows.iter_mut().find(|w| w.id == window_id) {
                    window.is_focused = *focused;

                    let args = WindowIsFocusedArgs::now(window_id, window.is_focused, false);
                    self.notify_focus(args, ctx.events);
                }
            }
            WindowEvent::Resized(_) => {
                if let Some(window) = ctx.services.req::<Windows>().windows.iter_mut().find(|w| w.id == window_id) {
                    let new_size = window.size();

                    // set the window size variable.
                    let new_size_l = Size::from(new_size);
                    if window.vars.size().get(ctx.vars) != &new_size_l {
                        // is new size:
                        window.vars.size().set(ctx.vars, new_size_l);
                        ctx.updates.layout();
                        window.expect_layout_update();
                        window.resize_renderer();

                        // raise window_resize
                        WindowResizeEvent::notify(ctx.events, WindowResizeArgs::now(window_id, new_size));
                    }
                }
            }
            WindowEvent::Moved(_) => {
                if let Some(window) = ctx.services.req::<Windows>().windows.iter().find(|w| w.id == window_id) {
                    let new_position = window.position();

                    // TODO check if in new monitor.

                    // set the window position variable if it is not read-only.
                    window.vars.position().set_ne(ctx.vars, new_position.into());

                    // raise window_move
                    WindowMoveEvent::notify(ctx.events, WindowMoveArgs::now(window_id, new_position));
                }
            }
            WindowEvent::CloseRequested => {
                if let Some(win) = ctx.services.req::<Windows>().windows.iter().find(|w| w.id == window_id) {
                    *win.close_response.borrow_mut() = Some(response_var().0);
                    ctx.updates.update();
                }
            }
            WindowEvent::ScaleFactorChanged {
                scale_factor,
                new_inner_size,
            } => {
                if let Some(window) = ctx.services.req::<Windows>().windows.iter_mut().find(|w| w.id == window_id) {
                    let scale_factor = *scale_factor as f32;
                    let new_size = LayoutSize::new(
                        new_inner_size.width as f32 / scale_factor,
                        new_inner_size.height as f32 / scale_factor,
                    );

                    // winit has not set the new_inner_size yet, so
                    // we can determinate if the system only changed the size
                    // to visually match the new scale_factor or if the window was
                    // really resized.
                    if *window.vars.size().get(ctx.vars) == new_size.into() {
                        // if it only changed to visually match, the WindowEvent::Resized
                        // will not cause a re-layout, so we need to do it here, but window.resize_renderer()
                        // calls window.size(), so we need to set the new_inner_size before winit.
                        if let Some(w) = &window.window {
                            w.set_inner_size(**new_inner_size);
                        }
                        ctx.updates.layout();
                        window.expect_layout_update();
                        window.resize_renderer();
                    }

                    WindowScaleChangedEvent::notify(ctx.events, WindowScaleChangedArgs::now(window_id, scale_factor, new_size));
                }
            }
            _ => {}
        }
    }

    fn event_ui<EV: EventUpdateArgs>(&mut self, ctx: &mut AppContext, args: &EV) {
        let wn_ctxs: Vec<_> = ctx
            .services
            .req::<Windows>()
            .windows
            .iter_mut()
            .map(|w| w.context.clone())
            .collect();

        for wn_ctx in wn_ctxs {
            wn_ctx.borrow_mut().event(ctx, args);
        }
    }

    fn update_ui(&mut self, ctx: &mut AppContext) {
        self.update_open_close(ctx);
        self.update_pump(ctx);
    }

    fn event<EV: EventUpdateArgs>(&mut self, ctx: &mut AppContext, args: &EV) {
        if let Some(args) = WindowCloseRequestedEvent::update(args) {
            self.update_closing(ctx, args);
        } else if let Some(args) = WindowCloseEvent::update(args) {
            self.update_close(ctx, args);
        }
    }

    fn update_display(&mut self, ctx: &mut AppContext, _: UpdateDisplayRequest) {
        // Pump layout and render in all windows.
        // The windows don't do a layout update unless they recorded
        // an update request for layout or render.

        // we need to detach the windows from the ctx, because the window needs it
        // to create a layout context. Services are not visible in the layout context
        // so this is fine. // TODO: REVIEW
        let (mut windows, mut opening) = {
            let wns = ctx.services.req::<Windows>();
            (mem::take(&mut wns.windows), mem::take(&mut wns.opening_windows))
        };
        for window in windows.iter_mut().chain(&mut opening) {
            window.layout(ctx);
            window.render(ctx);
            window.render_update(ctx);
        }

        let wns = ctx.services.req::<Windows>();
        wns.windows = windows;
        wns.opening_windows = opening;
    }

    fn new_frame_ready(&mut self, ctx: &mut AppContext, window_id: WindowId) {
        let wns = ctx.services.req::<Windows>();
        if let Some(window) = wns.windows.iter_mut().find(|w| w.id == window_id) {
            window.request_redraw(ctx.vars);
        } else if let Some(idx) = wns.opening_windows.iter().position(|w| w.id == window_id) {
            let mut window = wns.opening_windows.remove(idx);
            window.request_redraw(ctx.vars);

            debug_assert!(matches!(window.init_state, WindowInitState::Inited));

            let args = WindowEventArgs::now(window.id, true);
            window.open_response.take().unwrap().respond(ctx.vars, args.clone());
            WindowOpenEvent::notify(ctx.events, args);
            wns.windows.push(window);
        }
    }

    fn redraw_requested(&mut self, ctx: &mut AppContext, window_id: WindowId) {
        if let Some(window) = ctx.services.req::<Windows>().windows.iter_mut().find(|w| w.id == window_id) {
            window.redraw();
        }
    }

    fn shutdown_requested(&mut self, ctx: &mut AppContext, args: &ShutdownRequestedArgs) {
        if !args.cancel_requested() {
            let service = ctx.services.req::<Windows>();
            if service.shutdown_on_last_close {
                let windows: Vec<WindowId> = service.windows.iter().map(|w| w.id).collect();
                if !windows.is_empty() {
                    args.cancel();
                    service.close_together(windows).unwrap();
                }
            }
        }
    }

    fn deinit(&mut self, ctx: &mut AppContext) {
        let windows = mem::take(&mut ctx.services.req::<Windows>().windows);
        for window in windows {
            {
                log::error!(
                    target: "window",
                    "dropping `{:?} ({})` without closing events",
                    window.id,
                    window.vars.title().get(ctx.vars)
                );
                window.context.borrow_mut().deinit(ctx);
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
                request.force_headless,
                request.responder,
                ctx,
                ctx.event_loop,
                self.event_loop_proxy.as_ref().unwrap().clone(),
                Arc::clone(&self.ui_threads),
                ctx.updates.sender().clone(),
            );
            ctx.services.req::<Windows>().opening_windows.push(w);
        }

        for window_id in close {
            WindowCloseRequestedEvent::notify(ctx.events, WindowCloseRequestedArgs::now(window_id));
        }
    }

    /// Pump the requested update methods.
    fn update_pump(&mut self, ctx: &mut AppContext) {
        // detach context part so we can let a window content access its own window.
        let wn_ctxs: Vec<_> = ctx
            .services
            .req::<Windows>()
            .windows
            .iter_mut()
            .map(|w| w.context.clone())
            .collect();

        for wn_ctx in &wn_ctxs {
            wn_ctx.borrow_mut().update(ctx);
        }

        // do window vars update.
        let mut windows = mem::take(&mut ctx.services.req::<Windows>().windows);
        for window in windows.iter_mut() {
            window.update_window(ctx);
        }
        ctx.services.req::<Windows>().windows = windows;

        // do preload updates.
        let mut opening = mem::take(&mut ctx.services.req::<Windows>().opening_windows);
        for window in &mut opening {
            debug_assert!(!matches!(window.init_state, WindowInitState::Inited));
            window.preload_update_window(ctx);
        }
        ctx.services.req::<Windows>().opening_windows = opening;
    }

    /// Respond to window_closing events.
    fn update_closing(&mut self, ctx: &mut AppContext, args: &WindowCloseRequestedArgs) {
        let wins = ctx.services.req::<Windows>();
        if let Ok(win) = wins.window(args.window_id) {
            if args.cancel_requested() {
                let responder = win.close_response.borrow_mut().take().unwrap();
                // cancel, if is `close_together`, this sets cancel for all
                // windows in the group, because they share the same responder.
                responder.respond(ctx.vars, CloseWindowResult::Cancel);
                win.close_canceled.borrow().set(true);
            } else if win.close_canceled.borrow().get() {
                // another window in `close_together` canceled.
                let _ = win.close_response.borrow_mut().take();
            } else {
                // close was success.
                WindowCloseEvent::notify(ctx.events, WindowEventArgs::now(args.window_id, false));
                let responder = win.close_response.borrow_mut().take().unwrap();
                responder.respond(ctx.vars, CloseWindowResult::Close);
            }
        }
    }

    /// Respond to window_close events.
    fn update_close(&mut self, ctx: &mut AppContext, args: &WindowEventArgs) {
        // remove the window.
        let window = {
            let wns = ctx.services.req::<Windows>();
            wns.windows
                .iter()
                .position(|w| w.id == args.window_id)
                .map(|idx| wns.windows.remove(idx))
        };

        // deinit and notify lost of focus.
        if let Some(w) = window {
            w.context.clone().borrow_mut().deinit(ctx);
            if w.is_focused {
                let args = WindowIsFocusedArgs::now(w.id, false, true);
                self.notify_focus(args, ctx.events);
            }
        }

        // does shutdown_on_last_close.
        let service = ctx.services.req::<Windows>();
        if service.shutdown_on_last_close && service.windows.is_empty() && service.opening_windows.is_empty() {
            ctx.services.req::<AppProcess>().shutdown();
        }
    }

    fn notify_focus(&self, args: WindowIsFocusedArgs, events: &mut Events) {
        debug_assert!(!args.closed || (args.closed && !args.focused));

        WindowFocusChangedEvent::notify(events, args.clone());
        if args.focused {
        } else {
            WindowBlurEvent::notify(events, args);
        }
    }
}

/// Windows service.
#[derive(Service)]
pub struct Windows {
    /// If shutdown is requested when a window closes and there are no more windows open, `true` by default.
    pub shutdown_on_last_close: bool,

    windows: Vec<OpenWindow>,

    open_requests: Vec<OpenWindowRequest>,
    opening_windows: Vec<OpenWindow>,
    update_sender: UpdateSender,
}

impl Windows {
    fn new(update_sender: UpdateSender) -> Self {
        Windows {
            shutdown_on_last_close: true,
            windows: Vec::with_capacity(1),
            open_requests: Vec::with_capacity(1),
            opening_windows: Vec::with_capacity(1),
            update_sender,
        }
    }

    /// Requests a new window.
    ///
    /// The `new_window` argument is the [`WindowContext`] of the new window.
    ///
    /// The `force_headless` argument can be used to create a headless window in a headed app.
    ///
    /// Returns a listener that will update once when the window is opened, note that while the `window_id` is
    /// available in the `new_window` argument already, the window is only available in this service after
    /// the returned listener updates.
    pub fn open(
        &mut self,
        new_window: impl FnOnce(&mut WindowContext) -> Window + 'static,
        force_headless: Option<WindowMode>,
    ) -> ResponseVar<WindowEventArgs> {
        let (responder, response) = response_var();
        let request = OpenWindowRequest {
            new: Box::new(new_window),
            force_headless,
            responder,
        };
        self.open_requests.push(request);
        let _ = self.update_sender.send();

        response
    }

    /// Starts closing a window, the operation can be canceled by listeners of the
    /// [close requested event](WindowCloseRequestedEvent).
    ///
    /// Returns a response var that will update once with the result of the operation.
    pub fn close(&mut self, window_id: WindowId) -> Result<ResponseVar<CloseWindowResult>, GetWindowError> {
        if let Some(w) = self.windows.iter().find(|w| w.id == window_id) {
            Ok(w.close())
        } else {
            Err(self.get_window_error(window_id))
        }
    }

    fn get_window_error(&self, window_id: WindowId) -> GetWindowError {
        if let Some(w) = self.opening_windows.iter().find(|w| w.id == window_id) {
            GetWindowError::Opening(window_id, w.open_response.as_ref().unwrap().response_var())
        } else {
            GetWindowError::NotFound(window_id)
        }
    }

    /// Requests closing multiple windows together, the operation can be canceled by listeners of the
    /// [close requested event](WindowCloseRequestedEvent). If canceled none of the windows are closed.
    ///
    /// Returns a response var that will update once with the result of the operation. Returns
    /// [`Cancel`](CloseWindowResult::Cancel) if `windows` is empty or contains a window that already
    /// requested close during this update.
    pub fn close_together(
        &mut self,
        windows: impl IntoIterator<Item = WindowId>,
    ) -> Result<ResponseVar<CloseWindowResult>, GetWindowError> {
        let windows = windows.into_iter();
        let mut all = Vec::with_capacity(windows.size_hint().0);
        for window_id in windows {
            all.push(
                self.windows
                    .iter()
                    .find(|w| w.id == window_id)
                    .ok_or_else(|| self.get_window_error(window_id))?,
            );
        }
        if all.is_empty() || all.iter().any(|a| a.close_response.borrow().is_some()) {
            return Ok(response_done_var(CloseWindowResult::Cancel));
        }

        let (group_responder, response) = response_var();
        let group_cancel = Rc::default();

        for window in all {
            *window.close_response.borrow_mut() = Some(group_responder.clone());
            *window.close_canceled.borrow_mut() = Rc::clone(&group_cancel);
        }

        Ok(response)
    }

    /// Reference an open window.
    #[inline]
    pub fn window(&self, window_id: WindowId) -> Result<&OpenWindow, GetWindowError> {
        self.windows
            .iter()
            .find(|w| w.id == window_id)
            .ok_or_else(|| self.get_window_error(window_id))
    }

    /// All open windows.
    #[inline]
    pub fn windows(&self) -> &[OpenWindow] {
        &self.windows
    }

    fn take_requests(&mut self) -> (Vec<OpenWindowRequest>, Vec<WindowId>) {
        let mut close_requests = vec![];
        for w in self.windows.iter() {
            if w.close_response.borrow().is_some() {
                close_requests.push(w.id);
            }
        }
        (mem::take(&mut self.open_requests), close_requests)
    }
}

struct OpenWindowRequest {
    new: Box<dyn FnOnce(&mut WindowContext) -> Window>,
    force_headless: Option<WindowMode>,
    responder: ResponderVar<WindowEventArgs>,
}

/// Response message of [`close`](Windows::close) and [`close_together`](Windows::close_together).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CloseWindowResult {
    /// Operation completed, all requested windows closed.
    Close,

    /// Operation canceled, no window closed.
    Cancel,
}

/// Error when searching for an open window.
pub enum GetWindowError {
    /// Window not found, it is not open and not opening.
    NotFound(WindowId),
    /// Window is not available because it is still opening.
    ///
    /// The associated values are the requested window ID and a response var that will update once when
    /// the window finishes opening.
    ///
    /// **Note:** The window initial content is inited, updated, layout and rendered once before the window is open.
    Opening(WindowId, ResponseVar<WindowEventArgs>),
}
impl fmt::Debug for GetWindowError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GetWindowError::NotFound(id) => f.debug_tuple("NotFound").field(&id).finish(),
            GetWindowError::Opening(id, _) => f.debug_tuple("Opening").field(&id).finish(),
        }
    }
}
impl fmt::Display for GetWindowError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            GetWindowError::NotFound(id) => {
                write!(f, "window `{}` is not opened in `Windows` service", id)
            }
            GetWindowError::Opening(id, _) => {
                write!(f, "window `{}` not available because it is still opening in `Windows` service", id)
            }
        }
    }
}
impl std::error::Error for GetWindowError {}

// We don't use Rc<dyn ..> because of this issue: https://github.com/rust-lang/rust/issues/69757
type RenderIcon = Rc<Box<dyn Fn(&mut WindowContext) -> BoxedUiNode>>;

/// Window icon.
#[derive(Clone)]
pub enum WindowIcon {
    /// Operating system default icon.
    ///
    /// In Windows this is the icon associated with the executable.
    Default,
    /// A bitmap icon.
    ///
    /// Use the [`from_rgba`](Self::from_rgba), [`from_bytes`](Self::from_bytes) or [`from_file`](Self::from_file) functions to initialize.
    Icon(Rc<glutin::window::Icon>),
    /// An [`UiNode`] that draws the icon.
    ///
    /// Use the [`render`](Self::render) function to initialize.
    Render(RenderIcon),
}
impl fmt::Debug for WindowIcon {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "WindowIcon::")?;
        }
        match self {
            WindowIcon::Default => write!(f, "Default"),
            WindowIcon::Icon(_) => write!(f, "Icon"),
            WindowIcon::Render(_) => write!(f, "Render"),
        }
    }
}
impl PartialEq for WindowIcon {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (WindowIcon::Default, WindowIcon::Default) => true,
            (WindowIcon::Icon(a), WindowIcon::Icon(b)) => Rc::ptr_eq(a, b),
            (WindowIcon::Render(a), WindowIcon::Render(b)) => Rc::ptr_eq(a, b),
            _ => false,
        }
    }
}
impl Default for WindowIcon {
    /// [`WindowIcon::Default`]
    fn default() -> Self {
        Self::Default
    }
}
impl WindowIcon {
    /// New window icon from 32bpp RGBA data.
    ///
    /// The `rgba` must be a sequence of RGBA pixels in top-to-bottom rows.
    #[inline]
    pub fn from_rgba(rgba: Vec<u8>, width: u32, height: u32) -> Result<Self, glutin::window::BadIcon> {
        let icon = glutin::window::Icon::from_rgba(rgba, width, height)?;
        Ok(Self::Icon(Rc::new(icon)))
    }

    /// New window icon from the bytes of an encoded image.
    ///
    /// The image format is guessed, PNG is recommended. Loading is done using the [`image::load_from_memory`]
    /// function from the [`image`] crate.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, image::error::ImageError> {
        use image::GenericImageView;

        let image = image::load_from_memory(bytes)?;
        let (width, height) = image.dimensions();
        let icon = Self::from_rgba(image.into_bytes(), width, height).expect("image loaded a BadIcon from memory");
        Ok(icon)
    }

    /// New window icon from image file.
    ///
    /// The image format is guessed from the path extension. Loading is done using the [`image::open`]
    /// function from the [`image`] crate.
    pub fn from_file<P: AsRef<std::path::Path>>(file: P) -> Result<Self, image::error::ImageError> {
        use image::GenericImageView;

        let image = image::open(file)?;
        let (width, height) = image.dimensions();
        let icon = Self::from_rgba(image.into_bytes(), width, height).expect("image loaded a BadIcon from file");
        Ok(icon)
    }

    /// New window icon from a function that generates a new icon [`UiNode`] for the window.
    ///
    /// The function is called once on init and every time the window icon property changes,
    /// the input is the window context, the result is a node that is rendered to create an icon.
    ///
    /// The icon node is updated like any other node and it can request a new render, you should
    /// limit the interval for new frames,
    pub fn render<I: UiNode, F: Fn(&mut WindowContext) -> I + 'static>(new_icon: F) -> Self {
        Self::Render(Rc::new(Box::new(move |ctx| new_icon(ctx).boxed())))
    }
}
impl_from_and_into_var! {
    /// [`WindowIcon::from_bytes`]
    fn from(bytes: &'static [u8]) -> WindowIcon {
        WindowIcon::from_bytes(bytes).unwrap_or_else(|e| {
            log::error!(target: "window", "failed to load icon from encoded bytes: {:?}", e);
            WindowIcon::Default
        })
    }

    /// [`WindowIcon::from_rgba`]
    fn from(rgba: (Vec<u8>, u32, u32)) -> WindowIcon {
        WindowIcon::from_rgba(rgba.0, rgba.1, rgba.2).unwrap_or_else(|e| {
            log::error!(target: "window", "failed to load icon from RGBA data: {:?}", e);
            WindowIcon::Default
        })
    }

    /// [`WindowIcon::from_file`]
    fn from(file: &'static str) -> WindowIcon {
        WindowIcon::from_file(file).unwrap_or_else(|e| {
            log::error!(target: "window", "failed to load icon from file: {:?}", e);
            WindowIcon::Default
        })
    }

    /// [`WindowIcon::from_file`]
    fn from(file: std::path::PathBuf) -> WindowIcon {
        WindowIcon::from_file(file).unwrap_or_else(|e| {
            log::error!(target: "window", "failed to load icon from file: {:?}", e);
            WindowIcon::Default
        })
    }
}
impl<const N: usize> From<&'static [u8; N]> for WindowIcon {
    /// [`WindowIcon::from_file`]
    fn from(bytes: &'static [u8; N]) -> Self {
        Self::from_bytes(bytes).unwrap_or_else(|e| {
            log::error!(target: "window", "failed to load icon from encoded bytes: {:?}", e);
            WindowIcon::Default
        })
    }
}
impl<const N: usize> crate::var::IntoVar<WindowIcon> for &'static [u8; N] {
    type Var = crate::var::OwnedVar<WindowIcon>;

    /// [`WindowIcon::from_file`]
    fn into_var(self) -> Self::Var {
        crate::var::OwnedVar(self.into())
    }
}

/// Window chrome, the non-client area of the window.
#[derive(Clone, PartialEq)]
pub enum WindowChrome {
    /// Operating system chrome.
    Default,
    /// Chromeless.
    None,
    /// An [`UiNode`] that provides the window chrome.
    Custom,
}
impl fmt::Debug for WindowChrome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "WindowChrome::")?;
        }
        match self {
            WindowChrome::Default => write!(f, "Default"),
            WindowChrome::None => write!(f, "None"),
            WindowChrome::Custom => write!(f, "Custom"),
        }
    }
}
impl WindowChrome {
    /// Is operating system chrome.
    #[inline]
    fn is_default(&self) -> bool {
        matches!(self, WindowChrome::Default)
    }
}
impl Default for WindowChrome {
    /// [`WindowChrome::Default`]
    fn default() -> Self {
        Self::Default
    }
}
impl_from_and_into_var! {
    /// | Input  | Output                  |
    /// |--------|-------------------------|
    /// |`true`  | `WindowChrome::Default` |
    /// |`false` | `WindowChrome::None`    |
    fn from(default_: bool) -> WindowChrome {
        if default_ {
            WindowChrome::Default
        } else {
            WindowChrome::None
        }
    }
}

/// Window screen state.
#[derive(Clone, Copy, PartialEq)]
pub enum WindowState {
    /// A visible window, at the `position` and `size` configured.
    Normal,
    /// Window not visible, but maybe visible in the taskbar.
    Minimized,
    /// Window fills the screen, but window frame and taskbar are visible.
    Maximized,
    /// Window fully fills the screen, rendered using a frameless top-most window.
    Fullscreen,
    /// Exclusive video access to the monitor, only the window content is visible. TODO video config
    FullscreenExclusive,
}
impl fmt::Debug for WindowState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "WindowState::")?;
        }
        match self {
            WindowState::Normal => write!(f, "Normal"),
            WindowState::Minimized => write!(f, "Minimized"),
            WindowState::Maximized => write!(f, "Maximized"),
            WindowState::Fullscreen => write!(f, "Fullscreen"),
            WindowState::FullscreenExclusive => write!(f, "FullscreenExclusive"),
        }
    }
}

bitflags! {
    /// Mask of allowed [`WindowState`] states of a window.
    pub struct WindowStateAllowed: u8 {
        /// Enable minimize.
        const MINIMIZE = 0b0001;
        /// Enable maximize.
        const MAXIMIZE = 0b0010;
        /// Enable full-screen, but only windowed not exclusive video.
        const FULLSCREEN_WN_ONLY = 0b0100;
        /// Allow full-screen windowed or exclusive video.
        const FULLSCREEN = 0b1100;
    }
}

struct WindowVarsData {
    chrome: RcVar<WindowChrome>,
    icon: RcVar<WindowIcon>,
    title: RcVar<Text>,

    state: RcVar<WindowState>,

    position: RcVar<Point>,

    size: RcVar<Size>,
    auto_size: RcVar<AutoSize>,
    min_size: RcVar<Size>,
    max_size: RcVar<Size>,

    resizable: RcVar<bool>,
    movable: RcVar<bool>,

    always_on_top: RcVar<bool>,

    visible: RcVar<bool>,
    taskbar_visible: RcVar<bool>,

    parent: RcVar<Option<WindowId>>,
    modal: RcVar<bool>,

    transparent: RcVar<bool>,
}

/// Controls properties of an open window using variables.
///
/// You can get the controller for any window using [`OpenWindow::vars`].
///
/// You can get the controller for the current context window by getting `WindowVars` from the `window_state`
/// in [`WindowContext`](WindowContext::window_state) and [`WidgetContext`](WidgetContext::window_state).
pub struct WindowVars {
    vars: Rc<WindowVarsData>,
}
impl WindowVars {
    fn new() -> Self {
        let vars = Rc::new(WindowVarsData {
            chrome: var(WindowChrome::Default),
            icon: var(WindowIcon::Default),
            title: var("".to_text()),

            state: var(WindowState::Normal),

            position: var(Point::new(f32::NAN, f32::NAN)),
            size: var(Size::new(f32::NAN, f32::NAN)),

            min_size: var(Size::new(192.0, 48.0)),
            max_size: var(Size::new(100.pct(), 100.pct())),
            auto_size: var(AutoSize::empty()),

            resizable: var(true),
            movable: var(true),

            always_on_top: var(false),

            visible: var(true),
            taskbar_visible: var(true),

            parent: var(None),
            modal: var(false),

            transparent: var(false),
        });
        Self { vars }
    }

    /// Update all variables with the same value.
    fn refresh_all(&self, vars: &crate::var::Vars) {
        self.chrome().touch(vars);
        self.icon().touch(vars);
        self.title().touch(vars);
        self.state().touch(vars);
        self.position().touch(vars);
        self.size().touch(vars);
        self.min_size().touch(vars);
        self.max_size().touch(vars);
        self.auto_size().touch(vars);
        self.resizable().touch(vars);
        self.movable().touch(vars);
        self.always_on_top().touch(vars);
        self.visible().touch(vars);
        self.taskbar_visible().touch(vars);
        self.parent().touch(vars);
        self.modal().touch(vars);
        self.transparent().touch(vars);
    }

    fn clone(&self) -> Self {
        Self {
            vars: Rc::clone(&self.vars),
        }
    }

    /// Window chrome, the non-client area of the window.
    ///
    /// See [`WindowChrome`] for details.
    ///
    /// The default value is [`WindowChrome::Default`].
    #[inline]
    pub fn chrome(&self) -> &RcVar<WindowChrome> {
        &self.vars.chrome
    }

    /// If the window is see-through.
    ///
    /// The default value is `false`.
    #[inline]
    pub fn transparent(&self) -> &RcVar<bool> {
        &self.vars.transparent
    }

    /// Window icon.
    ///
    /// See [`WindowIcon`] for details.
    ///
    /// The default value is [`WindowIcon::Default`].
    #[inline]
    pub fn icon(&self) -> &RcVar<WindowIcon> {
        &self.vars.icon
    }

    /// Window title text.
    ///
    /// The default value is `""`.
    #[inline]
    pub fn title(&self) -> &RcVar<Text> {
        &self.vars.title
    }

    /// Window screen state.
    ///
    /// Minimized, maximized or full-screen. See [`WindowState`] for details.
    ///
    /// The default value is [`WindowState::Normal`]
    #[inline]
    pub fn state(&self) -> &RcVar<WindowState> {
        &self.vars.state
    }

    /// Window top-left offset on the screen.
    ///
    /// When a dimension is not a finite value it is computed from other variables.
    /// Relative values are computed in relation to the full-screen size.
    ///
    /// When the the window is moved this variable is updated back.
    ///
    /// The default value is `(f32::NAN, f32::NAN)`.
    #[inline]
    pub fn position(&self) -> &RcVar<Point> {
        &self.vars.position
    }

    /// Window width and height on the screen.
    ///
    /// When a dimension is not a finite value it is computed from other variables.
    /// Relative values are computed in relation to the full-screen size.
    ///
    /// When the window is resized this variable is updated back.
    ///
    /// The default value is `(f32::NAN, f32::NAN)`.
    #[inline]
    pub fn size(&self) -> &RcVar<Size> {
        &self.vars.size
    }

    /// Configure window size-to-content.
    ///
    /// When enabled overwrites [`size`](Self::size), but is still coerced by [`min_size`](Self::min_size)
    /// and [`max_size`](Self::max_size). Auto-size is disabled if the user [manually resizes](Self::resizable).
    ///
    /// The default value is [`AutoSize::DISABLED`].
    #[inline]
    pub fn auto_size(&self) -> &RcVar<AutoSize> {
        &self.vars.auto_size
    }

    /// Minimal window width and height.
    ///
    /// When a dimension is not a finite value it fallback to the previous valid value.
    /// Relative values are computed in relation to the full-screen size.
    ///
    /// Note that the operation systems can have their own minimal size that supersedes this variable.
    ///
    /// The default value is `(192, 48)`.
    #[inline]
    pub fn min_size(&self) -> &RcVar<Size> {
        &self.vars.min_size
    }

    /// Maximal window width and height.
    ///
    /// When a dimension is not a finite value it fallback to the previous valid value.
    /// Relative values are computed in relation to the full-screen size.
    ///
    /// Note that the operation systems can have their own maximal size that supersedes this variable.
    ///
    /// The default value is `(100.pct(), 100.pct())`
    #[inline]
    pub fn max_size(&self) -> &RcVar<Size> {
        &self.vars.max_size
    }

    /// If the user can resize the window using the window frame.
    ///
    /// Note that even if disabled the window can still be resized from other sources.
    ///
    /// The default value is `true`.
    #[inline]
    pub fn resizable(&self) -> &RcVar<bool> {
        &self.vars.resizable
    }

    /// If the user can move the window using the window frame.
    ///
    /// Note that even if disabled the window can still be moved from other sources.
    ///
    /// The default value is `true`.
    #[inline]
    pub fn movable(&self) -> &RcVar<bool> {
        &self.vars.movable
    }

    /// Whether the window should always stay on top of other windows.
    ///
    /// Note this only applies to other windows that are not also "always-on-top".
    ///
    /// The default value is `false`.
    #[inline]
    pub fn always_on_top(&self) -> &RcVar<bool> {
        &self.vars.always_on_top
    }

    /// If the window is visible on the screen and in the task-bar.
    ///
    /// This variable is observed only after the first frame render, before that the window
    /// is always not visible.
    ///
    /// The default value is `true`.
    #[inline]
    pub fn visible(&self) -> &RcVar<bool> {
        &self.vars.visible
    }

    /// If the window is visible in the task-bar.
    ///
    /// The default value is `true`.
    #[inline]
    pub fn taskbar_visible(&self) -> &RcVar<bool> {
        &self.vars.taskbar_visible
    }

    /// The window parent.
    ///
    /// If a parent is set this behavior applies:
    ///
    /// * If the parent is minimized, this window is also minimized.
    /// * If the parent window is maximized, this window is restored.
    /// * This window is always-on-top of the parent window.
    /// * If the parent window is closed, this window is also closed.
    /// * If [`modal`](Self::modal) is set, the parent window cannot be focused while this window is open.
    ///
    /// The default value is `None`.
    #[inline]
    pub fn parent(&self) -> &RcVar<Option<WindowId>> {
        &self.vars.parent
    }

    /// Configure the [`parent`](Self::parent) connection.
    ///
    /// Value is ignored is `parent` is not set.
    ///
    /// The default value is `false`.
    #[inline]
    pub fn modal(&self) -> &RcVar<bool> {
        &self.vars.modal
    }
}
impl StateKey for WindowVars {
    type Type = Self;
}

/// Arguments for `on_pre_redraw` and `on_redraw`.
pub struct RedrawArgs<'a> {
    renderer: &'a mut Renderer,
    close: bool,
}
impl<'a> RedrawArgs<'a> {
    fn new(renderer: &'a mut Renderer) -> Self {
        RedrawArgs { renderer, close: false }
    }

    /// Read the current presented frame into a [`FramePixels`].
    #[inline]
    pub fn frame_pixels(&mut self) -> Result<FramePixels, crate::render::RendererError> {
        self.renderer.frame_pixels()
    }

    /// Request window close.
    #[inline]
    pub fn close(&mut self) {
        self.close = true;
    }

    // TODO methods
}

/// Window startup configuration.
///
/// More window configuration is accessible using the [`WindowVars`] type.
pub struct Window {
    state: OwnedStateMap,
    id: WidgetId,
    start_position: StartPosition,
    kiosk: bool,
    headless_screen: HeadlessScreen,
    on_pre_redraw: Box<dyn FnMut(&mut RedrawArgs)>,
    on_redraw: Box<dyn FnMut(&mut RedrawArgs)>,
    child: BoxedUiNode,
}
impl Window {
    /// New window configuration.
    ///
    /// * `root_id` - Widget ID of `child`.
    /// * `start_position` - Position of the window when it first opens.
    /// * `kiosk` - Only allow full-screen mode. Note this does not configure the operating system, only blocks the app itself
    ///             from accidentally exiting full-screen. Also causes subsequent open windows to be child of this window.
    /// * `mode` - Custom window mode for this window only, set to default to use the app mode.
    /// * `headless_screen` - "Screen" configuration used in [headless mode](WindowMode::is_headless).
    /// * `on_pre_redraw`  - Event called just before a frame redraw.
    /// * `on_redraw`  - Event called just after a frame redraw.
    /// * `child` - The root widget outermost node, the window sets-up the root widget using this and the `root_id`.
    #[allow(clippy::clippy::too_many_arguments)]
    pub fn new(
        root_id: WidgetId,
        start_position: impl Into<StartPosition>,
        kiosk: bool,
        headless_screen: impl Into<HeadlessScreen>,
        on_pre_redraw: Box<dyn FnMut(&mut RedrawArgs)>,
        on_redraw: Box<dyn FnMut(&mut RedrawArgs)>,
        child: impl UiNode,
    ) -> Self {
        Window {
            state: OwnedStateMap::default(),
            id: root_id,
            kiosk,
            start_position: start_position.into(),
            headless_screen: headless_screen.into(),
            on_pre_redraw,
            on_redraw,
            child: child.boxed(),
        }
    }
}

/// "Screen" configuration used by windows in [headless mode](WindowMode::is_headless).
#[derive(Clone)]
pub struct HeadlessScreen {
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
impl fmt::Debug for HeadlessScreen {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            f.debug_struct("HeadlessScreen")
                .field("scale_factor", &self.scale_factor)
                .field("screen_size", &self.screen_size)
                .finish()
        } else {
            write!(
                f,
                "({}, ({}, {}))",
                self.scale_factor, self.screen_size.width, self.screen_size.height
            )
        }
    }
}
impl HeadlessScreen {
    /// New with custom size at `1.0` scale.
    #[inline]
    pub fn new(screen_size: LayoutSize) -> Self {
        Self::new_scaled(screen_size, 1.0)
    }

    /// New with custom size and scale.
    #[inline]
    pub fn new_scaled(screen_size: LayoutSize, scale_factor: f32) -> Self {
        HeadlessScreen { scale_factor, screen_size }
    }

    /// New default size `1920x1080` and custom scale.
    #[inline]
    pub fn new_scale(scale_factor: f32) -> Self {
        HeadlessScreen {
            scale_factor,
            ..Self::default()
        }
    }
}
impl Default for HeadlessScreen {
    /// New `1920x1080` at `1.0` scale.
    fn default() -> Self {
        Self::new(LayoutSize::new(1920.0, 1080.0))
    }
}
impl IntoValue<HeadlessScreen> for (f32, f32) {}
impl From<(f32, f32)> for HeadlessScreen {
    /// Calls [`HeadlessScreen::new_scaled`]
    fn from((width, height): (f32, f32)) -> Self {
        Self::new(LayoutSize::new(width, height))
    }
}
impl IntoValue<HeadlessScreen> for (u32, u32) {}
impl From<(u32, u32)> for HeadlessScreen {
    /// Calls [`HeadlessScreen::new`]
    fn from((width, height): (u32, u32)) -> Self {
        Self::new(LayoutSize::new(width as f32, height as f32))
    }
}
impl IntoValue<HeadlessScreen> for FactorNormal {}
impl From<FactorNormal> for HeadlessScreen {
    /// Calls [`HeadlessScreen::new_scale`]
    fn from(f: FactorNormal) -> Self {
        Self::new_scale(f.0)
    }
}
impl IntoValue<HeadlessScreen> for FactorPercent {}
impl From<FactorPercent> for HeadlessScreen {
    /// Calls [`HeadlessScreen::new_scale`]
    fn from(f: FactorPercent) -> Self {
        Self::new_scale(f.0 / 100.0)
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
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum StartPosition {
    /// Uses the value of the `position` property.
    Default,
    /// Centralizes the window in the monitor screen.
    CenterScreen,
    /// Centralizes the window the parent window.
    CenterParent,
}
impl Default for StartPosition {
    fn default() -> Self {
        Self::Default
    }
}
impl fmt::Debug for StartPosition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "StartPosition::")?;
        }
        match self {
            StartPosition::Default => write!(f, "Default"),
            StartPosition::CenterScreen => write!(f, "CenterScreen"),
            StartPosition::CenterParent => write!(f, "CenterParent"),
        }
    }
}

/// Mode of an [`OpenWindow`].
#[derive(Clone, Copy, PartialEq, Eq)]
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
impl fmt::Debug for WindowMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "WindowMode::")?;
        }
        match self {
            WindowMode::Headed => write!(f, "Headed"),
            WindowMode::Headless => write!(f, "Headless"),
            WindowMode::HeadlessWithRenderer => write!(f, "HeadlessWithRenderer"),
        }
    }
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

#[derive(Clone, Copy)]
enum WindowInitState {
    /// Window not visible, awaiting first call to `OpenWindow::preload_update`.
    New,
    /// Content `UiNode::init` called, next calls to `OpenWindow::preload_update` will do updates
    /// until the first layout and render.
    ContentInited,
    /// First frame rendered and presented, window `visible`synched with var, the window
    /// is fully launched.
    Inited,
}

/// An open window.
pub struct OpenWindow {
    context: Rc<RefCell<OwnedWindowContext>>,

    window: Option<glutin::window::Window>,
    renderer: Option<RefCell<Renderer>>,

    vars: WindowVars,

    mode: WindowMode,
    id: WindowId,
    root_id: WidgetId,

    kiosk: bool,

    init_state: WindowInitState,

    frame_info: FrameInfo,

    min_size: LayoutSize,
    max_size: LayoutSize,

    is_focused: bool,

    #[cfg(windows)]
    subclass_id: std::cell::Cell<usize>,

    headless_screen: HeadlessScreen,
    headless_position: LayoutPoint,
    headless_size: LayoutSize,
    headless_state: WindowState,
    taskbar_visible: bool,

    renderless_event_sender: Option<EventLoopProxy>,

    open_response: Option<ResponderVar<WindowEventArgs>>,
    close_response: RefCell<Option<ResponderVar<CloseWindowResult>>>,
    close_canceled: RefCell<Rc<Cell<bool>>>,
    update_sender: UpdateSender,
}
impl OpenWindow {
    #[allow(clippy::too_many_arguments)]
    fn new(
        new_window: Box<dyn FnOnce(&mut WindowContext) -> Window>,
        force_headless: Option<WindowMode>,
        open_response: ResponderVar<WindowEventArgs>,
        ctx: &mut AppContext,
        event_loop: EventLoopWindowTarget,
        event_loop_proxy: EventLoopProxy,
        ui_threads: Arc<ThreadPool>,
        update_sender: UpdateSender,
    ) -> Self {
        // get mode.
        let mut mode = if let Some(headless) = ctx.headless.state() {
            if headless.get::<app::HeadlessRendererEnabledKey>().copied().unwrap_or_default() {
                WindowMode::HeadlessWithRenderer
            } else {
                WindowMode::Headless
            }
        } else {
            WindowMode::Headed
        };
        if let Some(force) = force_headless {
            match force {
                WindowMode::Headed => {
                    log::error!(target: "window", "invalid `WindowMode::Headed` value in `force_headless`");
                }
                WindowMode::Headless => {
                    mode = WindowMode::Headless;
                }
                WindowMode::HeadlessWithRenderer => {
                    if mode.is_headed() {
                        mode = WindowMode::HeadlessWithRenderer;
                    }
                }
            }
        }
        let mode = mode;

        let id;

        let window;
        let renderer;
        let root;
        let api;
        let renderless_event_sender;

        let vars = WindowVars::new();
        let mut wn_state = OwnedStateMap::default();
        wn_state.set_single(vars.clone());

        let renderer_config = RendererConfig {
            clear_color: None,
            workers: Some(ui_threads),
            text_aa: ctx
                .services
                .get::<crate::text::Fonts>()
                .map(|f| f.system_text_aa())
                .unwrap_or(TextAntiAliasing::Subpixel),
        };
        match mode {
            WindowMode::Headed => {
                renderless_event_sender = None;

                let window_ = WindowBuilder::new().with_visible(false); // not visible until first render, to avoid flickering

                let event_loop = event_loop.headed_target().expect("AppContext is not headless but event_loop is");

                let r = Renderer::new_with_glutin(window_, &event_loop, renderer_config, move |args: NewFrameArgs| {
                    let _ = event_loop_proxy.send_event(AppEvent::NewFrameReady(args.window_id.unwrap()));
                })
                .expect("failed to create a window renderer");

                api = Some(Arc::clone(&r.0.api()));
                renderer = Some(RefCell::new(r.0));

                let window_ = r.1;
                id = WindowId::System(window_.id());

                // init window state and services.
                let mut wn_state = OwnedStateMap::default();
                root = ctx.window_context(id, mode, &mut wn_state, &api, new_window).0;

                window = Some(window_);
            }
            headless => {
                window = None;
                renderless_event_sender = Some(event_loop_proxy.clone());

                id = WindowId::new_unique();

                if headless == WindowMode::HeadlessWithRenderer {
                    let rend = Renderer::new(
                        RenderSize::zero(),
                        1.0,
                        renderer_config,
                        move |args: NewFrameArgs| {
                            let _ = event_loop_proxy.send_event(AppEvent::NewFrameReady(args.window_id.unwrap()));
                        },
                        Some(id),
                    )
                    .expect("failed to create a headless renderer");

                    api = Some(Arc::clone(rend.api()));
                    renderer = Some(RefCell::new(rend));
                } else {
                    renderer = None;
                    api = None;
                };

                root = ctx.window_context(id, mode, &mut wn_state, &api, new_window).0;
            }
        }

        let frame_info = FrameInfo::blank(id, root.id);
        let headless_screen = root.headless_screen.clone();
        let kiosk = root.kiosk;
        let root_id = root.id;

        OpenWindow {
            context: Rc::new(RefCell::new(OwnedWindowContext {
                window_id: id,
                mode,
                root_transform_key: WidgetTransformKey::new_unique(),
                state: wn_state,
                root,
                api,
                update: UpdateDisplayRequest::None,
            })),
            window,
            renderer,
            vars,
            id,
            root_id,
            kiosk,
            headless_position: LayoutPoint::zero(),
            headless_size: LayoutSize::new(800.0, 600.0), // same as winit
            headless_state: WindowState::Normal,
            headless_screen,
            taskbar_visible: true,
            mode,
            init_state: WindowInitState::New,
            min_size: LayoutSize::new(192.0, 48.0),
            max_size: LayoutSize::new(f32::INFINITY, f32::INFINITY),
            is_focused: true,
            frame_info,
            renderless_event_sender,

            open_response: Some(open_response),
            close_response: RefCell::default(),
            close_canceled: RefCell::default(),
            update_sender,

            #[cfg(windows)]
            subclass_id: std::cell::Cell::new(0),
        }
    }

    /// Starts closing a window, the operation can be canceled by listeners of the
    /// [close requested event](WindowCloseRequestedEvent).
    ///
    /// Returns a listener that will update once with the result of the operation.
    pub fn close(&self) -> ResponseVar<CloseWindowResult> {
        let mut close_response = self.close_response.borrow_mut();
        if let Some(r) = &*close_response {
            r.response_var()
        } else {
            let (responder, response) = response_var();
            *close_response = Some(responder);
            *self.close_canceled.borrow_mut() = Rc::default();
            let _ = self.update_sender.send();
            response
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

    /// Variables that control this window.
    ///
    /// Also available in the [`window_state`](WindowContext::window_state).
    pub fn vars(&self) -> &WindowVars {
        &self.vars
    }

    /// If the window has the keyboard focus.
    #[inline]
    pub fn is_focused(&self) -> bool {
        self.is_focused
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
            self.headless_screen.scale_factor
        }
    }

    /// Size of the current monitor screen.
    pub fn screen_size(&self) -> LayoutSize {
        if let Some(window) = &self.window {
            let pixel_factor = window.scale_factor() as f32;
            window
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
                })
        } else {
            self.headless_screen.screen_size
        }
    }

    /// Window screen state.
    pub fn state(&self) -> WindowState {
        if let Some(window) = &self.window {
            if let Some(full) = window.fullscreen() {
                match full {
                    glutin::window::Fullscreen::Exclusive(_) => WindowState::FullscreenExclusive,
                    glutin::window::Fullscreen::Borderless(_) => WindowState::Fullscreen,
                }
            } else {
                todo!("other states not available in winit?")
            }
        } else {
            self.headless_state
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

    /// Read the current frame pixels.
    ///
    /// # Panics
    ///
    /// Panics if running in [renderless mode](Self::mode).
    pub fn frame_pixels(&self) -> FramePixels {
        if let Some(renderer) = &self.renderer {
            renderer.borrow_mut().frame_pixels().expect("failed to read pixels")
        } else {
            panic!("cannot screenshot in renderless mode")
        }
    }

    /// Read a rectangle of pixels from the current frame.
    ///
    /// # Panics
    ///
    /// Panics if running in [renderless mode](Self::mode).
    pub fn frame_pixels_rect(&self, rect: LayoutRect) -> FramePixels {
        if let Some(renderer) = &self.renderer {
            renderer.borrow_mut().frame_pixels_l_rect(rect).expect("failed to read pixels")
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

    /// Updated not inited window.
    fn preload_update_window(&mut self, ctx: &mut AppContext) {
        match self.init_state {
            WindowInitState::New => {
                self.context.borrow_mut().init(ctx);
                self.vars.refresh_all(ctx.vars);
                self.init_state = WindowInitState::ContentInited;
            }
            WindowInitState::ContentInited => {
                self.context.borrow_mut().update(ctx);
                self.update_window(ctx);
                ctx.updates.layout();
                self.expect_layout_update();
            }
            WindowInitState::Inited => unreachable!(),
        }
    }

    /// Updated inited window.
    fn update_window(&mut self, ctx: &mut AppContext) {
        if let Some(title) = self.vars.title().get_new(ctx.vars) {
            if let Some(window) = &self.window {
                window.set_title(title);
            }
        }

        if let Some(icon) = self.vars.icon().get_new(ctx.vars) {
            Self::set_icon(&self.window, icon);
        }

        if !self.kiosk {
            if let Some(&auto_size) = self.vars.auto_size().get_new(ctx.vars) {
                // size will be updated in self.layout(..)
                ctx.updates.layout();

                let resizable = auto_size == AutoSize::DISABLED && *self.vars.resizable().get(ctx.vars);
                self.vars.resizable().set_ne(ctx.vars, resizable);

                if let Some(window) = &self.window {
                    window.set_resizable(resizable);
                }
            }

            if let Some(&min_size) = self.vars.min_size().get_new(ctx.vars) {
                let factor = self.scale_factor();
                let prev_min_size = self.min_size;
                let min_size = ctx.outer_layout_context(self.screen_size(), factor, self.id, self.root_id, |ctx| {
                    min_size.to_layout(*ctx.viewport_size, ctx)
                });

                if min_size.width.is_finite() {
                    self.min_size.width = min_size.width;
                }
                if min_size.height.is_finite() {
                    self.min_size.height = min_size.height;
                }
                self.vars.min_size().set_ne(ctx.vars, self.min_size.into());
                if let Some(window) = &self.window {
                    let size =
                        glutin::dpi::PhysicalSize::new((self.min_size.width * factor) as u32, (self.min_size.height * factor) as u32);
                    window.set_min_inner_size(Some(size));
                }

                if prev_min_size != self.min_size {
                    self.expect_layout_update();
                    ctx.updates.layout();
                }
            }

            if let Some(&max_size) = self.vars.max_size().get_new(ctx.vars) {
                let factor = self.scale_factor();
                let prev_max_size = self.max_size;
                let max_size = ctx.outer_layout_context(self.screen_size(), factor, self.id, self.root_id, |ctx| {
                    max_size.to_layout(*ctx.viewport_size, ctx)
                });

                if max_size.width.is_finite() {
                    self.max_size.width = max_size.width;
                }
                if max_size.height.is_finite() {
                    self.max_size.height = max_size.height;
                }
                self.vars.max_size().set_ne(ctx.vars, self.max_size.into());
                if let Some(window) = &self.window {
                    let size =
                        glutin::dpi::PhysicalSize::new((self.max_size.width * factor) as u32, (self.max_size.height * factor) as u32);
                    window.set_max_inner_size(Some(size));
                }

                if prev_max_size != self.max_size {
                    self.expect_layout_update();
                    ctx.updates.layout();
                }
            }

            if let Some(&size) = self.vars.size().get_new(ctx.vars) {
                let current_size = self.size();
                if AutoSize::DISABLED == *self.vars.auto_size().get(ctx.vars) {
                    let factor = self.scale_factor();
                    let mut size = ctx.outer_layout_context(self.screen_size(), factor, self.id, self.root_id, |ctx| {
                        size.to_layout(*ctx.viewport_size, ctx)
                    });

                    if !size.width.is_finite() {
                        size.width = current_size.width;
                    }
                    if !size.height.is_finite() {
                        size.height = current_size.height;
                    }

                    self.vars.size().set_ne(ctx.vars, size.into());
                    if let Some(window) = &self.window {
                        let size = glutin::dpi::PhysicalSize::new((size.width * factor) as u32, (size.height * factor) as u32);
                        window.set_inner_size(size);
                        self.resize_renderer();
                    } else {
                        self.headless_size = size;
                    }
                } else {
                    // cannot change size if auto-sizing.
                    self.vars.size().set_ne(ctx.vars, current_size.into());
                }
            }

            if let Some(&pos) = self.vars.position().get_new(ctx.vars) {
                let factor = self.scale_factor();
                let current_pos = self.position();
                let mut pos = ctx.outer_layout_context(self.screen_size(), factor, self.id, self.root_id, |ctx| {
                    pos.to_layout(*ctx.viewport_size, ctx)
                });

                if !pos.x.is_finite() {
                    pos.x = current_pos.x;
                }
                if !pos.y.is_finite() {
                    pos.y = current_pos.y;
                }

                self.vars.position().set_ne(ctx.vars, pos.into());

                if let Some(window) = &self.window {
                    let pos = glutin::dpi::PhysicalPosition::new((pos.x * factor) as i32, (pos.y * factor) as i32);
                    window.set_outer_position(pos);
                } else {
                    self.headless_position = pos;
                }
            }

            if let Some(&always_on_top) = self.vars.always_on_top().get_new(ctx.vars) {
                if let Some(window) = &self.window {
                    window.set_always_on_top(always_on_top);
                }
            }

            if let Some(&taskbar_visible) = self.vars.taskbar_visible().get_new(ctx.vars) {
                self.set_taskbar_visible(taskbar_visible);
            }

            if let Some(chrome) = self.vars.chrome().get_new(ctx.vars) {
                if let Some(window) = &self.window {
                    window.set_decorations(chrome.is_default());
                }
            }

            if let Some(&visible) = self.vars.visible().get_new(ctx.vars) {
                if let Some(window) = &self.window {
                    window.set_visible(visible && matches!(self.init_state, WindowInitState::Inited));
                }
            }
        } else {
            // kiosk mode
            if let Some(state) = self.vars.state().get_new(ctx.vars) {
                match state {
                    WindowState::Normal | WindowState::Minimized | WindowState::Maximized | WindowState::Fullscreen => {
                        self.vars.state().set_ne(ctx.vars, WindowState::Fullscreen);
                        if let Some(window) = &self.window {
                            window.set_fullscreen(None);
                        } else {
                            self.headless_state = WindowState::Fullscreen;
                        }
                    }
                    WindowState::FullscreenExclusive => {
                        if let Some(window) = &self.window {
                            window.set_fullscreen(None); // TODO
                        } else {
                            self.headless_state = WindowState::FullscreenExclusive;
                        }
                    }
                }
            }
            if self.vars.position().is_new(ctx.vars) {
                self.vars.position().set_ne(ctx.vars, Point::zero());
            }
            if self.vars.auto_size().is_new(ctx.vars) {
                self.vars.auto_size().set_ne(ctx.vars, AutoSize::DISABLED);
            }
            if self.vars.min_size().is_new(ctx.vars) {
                self.vars.min_size().set_ne(ctx.vars, Size::zero());
            }
            if self.vars.max_size().is_new(ctx.vars) {
                self.vars.max_size().set_ne(ctx.vars, Size::fill());
            }
            if self.vars.resizable().is_new(ctx.vars) {
                self.vars.resizable().set_ne(ctx.vars, false);
            }
            if self.vars.movable().is_new(ctx.vars) {
                self.vars.movable().set_ne(ctx.vars, false);
            }
            if self.vars.always_on_top().is_new(ctx.vars) {
                self.vars.always_on_top().set_ne(ctx.vars, true);
            }
            if self.vars.taskbar_visible().is_new(ctx.vars) {
                self.vars.taskbar_visible().set_ne(ctx.vars, true);
            }
            if self.vars.visible().is_new(ctx.vars) {
                self.vars.visible().set_ne(ctx.vars, true);
            }
        }
    }

    /// Re-flow layout if a layout pass was required. If yes will
    /// flag a render required.
    fn layout(&mut self, ctx: &mut AppContext) {
        let mut w_ctx = self.context.borrow_mut();

        if w_ctx.update != UpdateDisplayRequest::Layout {
            return;
        }
        w_ctx.update = UpdateDisplayRequest::Render;

        profile_scope!("window::layout");

        let auto_size = *self.vars.auto_size().get(ctx.vars);
        let mut size = self.size();
        let mut max_size = self.max_size;
        if auto_size.contains(AutoSize::CONTENT_WIDTH) {
            size.width = max_size.width;
        } else {
            max_size.width = size.width;
        }
        if auto_size.contains(AutoSize::CONTENT_HEIGHT) {
            size.height = max_size.height;
        } else {
            max_size.height = size.height;
        }

        let scale_factor = self.scale_factor();

        w_ctx.root_layout(ctx, self.size(), scale_factor, |root, layout_ctx| {
            let mut final_size = root.measure(layout_ctx, *layout_ctx.viewport_size);

            if !auto_size.contains(AutoSize::CONTENT_WIDTH) {
                final_size.width = size.width;
            }
            if !auto_size.contains(AutoSize::CONTENT_HEIGHT) {
                final_size.height = size.height;
            }
            size = final_size.max(self.min_size).min(self.max_size);
            root.arrange(layout_ctx, size);
        });

        let start_position = w_ctx.root.start_position;

        drop(w_ctx);

        if auto_size != AutoSize::DISABLED {
            if let Some(window) = &self.window {
                let factor = scale_factor;
                let size = glutin::dpi::PhysicalSize::new((size.width * factor) as u32, (size.height * factor) as u32);
                window.set_inner_size(size);
            } else {
                self.headless_size = size;
            }
            self.vars.size().set_ne(ctx.vars, self.size().into());
            self.resize_renderer();
        }

        if let WindowInitState::ContentInited = self.init_state {
            let center_space = match start_position {
                StartPosition::Default => None,
                StartPosition::CenterScreen => Some(LayoutRect::from_size(self.screen_size())),
                StartPosition::CenterParent => {
                    if let Some(parent_id) = self.vars.parent().get(ctx.vars) {
                        if let Ok(parent) = ctx.services.req::<Windows>().window(*parent_id) {
                            Some(LayoutRect::new(parent.position(), parent.size()))
                        } else {
                            Some(LayoutRect::from_size(self.screen_size()))
                        }
                    } else {
                        Some(LayoutRect::from_size(self.screen_size()))
                    }
                }
            };
            if let Some(c) = center_space {
                let x = c.origin.x + ((c.size.width - size.width) / 2.0);
                let y = c.origin.y + ((c.size.height - size.height) / 2.0);
                let pos = LayoutPoint::new(x, y);
                if let Some(wn) = &self.window {
                    let factor = self.scale_factor();
                    let pos = glutin::dpi::PhysicalPosition::new((x * factor) as i32, (y * factor) as i32);
                    wn.set_outer_position(pos);
                } else {
                    self.headless_position = pos;
                }
                self.vars.position().set_ne(ctx.vars, self.position().into());
            }

            if auto_size == AutoSize::DISABLED {
                self.resize_renderer();
            }
        }
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
            let _ = self
                .renderless_event_sender
                .as_ref()
                .unwrap()
                .send_event(AppEvent::NewFrameReady(self.id));

            self.init_state = WindowInitState::Inited;
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
                let _ = self
                    .renderless_event_sender
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
            if let WindowInitState::ContentInited = self.init_state {
                self.redraw();

                // apply initial visibility.
                if *self.vars.visible().get(vars) {
                    self.window.as_ref().unwrap().set_visible(true);
                }
            } else {
                debug_assert!(matches!(self.init_state, WindowInitState::Inited));
                window.request_redraw();
            }
        } else if self.renderer.is_some() {
            self.redraw();
        }
        self.init_state = WindowInitState::Inited;
    }

    /// Redraws the last ready frame and swaps buffers.
    fn redraw(&mut self) {
        if let Some(renderer) = &mut self.renderer {
            profile_scope!("window::redraw");
            let renderer = renderer.get_mut();
            let mut ctx = self.context.borrow_mut();
            let mut args = RedrawArgs::new(renderer);
            (ctx.root.on_pre_redraw)(&mut args);
            args.renderer.present().expect("failed redraw");
            (ctx.root.on_redraw)(&mut args);
            if args.close {
                self.close();
            }
        }
    }

    fn set_icon(window: &Option<glutin::window::Window>, icon: &WindowIcon) {
        match icon {
            WindowIcon::Default => {
                if let Some(window) = window {
                    window.set_window_icon(None);
                }
            }
            WindowIcon::Icon(ico) => {
                if let Some(window) = window {
                    window.set_window_icon(Some((&**ico).clone()));
                }
            }
            WindowIcon::Render(_) => {
                todo!()
            }
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

    fn set_taskbar_visible(&mut self, visible: bool) {
        if visible == self.taskbar_visible {
            return;
        }
        self.taskbar_visible = visible;

        use std::ptr;
        use winapi::shared::winerror;
        use winapi::um::combaseapi;
        use winapi::um::shobjidl_core::ITaskbarList;
        use winapi::Interface;

        // winit already initializes COM

        unsafe {
            let mut tb_ptr: *mut ITaskbarList = ptr::null_mut();
            let result = combaseapi::CoCreateInstance(
                &winapi::um::shobjidl_core::CLSID_TaskbarList,
                ptr::null_mut(),
                winapi::shared::wtypesbase::CLSCTX_INPROC_SERVER,
                &ITaskbarList::uuidof(),
                &mut tb_ptr as *mut _ as *mut _,
            );
            match result {
                winerror::S_OK => {
                    let tb = tb_ptr.as_ref().unwrap();
                    let result = if visible {
                        tb.AddTab(self.hwnd())
                    } else {
                        tb.DeleteTab(self.hwnd())
                    };
                    match result {
                        winerror::S_OK => {}
                        error => {
                            let mtd_name = if visible { "AddTab" } else { "DeleteTab" };
                            log::error!(
                                target: "window",
                                "cannot set `taskbar_visible`, `ITaskbarList::{}` failed, error: {:X}",
                                mtd_name,
                                error
                            )
                        }
                    }
                    tb.Release();
                }
                error => {
                    log::error!(
                        target: "window",
                        "cannot set `taskbar_visible`, failed to create instance of `ITaskbarList`, error: {:X}",
                        error
                    )
                }
            }
        }
    }
}
#[cfg(not(windows))]
impl OpenWindow {
    fn set_taskbar_visible(&mut self, visible: bool) {
        if !visible {
            log::error!(target: "window", "`taskbar_visible = false` only implemented for Windows");
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

struct OwnedWindowContext {
    window_id: WindowId,
    mode: WindowMode,
    root_transform_key: WidgetTransformKey,
    state: OwnedStateMap,
    root: Window,
    api: Option<Arc<RenderApi>>,
    update: UpdateDisplayRequest,
}
impl OwnedWindowContext {
    fn root_context(&mut self, ctx: &mut AppContext, f: impl FnOnce(&mut BoxedUiNode, &mut WidgetContext)) -> UpdateDisplayRequest {
        let root = &mut self.root;

        ctx.window_context(self.window_id, self.mode, &mut self.state, &self.api, |ctx| {
            let child = &mut root.child;
            ctx.widget_context(root.id, &mut root.state, |ctx| {
                f(child, ctx);
            });
        })
        .1
    }

    fn root_layout<R>(
        &mut self,
        ctx: &mut AppContext,
        window_size: LayoutSize,
        scale_factor: f32,
        f: impl FnOnce(&mut BoxedUiNode, &mut LayoutContext) -> R,
    ) -> R {
        let root = &mut self.root;
        ctx.window_context(self.window_id, self.mode, &mut self.state, &self.api, |ctx| {
            let child = &mut root.child;
            ctx.layout_context(14.0, PixelGrid::new(scale_factor), window_size, root.id, &mut root.state, |ctx| {
                f(child, ctx)
            })
        })
        .0
    }

    fn root_render(&mut self, ctx: &mut AppContext, f: impl FnOnce(&mut BoxedUiNode, &mut RenderContext)) {
        let root = &mut self.root;
        ctx.window_context(self.window_id, self.mode, &mut self.state, &self.api, |ctx| {
            let child = &mut root.child;
            ctx.render_context(root.id, &root.state, |ctx| f(child, ctx))
        });
    }

    /// Call [`UiNode::init`] in all nodes.
    pub fn init(&mut self, ctx: &mut AppContext) {
        profile_scope!("window::init");

        let update = self.root_context(ctx, |root, ctx| {
            root.init(ctx);
        });
        self.update |= update;
    }

    /// Call [`UiNode::update`] in all nodes.
    pub fn update(&mut self, ctx: &mut AppContext) {
        profile_scope!("window::update");

        // do UiNode updates
        let update = self.root_context(ctx, |root, ctx| root.update(ctx));
        self.update |= update;
    }

    /// Call [`UiNode::event`] in all nodes.
    pub fn event<EU: EventUpdateArgs>(&mut self, ctx: &mut AppContext, args: &EU) {
        profile_scope!("window::event");

        let update = self.root_context(ctx, |root, ctx| root.event(ctx, args));
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

        app.ctx().services.req::<Windows>().open(test_window, None);

        app.update(false);
    }

    #[test]
    #[should_panic(expected = "can only init renderer in the main thread")]
    pub fn new_window_with_render() {
        let mut app = App::default().run_headless();
        app.enable_renderer(true);
        assert!(app.renderer_enabled());

        app.ctx().services.req::<Windows>().open(test_window, None);

        app.update(false);
    }

    #[test]
    pub fn query_frame() {
        let mut app = App::default().run_headless();

        app.ctx().services.req::<Windows>().open(test_window, None);

        app.update(false); // process open request.
        app.update(true); // process first render.

        let wn = &app.ctx().services.req::<Windows>().windows()[0];

        assert_eq!(wn.id(), wn.frame_info().window_id());

        let root = wn.frame_info().root();

        let expected = Some(true);
        let actual = root.meta().get::<FooMetaKey>().copied();
        assert_eq!(expected, actual);

        let expected = LayoutRect::new(LayoutPoint::zero(), LayoutSize::new(520.0, 510.0));
        let actual = *root.bounds();
        assert_eq!(expected, actual);
    }

    fn test_window(ctx: &mut WindowContext) -> Window {
        ctx.window_state.req::<WindowVars>().size().set(ctx.vars, (520, 510).into());
        Window::new(
            WidgetId::new_unique(),
            StartPosition::Default,
            false,
            HeadlessScreen::default(),
            Box::new(|_| {}),
            Box::new(|_| {}),
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
            frame.meta().set::<FooMetaKey>(true);
        }
    }
}
