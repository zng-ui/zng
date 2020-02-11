use crate::core::app::{AppEvent, AppExtension, ShutdownRequestedArgs};
use crate::core::context::*;
use crate::core::event::*;
use crate::core::render::{FrameBuilder, FrameHitInfo, FrameInfo};
use crate::core::types::*;
use crate::core::var::*;
use crate::core::UiNode;
use fnv::FnvHashMap;
use gleam::gl;
use glutin::dpi::LogicalSize;
use glutin::event_loop::{EventLoopProxy, EventLoopWindowTarget};
use glutin::window::WindowBuilder;
use glutin::{Api, ContextBuilder, GlRequest};
use glutin::{NotCurrent, WindowedContext};
use rayon::{ThreadPool, ThreadPoolBuilder};
use std::borrow::Cow;
use std::num::NonZeroU16;
use std::sync::Arc;
use std::time::Instant;
use webrender::api::*;

event_args! {
    /// [WindowOpen], [WindowClose] event args.
    pub struct WindowEventArgs {
        /// Id of window that was opened or closed.
        pub window_id: WindowId,

        ..

        /// If the widget is in the same window.
        fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
            ctx.window_id == self.window_id
        }
    }

    /// [WindowResize] event args.
    pub struct WindowResizeArgs {
        pub window_id: WindowId,
        pub new_size: LayoutSize,

        ..

        /// If the widget is in the same window.
        fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
            ctx.window_id == self.window_id
        }
    }

    /// [WindowMove] event args.
    pub struct WindowMoveArgs {
        pub window_id: WindowId,
        pub new_position: LayoutPoint,

        ..

        /// If the widget is in the same window.
        fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
            ctx.window_id == self.window_id
        }
    }

    /// [WindowScaleChanged] event args.
    pub struct WindowScaleChangedArgs {
        pub window_id: WindowId,
        pub new_scale_factor: f32,
        pub new_size: LayoutSize,

        ..

        /// If the widget is in the same window.
        fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
            ctx.window_id == self.window_id
        }
    }
}
cancelable_event_args! {
    /// [WindowCloseRequested] event args.
    pub struct WindowCloseRequestedArgs {
        pub window_id: WindowId,
        group: CloseTogetherGroup,

        ..

        /// If the widget is in the same window.
        fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
            ctx.window_id == self.window_id
        }
    }
}

/// New window event.
pub struct WindowOpen;
impl Event for WindowOpen {
    type Args = WindowEventArgs;
}

/// Window resized event.
pub struct WindowResize;
impl Event for WindowResize {
    type Args = WindowResizeArgs;

    const IS_HIGH_PRESSURE: bool = true;
}

/// Window moved event.
pub struct WindowMove;
impl Event for WindowMove {
    type Args = WindowMoveArgs;

    const IS_HIGH_PRESSURE: bool = true;
}

/// Window scale factor changed.
pub struct WindowScaleChanged;
impl Event for WindowScaleChanged {
    type Args = WindowScaleChangedArgs;
}

/// Closing window event.
pub struct WindowCloseRequested;
impl Event for WindowCloseRequested {
    type Args = WindowCloseRequestedArgs;
}
impl CancelableEvent for WindowCloseRequested {
    type Args = WindowCloseRequestedArgs;
}

/// Close window event.
pub struct WindowClose;
impl Event for WindowClose {
    type Args = WindowEventArgs;
}

/// Application extension that manages windows.
///
/// # Events
///
/// Events this extension provides.
///
/// * [WindowOpen]
/// * [WindowResize]
/// * [WindowMove]
/// * [WindowScaleChanged]
/// * [WindowCloseRequested]
/// * [WindowClose]
///
/// # Services
///
/// Services this extension provides.
///
/// * [Windows]
pub struct WindowManager {
    event_loop_proxy: Option<EventLoopProxy<AppEvent>>,
    ui_threads: Arc<ThreadPool>,
    window_open: EventEmitter<WindowEventArgs>,
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
                    crate::core::profiler::register_thread_with_profiler();
                })
                .build()
                .unwrap(),
        );

        WindowManager {
            event_loop_proxy: None,
            ui_threads,
            window_open: EventEmitter::new(false),
            window_resize: EventEmitter::new(true),
            window_move: EventEmitter::new(true),
            window_scale_changed: EventEmitter::new(false),
            window_closing: EventEmitter::new(false),
            window_close: EventEmitter::new(false),
        }
    }
}

impl WindowManager {
    /// Respond to open/close requests.
    fn update_open_close(&mut self, ctx: &mut AppContext) {
        // respond to service requests
        let (open, close) = ctx.services.req::<Windows>().take_requests();

        for request in open {
            let mut w = GlWindow::new(
                request.new,
                ctx,
                ctx.event_loop,
                self.event_loop_proxy.as_ref().unwrap().clone(),
                Arc::clone(&self.ui_threads),
            );

            let args = WindowEventArgs {
                timestamp: Instant::now(),
                window_id: w.id(),
            };

            let mut w_ctx = w.detach_context();
            ctx.services.req::<Windows>().windows.insert(args.window_id, w);
            w_ctx.init(ctx);
            ctx.services
                .req::<Windows>()
                .windows
                .get_mut(&args.window_id)
                .unwrap()
                .attach_context(w_ctx);

            // notify the window requester
            ctx.updates.push_notify(request.notifier, args.clone());

            // notify everyone
            ctx.updates.push_notify(self.window_open.clone(), args.clone());
        }

        for (window_id, group) in close {
            ctx.updates
                .push_notify(self.window_closing.clone(), WindowCloseRequestedArgs::now(window_id, group))
        }
    }

    /// Pump the requested update methods.
    fn update_pump(&mut self, update: UpdateRequest, ctx: &mut AppContext) {
        if update.update_hp || update.update {
            // detach context part so we can let a window content access its own window.
            let mut w_ctxs: Vec<_> = ctx
                .services
                .req::<Windows>()
                .windows
                .iter_mut()
                .map(|(_, w)| w.detach_context())
                .collect();

            // high-pressure pump.
            if update.update_hp {
                for w_ctx in w_ctxs.iter_mut() {
                    w_ctx.update_hp(ctx);
                }
            }

            // low-pressure pump.
            if update.update {
                for w_ctx in w_ctxs.iter_mut() {
                    w_ctx.update(ctx);
                }
            }

            // reatach context parts.
            {
                let service = ctx.services.req::<Windows>();
                for w_ctx in w_ctxs {
                    service.windows.get_mut(&w_ctx.id()).unwrap().attach_context(w_ctx);
                }
            }

            // do window vars update.
            if update.update {
                for (_, window) in ctx.services.req::<Windows>().windows.iter_mut() {
                    window.update(ctx.vars);
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
                // notify close, the window will be deinited on
                // the next update.
                ctx.updates
                    .push_notify(self.window_close.clone(), WindowEventArgs::now(closing.window_id));

                for listener in service.close_listeners.remove(&closing.window_id).unwrap_or_default() {
                    ctx.updates.push_notify(listener, CloseWindowResult::Close);
                }
            } else {
                // canceled notify operation listeners.

                for listener in service.close_listeners.remove(&closing.window_id).unwrap_or_default() {
                    ctx.updates.push_notify(listener, CloseWindowResult::Cancel);
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
            if let Some(mut w) = ctx.services.req::<Windows>().windows.remove(&close.window_id) {
                w.detach_context().deinit(ctx);
                w.deinit();
            }
        }

        let service = ctx.services.req::<Windows>();
        if service.shutdown_on_last_close && service.windows.is_empty() {
            todo!()
        }
    }
}

impl AppExtension for WindowManager {
    fn init(&mut self, r: &mut AppInitContext) {
        self.event_loop_proxy = Some(r.event_loop.clone());
        r.services.register(Windows::new(r.updates.notifier().clone()));
        r.events.register::<WindowOpen>(self.window_open.listener());
        r.events.register::<WindowResize>(self.window_resize.listener());
        r.events.register::<WindowMove>(self.window_move.listener());
        r.events.register::<WindowScaleChanged>(self.window_scale_changed.listener());
        r.events.register::<WindowCloseRequested>(self.window_closing.listener());
        r.events.register::<WindowClose>(self.window_close.listener());
    }

    fn on_window_event(&mut self, window_id: WindowId, event: &WindowEvent, ctx: &mut AppContext) {
        match event {
            WindowEvent::Resized(new_size) => {
                let new_size = LayoutSize::new(new_size.width as f32, new_size.height as f32);

                // raise window_resize
                ctx.updates
                    .push_notify(self.window_resize.clone(), WindowResizeArgs::now(window_id, new_size));

                // set the window size variable if it is not read-only.
                if let Some(window) = ctx.services.req::<Windows>().windows.get_mut(&window_id) {
                    let _ = ctx.updates.push_set(&window.ctx().root.size, new_size);
                }
            }
            WindowEvent::Moved(new_position) => {
                let new_position = LayoutPoint::new(new_position.x as f32, new_position.y as f32);
                ctx.updates
                    .push_notify(self.window_move.clone(), WindowMoveArgs::now(window_id, new_position))
            }
            WindowEvent::CloseRequested => {
                let wins = ctx.services.req::<Windows>();
                if wins.windows.contains_key(&window_id) {
                    wins.close_requests.insert(window_id, None);
                    ctx.updates.push_update();
                }
            }
            WindowEvent::ScaleFactorChanged {
                scale_factor,
                new_inner_size,
            } => {
                if ctx.services.req::<Windows>().windows.contains_key(&window_id) {
                    ctx.updates.push_notify(
                        self.window_scale_changed.clone(),
                        WindowScaleChangedArgs::now(
                            window_id,
                            *scale_factor as f32,
                            LayoutSize::new(new_inner_size.width as f32, new_inner_size.height as f32),
                        ),
                    )
                }
            }
            _ => {}
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

    fn update(&mut self, update: UpdateRequest, ctx: &mut AppContext) {
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
        let windows = std::mem::replace(&mut ctx.services.req::<Windows>().windows, Default::default());
        for (id, mut window) in windows {
            println!("WARNING: destroying `{:?}` without closing events", id);
            window.detach_context().deinit(ctx);
            window.deinit();
        }
    }
}

struct OpenWindowRequest {
    new: Box<dyn FnOnce(&AppContext) -> UiRoot>,
    notifier: EventEmitter<WindowEventArgs>,
}

/// Response message of [Windows::close] and [Windows::close_together].
#[derive(Debug)]
pub enum CloseWindowResult {
    /// Notifying [WindowClose].
    Close,

    /// [WindowCloseRequested] canceled.
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

type CloseTogetherGroup = Option<NonZeroU16>;

/// Windows service.
pub struct Windows {
    /// If shutdown is requested when there are no more windows open, `true` by default.
    pub shutdown_on_last_close: bool,

    open_requests: Vec<OpenWindowRequest>,
    close_requests: FnvHashMap<WindowId, CloseTogetherGroup>,
    next_group: u16,
    close_listeners: FnvHashMap<WindowId, Vec<EventEmitter<CloseWindowResult>>>,
    windows: FnvHashMap<WindowId, GlWindow>,
    update_notifier: UpdateNotifier,
}

impl AppService for Windows {}

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

    fn take_requests(&mut self) -> (Vec<OpenWindowRequest>, FnvHashMap<WindowId, CloseTogetherGroup>) {
        (
            std::mem::replace(&mut self.open_requests, Vec::default()),
            std::mem::replace(&mut self.close_requests, FnvHashMap::default()),
        )
    }

    /// Requests a new window. Returns a listener that will update once when the window is opened.
    pub fn open(&mut self, new_window: impl FnOnce(&AppContext) -> UiRoot + 'static) -> EventListener<WindowEventArgs> {
        let request = OpenWindowRequest {
            new: Box::new(new_window),
            notifier: EventEmitter::new(false),
        };
        let notice = request.notifier.listener();
        self.open_requests.push(request);

        self.update_notifier.push_update();

        notice
    }

    /// Starts closing a window, the operation can be canceled by listeners of the [WindowCloseRequested] event.
    ///
    /// Returns a listener that will update once with the result of the operation.
    pub fn close(&mut self, window_id: WindowId) -> Result<EventListener<CloseWindowResult>, WindowNotFound> {
        if self.windows.contains_key(&window_id) {
            let notifier = EventEmitter::new(false);
            let notice = notifier.listener();
            self.insert_close(window_id, None, notifier);
            self.update_notifier.push_update();
            Ok(notice)
        } else {
            Err(WindowNotFound(window_id))
        }
    }

    fn insert_close(&mut self, window_id: WindowId, set: CloseTogetherGroup, notifier: EventEmitter<CloseWindowResult>) {
        self.close_requests.insert(window_id, set);
        use std::collections::hash_map::Entry::*;
        match self.close_listeners.entry(window_id) {
            Vacant(ve) => {
                ve.insert(vec![notifier]);
            }
            Occupied(mut oe) => oe.get_mut().push(notifier),
        }
    }

    /// Requests closing multi-windows together, the operation can be canceled by listeners of the [WindowCloseRequested] event.
    /// If canceled none of the windows are closed.
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

        let notifier = EventEmitter::new(false);

        for id in buffer {
            self.insert_close(id, Some(set_id), notifier.clone());
        }

        self.update_notifier.push_update();

        Ok(notifier.into_listener())
    }

    fn req_window(&self, window_id: WindowId) -> Result<&GlWindow, WindowNotFound> {
        self.windows.get(&window_id).ok_or(WindowNotFound(window_id))
    }

    /// Hit-test the window lastest frame.
    pub fn hit_test(&self, window_id: WindowId, point: LayoutPoint) -> Result<FrameHitInfo, WindowNotFound> {
        self.req_window(window_id).map(|w| w.hit_test(point))
    }

    /// Reference the window latest frame.
    pub fn frame_info(&self, window_id: WindowId) -> Result<&FrameInfo, WindowNotFound> {
        self.req_window(window_id).map(|w| w.frame_info())
    }
}

#[derive(Clone)]
struct Notifier {
    window_id: WindowId,
    event_loop: EventLoopProxy<AppEvent>,
}
impl RenderNotifier for Notifier {
    fn clone(&self) -> Box<dyn RenderNotifier> {
        Box::new(Clone::clone(self))
    }

    fn wake_up(&self) {}

    fn new_frame_ready(&self, _: DocumentId, _scrolled: bool, _composite_needed: bool, _: Option<u64>) {
        let _ = self.event_loop.send_event(AppEvent::NewFrameReady(self.window_id));
    }
}

struct GlWindow {
    gl_ctx: Option<WindowedContext<NotCurrent>>,
    ctx: Option<OwnedWindowContext>,

    renderer: webrender::Renderer,
    pipeline_id: PipelineId,
    document_id: DocumentId,
    api: Arc<RenderApi>,

    first_draw: bool,
    frame_info: FrameInfo,
}

macro_rules! win_profile_scope {
    ($ctx: expr, $mtd_name: tt) => {
        win_profile_scope!($ctx.id(), $ctx.root.title.get_local(), $mtd_name)
    };
    ($id: expr, $title: expr, $mtd_name: tt) => {
        profile_scope!(r#"({:?} "{}")::{}"#, $id, $title, $mtd_name)
    };
}

impl GlWindow {
    pub fn new(
        new_window: Box<dyn FnOnce(&AppContext) -> UiRoot>,
        ctx: &mut AppContext,
        event_loop: &EventLoopWindowTarget<AppEvent>,
        event_loop_proxy: EventLoopProxy<AppEvent>,
        ui_threads: Arc<ThreadPool>,
    ) -> Self {
        let root = new_window(ctx);
        let inner_size = *root.size.get(ctx.vars);
        let clear_color = *root.background_color.get(ctx.vars);

        let window_builder = WindowBuilder::new()
            .with_visible(false) // not visible until first render, to flickering
            .with_inner_size(LogicalSize::<f64>::new(inner_size.width.into(), inner_size.height.into()));

        let context = ContextBuilder::new()
            .with_gl(GlRequest::GlThenGles {
                opengl_version: (3, 2),
                opengles_version: (3, 0),
            })
            .build_windowed(window_builder, &event_loop)
            .unwrap();

        // SAFETY: This is the recomended way of doing it.
        let context = unsafe { context.make_current().unwrap() };

        let gl = match context.get_api() {
            Api::OpenGl => unsafe { gl::GlFns::load_with(|symbol| context.get_proc_address(symbol) as *const _) },
            Api::OpenGlEs => unsafe { gl::GlesFns::load_with(|symbol| context.get_proc_address(symbol) as *const _) },
            Api::WebGl => panic!("WebGl is not supported"),
        };

        let dpi_factor = context.window().scale_factor() as f32;

        let opts = webrender::RendererOptions {
            device_pixel_ratio: dpi_factor,
            clear_color: Some(clear_color),
            workers: Some(ui_threads),
            ..webrender::RendererOptions::default()
        };

        let notifier = Box::new(Notifier {
            window_id: context.window().id(),
            event_loop: event_loop_proxy,
        });

        let start_size = inner_size * units::LayoutToDeviceScale::new(dpi_factor);
        let start_size = units::DeviceIntSize::new(start_size.width as i32, start_size.height as i32);
        let (renderer, sender) = webrender::Renderer::new(gl.clone(), notifier, opts, None, start_size).unwrap();
        let api = Arc::new(sender.create_api());
        let document_id = api.add_document(start_size, 0);

        let window_id = context.window().id();
        let (state, services) = ctx.new_window(window_id, &api);

        GlWindow {
            gl_ctx: Some(unsafe { context.make_not_current().unwrap() }),
            renderer,
            pipeline_id: PipelineId(1, 0),
            document_id,
            frame_info: FrameInfo::blank(window_id, root.id),
            api: Arc::clone(&api),

            ctx: Some(OwnedWindowContext {
                window_id,
                state,
                services,
                root,
                api,
                update: UpdateDisplayRequest::Layout,
            }),

            first_draw: true,
        }
    }

    /// Window id.
    pub fn id(&self) -> WindowId {
        // this method is required for [win_profile_scope!] to work with [GlWindow] and [OwnedWindowContext].
        self.gl_ctx.as_ref().unwrap().window().id()
    }

    pub fn ctx(&mut self) -> &mut OwnedWindowContext {
        self.ctx.as_mut().unwrap()
    }

    /// Detaches the part of the window required for updating ui-nodes.
    pub fn detach_context(&mut self) -> OwnedWindowContext {
        self.ctx.take().unwrap()
    }

    /// Reatches the part of the window required for updating ui-nodes.
    pub fn attach_context(&mut self, ctx: OwnedWindowContext) {
        assert_eq!(self.id(), ctx.id());
        self.ctx = Some(ctx);
    }

    /// Update window vars.
    pub fn update(&mut self, vars: &Vars) {
        let ctx = self.ctx.as_mut().unwrap();

        win_profile_scope!(ctx, "update::self");

        let window = self.gl_ctx.as_ref().unwrap().window();
        let r = &mut ctx.root;

        if let Some(title) = r.title.update_local(vars) {
            window.set_title(title);
        }
    }

    /// Recompute layout if a layout pass was required. If yes will
    /// flag a [render] required.
    pub fn layout(&mut self) {
        let ctx = self.ctx.as_mut().unwrap();

        if ctx.update == UpdateDisplayRequest::Layout {
            win_profile_scope!(ctx, "layout");

            ctx.update = UpdateDisplayRequest::Render;

            let available_size = self.gl_ctx.as_ref().unwrap().window().inner_size();
            let available_size = LayoutSize::new(available_size.width as f32, available_size.height as f32);

            let desired_size = ctx.root.child.measure(available_size);

            let final_size = desired_size.min(available_size);

            ctx.root.child.arrange(final_size);
        }
    }

    /// Render a frame if one was required.
    pub fn render(&mut self) {
        let ctx = self.ctx.as_mut().unwrap();

        if ctx.update == UpdateDisplayRequest::Render {
            win_profile_scope!(ctx, "render");

            ctx.update = UpdateDisplayRequest::None;

            let size = self.gl_ctx.as_ref().unwrap().window().inner_size();
            let size = LayoutSize::new(size.width as f32, size.height as f32);

            let frame_id = Epoch({
                let mut next = self.frame_info.frame_id().0.wrapping_add(1);
                if next == FrameId::invalid().0 {
                    next = next.wrapping_add(1);
                }
                next
            });

            let mut frame = FrameBuilder::new(frame_id, ctx.id(), self.pipeline_id, ctx.root.id, size);

            ctx.root.child.render(&mut frame);

            let (display_list_data, frame_info) = frame.finalize();

            self.frame_info = frame_info;

            let mut txn = Transaction::new();
            txn.set_display_list(frame_id, None, size, display_list_data, true);
            txn.set_root_pipeline(self.pipeline_id);
            txn.generate_frame();
            self.api.send_transaction(self.document_id, txn);
        }
    }

    /// Notifies the OS to redraw the window, will receive WindowEvent::RedrawRequested
    /// from the OS after calling this.
    pub fn request_redraw(&mut self) {
        let context = self.gl_ctx.as_ref().unwrap();
        if self.first_draw {
            context.window().set_visible(true); // OS generates a RequestRedraw here
            self.first_draw = false;
        } else {
            context.window().request_redraw();
        }
    }

    /// Redraws the last ready frame and swaps buffers.
    ///
    /// **`swap_buffers` Warning**: if you enabled vsync, this function will block until the
    /// next time the screen is refreshed. However drivers can choose to
    /// override your vsync settings, which means that you can't know in
    /// advance whether `swap_buffers` will block or not.
    pub fn redraw(&mut self) {
        win_profile_scope!(self.ctx.as_ref().unwrap(), "redraw");

        let context = unsafe { self.gl_ctx.take().unwrap().make_current().unwrap() };

        self.renderer.update();

        let size = context.window().inner_size();
        let size = LayoutSize::new(size.width as f32, size.height as f32);
        let dpi_factor = context.window().scale_factor() as f32;

        let device_size = size * units::LayoutToDeviceScale::new(dpi_factor);
        let device_size = units::DeviceIntSize::new(device_size.width as i32, device_size.height as i32);

        self.renderer.render(device_size).unwrap();
        let _ = self.renderer.flush_pipeline_info();

        context.swap_buffers().ok();

        self.gl_ctx = Some(unsafe { context.make_not_current().unwrap() });
    }

    /// Deinits renderer and OpenGl context.
    ///
    /// # Panics
    /// If the [OwnedWindowContext] was not already deinited.
    pub fn deinit(mut self) {
        assert!(self.ctx.is_none()); // must deinit UiNodes first.

        win_profile_scope!(self.id(), "", "deinit::self");

        let context = unsafe { self.gl_ctx.take().unwrap().make_current().unwrap() };
        self.renderer.deinit();
        unsafe { context.make_not_current().unwrap() };
    }

    /// Hit-test the latest frame.
    pub fn hit_test(&self, point: LayoutPoint) -> FrameHitInfo {
        let r = self.api.hit_test(
            self.document_id,
            Some(self.pipeline_id),
            units::WorldPoint::new(point.x, point.y),
            HitTestFlags::all(),
        );

        FrameHitInfo::new(self.id(), self.frame_info.frame_id(), point, r)
    }

    /// Latest frame info.
    pub fn frame_info(&self) -> &FrameInfo {
        &self.frame_info
    }
}

/// The part of a [GlWindow] that must be detached to provide update notifications
/// that still permit borrowing the owning [GlWindow].
pub(crate) struct OwnedWindowContext {
    window_id: WindowId,
    state: WindowState,
    services: WindowServices,
    root: UiRoot,
    api: Arc<RenderApi>,
    update: UpdateDisplayRequest,
}

impl OwnedWindowContext {
    /// Window id.
    pub fn id(&self) -> WindowId {
        // this method is required for [win_profile_scope!] to work with [GlWindow] and [OwnedWindowContext].
        self.window_id
    }

    fn root_context(&mut self, ctx: &mut AppContext, f: impl FnOnce(&mut Box<dyn UiNode>, &mut WidgetContext)) -> UpdateDisplayRequest {
        let root = &mut self.root;

        ctx.window_context(self.window_id, &mut self.state, &mut self.services, &self.api, |ctx| {
            let child = &mut root.child;
            ctx.widget_context(root.id, &mut root.meta, |ctx| {
                f(child, ctx);
            });
        })
    }

    /// Call [UiNode::init] in all nodes.
    pub fn init(&mut self, ctx: &mut AppContext) {
        win_profile_scope!(self, "init");

        let update = self.root_context(ctx, |root, ctx| {
            ctx.updates.push_layout();

            root.init(ctx);
        });
        self.update |= update;
    }

    /// Call [UiNode::update_hp] in all nodes.
    pub fn update_hp(&mut self, ctx: &mut AppContext) {
        win_profile_scope!(self, "update_hp");

        let update = self.root_context(ctx, |root, ctx| root.update_hp(ctx));
        self.update |= update;
    }

    /// Call [UiNode::update] in all nodes.
    pub fn update(&mut self, ctx: &mut AppContext) {
        win_profile_scope!(self, "update");

        // do UiNode updates
        let update = self.root_context(ctx, |root, ctx| root.update_hp(ctx));
        self.update |= update;
    }

    /// Call [UiNode::deinit] in all nodes.
    pub fn deinit(mut self, ctx: &mut AppContext) {
        win_profile_scope!(self, "deinit");
        self.root_context(ctx, |root, ctx| root.deinit(ctx));
    }
}

pub struct UiRoot {
    id: WidgetId,
    meta: LazyStateMap,
    title: BoxLocalVar<Cow<'static, str>>,
    size: SharedVar<LayoutSize>,
    background_color: BoxVar<ColorF>,
    child: Box<dyn UiNode>,
}

// TODO widget like window! macro
//fn window(
//    child: impl UiNode,
//    title: impl IntoVar<Cow<'static, str>>,
//    size: impl Into<SharedVar<LayoutSize>>,
//    background_color: impl IntoVar<ColorF>,
//) -> UiRoot {
//    UiRoot {
//        title: Box::new(title.into_var()),
//        size: size.into(),
//        background_color: Box::new(background_color.into_var()),
//        child: Box::new(child),
//    }
//}
