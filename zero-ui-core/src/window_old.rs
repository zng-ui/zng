//! App windows manager.
use crate::{
    app::{self, raw_events::*, view_process::ViewWindow, *},
    context::*,
    event::*,
    profiler::profile_scope,
    render::*,
    service::Service,
    text::{Text, ToText},
    units::*,
    var::*,
    BoxedUiNode, UiNode, WidgetId,
};

use linear_map::LinearMap;
use rayon::{ThreadPool, ThreadPoolBuilder};
use std::{
    cell::{Cell, RefCell},
    fmt, mem,
    rc::Rc,
    sync::Arc,
};

impl AppExtension for WindowManager {
    fn init(&mut self, ctx: &mut AppContext) {
        ctx.services.register(Screens::new());
        ctx.services.register(Windows::new(ctx.updates.sender()));
    }

    fn event_preview<EV: EventUpdateArgs>(&mut self, ctx: &mut AppContext, args: &EV) {
        if let Some(args) = RawWindowFocusEvent.update(args) {
            if let Some(window) = ctx.services.windows().windows.iter_mut().find(|w| w.id == args.window_id) {
                window.is_focused = args.focused;

                let args = WindowIsFocusedArgs::now(args.window_id, window.is_focused, false);
                self.notify_focus(args, ctx.events);
            }
        } else if let Some(args) = RawWindowResizedEvent.update(args) {
            if let Some(window) = ctx.services.windows().windows.iter_mut().find(|w| w.id == args.window_id) {
                let new_size = window.size();

                // set the window size variable.
                if window.vars.size().set_ne(ctx.vars, new_size) {
                    // is new size:
                    ctx.updates.layout();
                    window.expect_layout_update();
                    window.resize_renderer();

                    // raise window_resize
                    WindowResizeEvent.notify(ctx.events, WindowResizeArgs::now(args.window_id, new_size));
                }
            }
        } else if let Some(args) = RawWindowMovedEvent.update(args) {
            if let Some(window) = ctx.services.windows().windows.iter().find(|w| w.id == args.window_id) {
                let new_position = window.position();

                // TODO check if in new monitor.

                // set the window position variable if it is not read-only.
                window.vars.position().set_ne(ctx.vars, new_position);

                // raise window_move
                WindowMoveEvent.notify(ctx.events, WindowMoveArgs::now(args.window_id, new_position));
            }
        } else if let Some(args) = RawWindowCloseRequestedEvent.update(args) {
            if let Some(win) = ctx.services.windows().windows.iter().find(|w| w.id == args.window_id) {
                *win.close_response.borrow_mut() = Some(response_var().0);
                ctx.updates.update();
            }
        } else if let Some(args) = RawWindowScaleFactorChangedEvent.update(args) {
            if let Some(window) = ctx.services.windows().windows.iter_mut().find(|w| w.id == args.window_id) {
                let scale_factor = args.scale_factor as f32;
                let new_size = LayoutSize::new(args.size.0 as f32 / scale_factor, args.size.1 as f32 / scale_factor);

                // winit has not set the new_inner_size yet, so
                // we can determinate if the system only changed the size
                // to visually match the new scale_factor or if the window was
                // really resized.
                if window.vars.size().get(ctx.vars) == &new_size.into() {
                    // if it only changed to visually match, the WindowEvent::Resized
                    // will not cause a re-layout, so we need to do it here, but window.resize_renderer()
                    // calls window.size(), so we need to set the new_inner_size before winit.
                    if let Some(w) = &window.window {
                        w.set_size(width, height);
                        w.set_inner_size(glutin::dpi::PhysicalSize::new(args.size.0, args.size.1));
                    }
                    ctx.updates.layout();
                    window.expect_layout_update();
                    window.resize_renderer();
                }

                WindowScaleChangedEvent.notify(ctx.events, WindowScaleChangedArgs::now(args.window_id, scale_factor, new_size));
            }
        }
    }

    fn event_ui<EV: EventUpdateArgs>(&mut self, ctx: &mut AppContext, args: &EV) {
        let wn_ctxs: Vec<_> = ctx.services.windows().windows.iter_mut().map(|w| w.context.clone()).collect();

        for wn_ctx in wn_ctxs {
            wn_ctx.borrow_mut().event(ctx, args);
        }
    }

    fn update_ui(&mut self, ctx: &mut AppContext) {
        self.update_open_close(ctx);
        self.update_pump(ctx);
    }

    fn event<EV: EventUpdateArgs>(&mut self, ctx: &mut AppContext, args: &EV) {
        if let Some(args) = WindowCloseRequestedEvent.update(args) {
            self.update_closing(ctx, args);
        } else if let Some(args) = WindowCloseEvent.update(args) {
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
            let wns = ctx.services.windows();
            (mem::take(&mut wns.windows), mem::take(&mut wns.opening_windows))
        };
        for window in windows.iter_mut().chain(&mut opening) {
            window.layout(ctx);
            window.render(ctx);
            window.render_update(ctx);
        }

        let wns = ctx.services.windows();
        wns.windows = windows;
        wns.opening_windows = opening;
    }

    fn new_frame(&mut self, ctx: &mut AppContext, window_id: WindowId) {
        let wns = ctx.services.windows();
        if let Some(window) = wns.windows.iter_mut().find(|w| w.id == window_id) {
            window.request_redraw(ctx.vars);
        } else if let Some(idx) = wns.opening_windows.iter().position(|w| w.id == window_id) {
            let mut window = wns.opening_windows.remove(idx);
            window.request_redraw(ctx.vars);

            debug_assert!(matches!(window.init_state, WindowInitState::Inited));

            let args = WindowOpenArgs::now(window.id);
            window.open_response.take().unwrap().respond(ctx.vars, args.clone());
            WindowOpenEvent.notify(ctx.events, args);
            wns.windows.push(window);
        }
    }

    fn redraw_requested(&mut self, ctx: &mut AppContext, window_id: WindowId) {
        if let Some(window) = ctx.services.windows().windows.iter_mut().find(|w| w.id == window_id) {
            window.redraw();
        }
    }

    fn shutdown_requested(&mut self, ctx: &mut AppContext, args: &ShutdownRequestedArgs) {
        if !args.cancel_requested() {
            let service = ctx.services.windows();
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
        let windows = mem::take(&mut ctx.services.windows().windows);
        for window in windows {
            {
                log::error!(
                    target: "window",
                    "dropping `{:?} ({})` without closing events",
                    window.id,
                    window.vars.title().get(ctx)
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
        let (open, close) = ctx.services.windows().take_requests();

        for request in open {
            let ppi_map = ctx.services.screens().ppi.clone();
            let w = OpenWindow::new(
                request.new,
                request.force_headless,
                request.responder,
                ctx,
                ctx.window_target,
                Arc::clone(&self.ui_threads),
                ctx.updates.sender(),
                ppi_map,
            );
            ctx.services.windows().opening_windows.push(w);
        }

        for window_id in close {
            WindowCloseRequestedEvent.notify(ctx.events, WindowCloseRequestedArgs::now(window_id));
        }
    }

    /// Pump the requested update methods.
    fn update_pump(&mut self, ctx: &mut AppContext) {
        // detach context part so we can let a window content access its own window.
        let wn_ctxs: Vec<_> = ctx.services.windows().windows.iter_mut().map(|w| w.context.clone()).collect();

        for wn_ctx in &wn_ctxs {
            wn_ctx.borrow_mut().update(ctx);
        }

        // do window vars update.
        let mut windows = mem::take(&mut ctx.services.windows().windows);
        for window in windows.iter_mut() {
            window.update_window(ctx);
        }
        ctx.services.windows().windows = windows;

        // do preload updates.
        let mut opening = mem::take(&mut ctx.services.windows().opening_windows);
        for window in &mut opening {
            debug_assert!(!matches!(window.init_state, WindowInitState::Inited));
            window.preload_update_window(ctx);
        }
        ctx.services.windows().opening_windows = opening;
    }

    /// Respond to window_closing events.
    fn update_closing(&mut self, ctx: &mut AppContext, args: &WindowCloseRequestedArgs) {
        let wins = ctx.services.windows();
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
                WindowCloseEvent.notify(ctx.events, WindowCloseArgs::now(args.window_id));
                let responder = win.close_response.borrow_mut().take().unwrap();
                responder.respond(ctx.vars, CloseWindowResult::Close);
            }
        }
    }

    /// Respond to window_close events.
    fn update_close(&mut self, ctx: &mut AppContext, args: &WindowCloseArgs) {
        // remove the window.
        let window = {
            let wns = ctx.services.windows();
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
        let service = ctx.services.windows();
        if service.shutdown_on_last_close && service.windows.is_empty() && service.opening_windows.is_empty() {
            ctx.services.app_process().shutdown();
        }
    }

    fn notify_focus(&self, args: WindowIsFocusedArgs, events: &mut Events) {
        debug_assert!(!args.closed || (args.closed && !args.focused));

        WindowFocusChangedEvent.notify(events, args.clone());
        if args.focused {
            WindowFocusEvent.notify(events, args)
        } else {
            WindowBlurEvent.notify(events, args);
        }
    }
}


/// Windows service.
///
/// # Provider
///
/// This service is provided by the [`WindowManager`].
#[derive(Service)]
pub struct Windows {
    /// If shutdown is requested when a window closes and there are no more windows open, `true` by default.
    pub shutdown_on_last_close: bool,

    windows: Vec<OpenWindow>,

    open_requests: Vec<OpenWindowRequest>,
    opening_windows: Vec<OpenWindow>,
    update_sender: AppEventSender,
}
impl Windows {
    

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


/// An open window.
pub struct OpenWindow {
    context: Rc<RefCell<OwnedWindowContext>>,

    window: Option<ViewWindow>,
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

    open_response: Option<ResponderVar<WindowOpenArgs>>,
    close_response: RefCell<Option<ResponderVar<CloseWindowResult>>>,
    close_canceled: RefCell<Rc<Cell<bool>>>,
    app_sender: AppEventSender,

    screen_ppi: ScreenPpiMap,
}
impl OpenWindow {
    #[allow(clippy::too_many_arguments)]
    fn new(
        new_window: Box<dyn FnOnce(&mut WindowContext) -> Window>,
        force_headless: Option<WindowMode>,
        open_response: ResponderVar<WindowOpenArgs>,
        ctx: &mut AppContext,
        ui_threads: Arc<ThreadPool>,
        app_sender: AppEventSender,
        screen_ppi: ScreenPpiMap,
    ) -> Self {
        // get mode.
        let mut mode = if ctx.is_headless() {
            if ctx.app_state.get(app::HeadlessRendererEnabledKey).copied().unwrap_or_default() {
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

        let vars = WindowVars::new();
        let mut wn_state = OwnedStateMap::default();
        wn_state.set(WindowVarsKey, vars.clone());

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
                let window_ = WindowBuilder::new()
                    .with_inner_size(glutin::dpi::LogicalSize::new(800.0, 600.0))
                    .with_visible(false); // not visible until first render, to avoid flickering

                let sender = app_sender.clone();
                let r = Renderer::new_with_glutin(window_, window_target, renderer_config, move |args: NewFrameArgs| {
                    let _ = sender.send_new_frame(args.window_id.unwrap());
                })
                .expect("failed to create a window renderer");

                api = Some(Rc::clone(r.0.api()));
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

                id = WindowId::new_unique();

                if headless == WindowMode::HeadlessWithRenderer {
                    let sender = app_sender.clone();
                    let rend = Renderer::new(
                        RenderSize::zero(),
                        1.0,
                        renderer_config,
                        move |args: NewFrameArgs| {
                            let _ = sender.send_new_frame(args.window_id.unwrap());
                        },
                        Some(id),
                    )
                    .expect("failed to create a headless renderer");

                    api = Some(Rc::clone(rend.api()));
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
            headless_size: LayoutSize::new(800.0, 600.0),
            headless_state: WindowState::Normal,
            headless_screen,
            taskbar_visible: true,
            mode,
            init_state: WindowInitState::New,
            min_size: LayoutSize::new(192.0, 48.0),
            max_size: LayoutSize::new(f32::INFINITY, f32::INFINITY),
            is_focused: true,
            frame_info,

            open_response: Some(open_response),
            close_response: RefCell::default(),
            close_canceled: RefCell::default(),
            app_sender,

            screen_ppi,

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
            let _ = self.app_sender.send_update();
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

    /// Current monitor screen that is presenting the headed window.
    ///
    /// Returns `None` if the window is headless or the current monitor can't be detected.
    pub fn current_screen(&self) -> Option<Screen> {
        self.window.as_ref().and_then(|w| w.current_monitor()).map(|m| Screen {
            handle: m,
            ppi: self.screen_ppi.clone(),
        })
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
                        LayoutSize::new(1920.0, 1080.0)
                    } else {
                        // Monitor
                        LayoutSize::new(s.width as f32 / pixel_factor, s.height as f32 / pixel_factor)
                    }
                })
                .unwrap_or_else(|| {
                    // No Monitor
                    LayoutSize::new(1920.0, 1080.0)
                })
        } else {
            self.headless_screen.screen_size
        }
    }

    /// Pixel-per-inch configured for the current monitor screen.
    ///
    /// Returns the [`HeadlessScreen::ppi`] if the window is headless, returns `96.0` for
    /// headed windows without current screen or when the screen does not have a PPI configured.
    pub fn screen_ppi(&self) -> f32 {
        if let Some(window) = &self.window {
            window
                .current_monitor()
                .and_then(|h| self.screen_ppi.borrow().get(&h).copied())
                .unwrap_or(96.0)
        } else {
            self.headless_screen.ppi
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
        if let Some(title) = self.vars.title().get_new(ctx) {
            if let Some(window) = &self.window {
                window.set_title(title);
            }
        }

        if let Some(icon) = self.vars.icon().get_new(ctx) {
            Self::set_icon(&self.window, icon);
        }

        if !self.kiosk {
            if let Some(auto_size) = self.vars.auto_size().copy_new(ctx) {
                // size will be updated in self.layout(..)
                ctx.updates.layout();

                let resizable = auto_size == AutoSize::DISABLED && *self.vars.resizable().get(ctx);
                self.vars.resizable().set_ne(ctx.vars, resizable);

                if let Some(window) = &self.window {
                    window.set_resizable(resizable);
                }
            }

            if let Some(min_size) = self.vars.min_size().get_new(ctx.vars) {
                let factor = self.scale_factor();
                let prev_min_size = self.min_size;
                let min_size = ctx.outer_layout_context(self.screen_size(), factor, self.screen_ppi(), self.id, self.root_id, |ctx| {
                    min_size.to_layout(ctx.metrics.viewport_size, ctx)
                });

                if min_size.width.is_finite() {
                    self.min_size.width = min_size.width;
                }
                if min_size.height.is_finite() {
                    self.min_size.height = min_size.height;
                }
                self.vars.min_size().set_ne(ctx.vars, self.min_size);
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

            if let Some(max_size) = self.vars.max_size().get_new(ctx.vars) {
                let factor = self.scale_factor();
                let prev_max_size = self.max_size;
                let max_size = ctx.outer_layout_context(self.screen_size(), factor, self.screen_ppi(), self.id, self.root_id, |ctx| {
                    max_size.to_layout(ctx.metrics.viewport_size, ctx)
                });

                if max_size.width.is_finite() {
                    self.max_size.width = max_size.width;
                }
                if max_size.height.is_finite() {
                    self.max_size.height = max_size.height;
                }
                self.vars.max_size().set_ne(ctx.vars, self.max_size);
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

            if let Some(size) = self.vars.size().get_new(ctx.vars) {
                let current_size = self.size();
                if AutoSize::DISABLED == *self.vars.auto_size().get(ctx) {
                    let factor = self.scale_factor();
                    let mut size = ctx.outer_layout_context(self.screen_size(), factor, self.screen_ppi(), self.id, self.root_id, |ctx| {
                        size.to_layout(ctx.metrics.viewport_size, ctx)
                    });

                    if !size.width.is_finite() {
                        size.width = current_size.width;
                    }
                    if !size.height.is_finite() {
                        size.height = current_size.height;
                    }

                    self.vars.size().set_ne(ctx.vars, size);
                    if let Some(window) = &self.window {
                        let size = glutin::dpi::PhysicalSize::new((size.width * factor) as u32, (size.height * factor) as u32);
                        window.set_inner_size(size);
                        self.resize_renderer();
                    } else {
                        self.headless_size = size;
                    }
                } else {
                    // cannot change size if auto-sizing.
                    self.vars.size().set_ne(ctx.vars, current_size);
                }
            }

            if let Some(pos) = self.vars.position().get_new(ctx.vars) {
                let factor = self.scale_factor();
                let current_pos = self.position();
                let mut pos = ctx.outer_layout_context(self.screen_size(), factor, self.screen_ppi(), self.id, self.root_id, |ctx| {
                    pos.to_layout(ctx.metrics.viewport_size, ctx)
                });

                if !pos.x.is_finite() {
                    pos.x = current_pos.x;
                }
                if !pos.y.is_finite() {
                    pos.y = current_pos.y;
                }

                self.vars.position().set_ne(ctx.vars, pos);

                if let Some(window) = &self.window {
                    let pos = glutin::dpi::PhysicalPosition::new((pos.x * factor) as i32, (pos.y * factor) as i32);
                    window.set_outer_position(pos);
                } else {
                    self.headless_position = pos;
                }
            }

            if let Some(always_on_top) = self.vars.always_on_top().copy_new(ctx) {
                if let Some(window) = &self.window {
                    window.set_always_on_top(always_on_top);
                }
            }

            if let Some(taskbar_visible) = self.vars.taskbar_visible().copy_new(ctx) {
                self.set_taskbar_visible(taskbar_visible);
            }

            if let Some(chrome) = self.vars.chrome().get_new(ctx) {
                if let Some(window) = &self.window {
                    window.set_decorations(chrome.is_default());
                }
            }

            if let Some(visible) = self.vars.visible().copy_new(ctx) {
                if let Some(window) = &self.window {
                    window.set_visible(visible && matches!(self.init_state, WindowInitState::Inited));
                }
            }
        } else {
            // kiosk mode
            if let Some(state) = self.vars.state().copy_new(ctx) {
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
            if self.vars.position().is_new(ctx) {
                self.vars.position().set_ne(ctx.vars, Point::zero());
            }
            if self.vars.auto_size().is_new(ctx) {
                self.vars.auto_size().set_ne(ctx.vars, AutoSize::DISABLED);
            }
            if self.vars.min_size().is_new(ctx) {
                self.vars.min_size().set_ne(ctx.vars, Size::zero());
            }
            if self.vars.max_size().is_new(ctx) {
                self.vars.max_size().set_ne(ctx.vars, Size::fill());
            }
            if self.vars.resizable().is_new(ctx) {
                self.vars.resizable().set_ne(ctx.vars, false);
            }
            if self.vars.movable().is_new(ctx) {
                self.vars.movable().set_ne(ctx.vars, false);
            }
            if self.vars.always_on_top().is_new(ctx) {
                self.vars.always_on_top().set_ne(ctx.vars, true);
            }
            if self.vars.taskbar_visible().is_new(ctx) {
                self.vars.taskbar_visible().set_ne(ctx.vars, true);
            }
            if self.vars.visible().is_new(ctx) {
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

        let auto_size = *self.vars.auto_size().get(ctx);
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
        let screen_ppi = self.screen_ppi();

        w_ctx.root_layout(ctx, self.size(), scale_factor, screen_ppi, |root, layout_ctx| {
            let mut final_size = root.measure(layout_ctx, layout_ctx.metrics.viewport_size);

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
            self.vars.size().set_ne(ctx.vars, self.size());
            self.resize_renderer();
        }

        if let WindowInitState::ContentInited = self.init_state {
            let center_space = match start_position {
                StartPosition::Default => None,
                StartPosition::CenterScreen => Some(LayoutRect::from_size(self.screen_size())),
                StartPosition::CenterParent => {
                    if let Some(parent_id) = self.vars.parent().copy(ctx) {
                        if let Ok(parent) = ctx.services.windows().window(parent_id) {
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
                self.vars.position().set_ne(ctx.vars, self.position());
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
            let _ = self.app_sender.send_new_frame(self.id);

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
                let _ = self.app_sender.send_new_frame(self.id);
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
    /// Use this to receive Windows OS events not covered in [`raw_events`].
    ///
    /// Returns if adding a subclass handler succeeded.
    ///
    /// # Handler
    ///
    /// The handler inputs are the first 4 arguments of a [`SUBCLASSPROC`].
    /// You can use closure capture to include extra data.
    ///
    /// The handler must return `Some(LRESULT)` to stop the propagation of a specific message.
    ///
    /// The handler is dropped after it receives the `WM_DESTROY` message.
    ///
    /// # Panics
    ///
    /// Panics in headless mode.
    ///
    /// [`raw_events`]: crate::app::raw_events
    /// [`SUBCLASSPROC`]: https://docs.microsoft.com/en-us/windows/win32/api/commctrl/nc-commctrl-subclassproc
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
    update: UpdateDisplayRequest,
}
impl OwnedWindowContext {
    fn root_context(&mut self, ctx: &mut AppContext, f: impl FnOnce(&mut BoxedUiNode, &mut WidgetContext)) -> UpdateDisplayRequest {
        let root = &mut self.root;

        ctx.window_context(self.window_id, self.mode, &mut self.state, |ctx| {
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
        screen_ppi: f32,
        f: impl FnOnce(&mut BoxedUiNode, &mut LayoutContext) -> R,
    ) -> R {
        let root = &mut self.root;
        ctx.window_context(self.window_id, self.mode, &mut self.state, &self.api, |ctx| {
            let child = &mut root.child;
            ctx.layout_context(
                14.0,
                PixelGrid::new(scale_factor),
                screen_ppi,
                window_size,
                root.id,
                &mut root.state,
                |ctx| f(child, ctx),
            )
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

        app.ctx().services.windows().open(test_window);

        app.update(false);
    }

    #[test]
    #[should_panic(expected = "can only init renderer in the main thread")]
    pub fn new_window_with_render() {
        let mut app = App::default().run_headless();
        app.enable_renderer(true);
        assert!(app.renderer_enabled());

        app.ctx().services.windows().open(test_window);

        app.update(false);
    }

    #[test]
    pub fn query_frame() {
        let mut app = App::default().run_headless();

        app.ctx().services.windows().open(test_window);

        app.update(false); // process open request.
        app.update(true); // process first render.

        let wn = &app.ctx().services.windows().windows()[0];

        assert_eq!(wn.id(), wn.frame_info().window_id());

        let root = wn.frame_info().root();

        let expected = Some(true);
        let actual = root.meta().get(FooMetaKey).copied();
        assert_eq!(expected, actual);

        let expected = LayoutRect::new(LayoutPoint::zero(), LayoutSize::new(520.0, 510.0));
        let actual = *root.bounds();
        assert_eq!(expected, actual);
    }

    fn test_window(ctx: &mut WindowContext) -> Window {
        ctx.window_state.req(WindowVarsKey).size().set(ctx.vars, (520, 510));
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
            frame.meta().set(FooMetaKey, true);
        }
    }
}
