use super::*;
use contexts::*;
use gleam::gl;
use glutin::dpi::LogicalSize;
use glutin::event_loop::{EventLoopProxy, EventLoopWindowTarget};
use glutin::window::WindowBuilder;
use glutin::{Api, ContextBuilder, GlRequest};
use glutin::{NotCurrent, WindowedContext};
use rayon::{ThreadPool, ThreadPoolBuilder};
use std::borrow::Cow;
use std::cell::Cell;
use std::sync::Arc;
use std::time::Instant;
use webrender::api::*;

pub use webrender::api::ColorF;

/// New window event.
pub enum WindowOpen {}

/// Window resized event.
pub enum WindowResize {}

/// Window moved event.
pub enum WindowMove {}

/// Closing window event.
pub enum WindowClosing {}

/// Closed window event.
pub enum WindowClose {}

/// [WindowOpen], [WindowClose] event args.
#[derive(Debug, Clone)]
pub struct WindowArgs {
    pub timestamp: Instant,
    pub window_id: WindowId,
}

impl EventArgs for WindowArgs {
    fn timestamp(&self) -> Instant {
        self.timestamp
    }
}

/// [NewWindow] event args.
#[derive(Debug, Clone)]
pub struct WindowClosingArgs {
    pub timestamp: Instant,
    pub window_id: WindowId,
    cancel: Cell<bool>,
}

impl WindowClosingArgs {
    pub fn new(window_id: WindowId) -> Self {
        WindowClosingArgs {
            timestamp: Instant::now(),
            window_id,
            cancel: Cell::new(false),
        }
    }

    /// Gets if a handler canceled the window close.
    pub fn cancel_requested(&self) -> bool {
        self.cancel.get()
    }

    /// Cancel the window close.
    pub fn cancel(&self) {
        self.cancel.set(true);
    }
}

impl EventArgs for WindowClosingArgs {
    fn timestamp(&self) -> Instant {
        self.timestamp
    }
}

impl Event for WindowOpen {
    type Args = WindowArgs;
}

impl Event for WindowClose {
    type Args = WindowArgs;
}

impl Event for WindowClosing {
    type Args = WindowClosingArgs;
}

/// Windows management [AppExtension].
pub(crate) struct AppWindows {
    event_loop_proxy: EventLoopProxy<WebRenderEvent>,
    ui_threads: Arc<ThreadPool>,
    windows: Vec<GlWindow>,
    window_open: EventEmitter<WindowArgs>,
    window_closing: EventEmitter<WindowClosingArgs>,
    window_close: EventEmitter<WindowArgs>,
}

impl AppWindows {
    pub fn new(event_loop_proxy: EventLoopProxy<WebRenderEvent>) -> Self {
        let ui_threads = Arc::new(
            ThreadPoolBuilder::new()
                .thread_name(|idx| format!("UI#{}", idx))
                .start_handler(|_| {
                    #[cfg(feature = "app_profiler")]
                    register_thread_with_profiler();
                })
                .build()
                .unwrap(),
        );

        AppWindows {
            event_loop_proxy,
            ui_threads,
            windows: Vec::with_capacity(1),
            window_open: EventEmitter::new(false),
            window_closing: EventEmitter::new(false),
            window_close: EventEmitter::new(false),
        }
    }

    pub fn update_hp(&mut self, ctx: &mut AppContext) {
        for window in self.windows.iter_mut() {
            window.update_hp(ctx);
        }
    }

    pub fn update(&mut self, ctx: &mut AppContext) {
        for window in self.windows.iter_mut() {
            window.update(ctx);
        }
    }

    pub fn layout(&mut self) {
        for window in self.windows.iter_mut() {
            window.layout();
        }
    }

    pub fn render(&mut self) {
        for window in self.windows.iter_mut() {
            window.render();
        }
    }

    pub fn new_frame_ready(&mut self, window_id: WindowId) {
        // TODO do we need a hash_map?
        for window in self.windows.iter_mut() {
            if window.id() == window_id {
                window.request_redraw();
                break;
            }
        }
    }

    fn window_mut(&mut self, window_id: WindowId) -> Option<&mut GlWindow> {
        for window in self.windows.iter_mut() {
            if window.id() == window_id {
                return Some(window);
            }
        }
        None
    }
}

impl AppExtension for AppWindows {
    fn init(&mut self, r: &mut AppInitContext) {
        r.services.register(Windows::new());
        r.events.register::<WindowOpen>(self.window_open.listener());
    }

    fn on_window_event(&mut self, window_id: WindowId, event: &WindowEvent, _ctx: &mut AppContext) {
        match event {
            WindowEvent::RedrawRequested => {
                if let Some(window) = self.window_mut(window_id) {
                    window.redraw();
                }
            }
            WindowEvent::Resized(_new_size) => todo!(),
            WindowEvent::Moved(_new_position) => todo!(),
            WindowEvent::CloseRequested => todo!(),
            WindowEvent::HiDpiFactorChanged(_new_dpi) => todo!(),
            _ => {}
        }
    }

    fn respond(&mut self, r: &mut AppContext) {
        let requests = r.services.require::<Windows>().take_requests();
        for request in requests {
            let w = GlWindow::new(
                request.new,
                r,
                r.event_loop,
                self.event_loop_proxy.clone(),
                Arc::clone(&self.ui_threads),
            );

            let args = WindowArgs {
                timestamp: Instant::now(),
                window_id: w.id(),
            };

            r.updates.push_notify(request.notifier, args.clone());
            r.updates.push_notify(self.window_open.clone(), args.clone());
        }
    }
}

struct NewWindowRequest {
    new: Box<dyn FnOnce(&AppContext) -> UiRoot>,
    notifier: EventEmitter<WindowArgs>,
}

/// Windows service.
pub struct Windows {
    requests: Vec<NewWindowRequest>,
}

impl Service for Windows {}

impl Windows {
    fn new() -> Self {
        Windows {
            requests: Vec::default(),
        }
    }

    fn take_requests(&mut self) -> Vec<NewWindowRequest> {
        std::mem::replace(&mut self.requests, Vec::default())
    }

    /// Requests a new window. Returns a notice that gets updated once
    /// when the window is launched.
    pub fn new_window(
        &mut self,
        new_window: impl FnOnce(&AppContext) -> UiRoot + 'static,
    ) -> EventListener<WindowArgs> {
        let request = NewWindowRequest {
            new: Box::new(new_window),
            notifier: EventEmitter::new(false),
        };
        let notice = request.notifier.listener();
        self.requests.push(request);
        notice
    }
}

#[derive(Clone)]
struct Notifier {
    window_id: WindowId,
    event_loop: EventLoopProxy<WebRenderEvent>,
}
impl RenderNotifier for Notifier {
    fn clone(&self) -> Box<dyn RenderNotifier> {
        Box::new(Clone::clone(self))
    }

    fn wake_up(&self) {}

    fn new_frame_ready(&self, _: DocumentId, _scrolled: bool, _composite_needed: bool, _: Option<u64>) {
        let _ = self
            .event_loop
            .send_event(WebRenderEvent::NewFrameReady(self.window_id));
    }
}

struct GlWindow {
    context: Option<WindowedContext<NotCurrent>>,
    renderer: webrender::Renderer,
    api: Arc<RenderApi>,
    state: WindowState,
    services: WindowServices,

    root: UiRoot,
    update: UpdateFlags,
    first_draw: bool,
}

impl GlWindow {
    pub fn new(
        new_window: Box<dyn FnOnce(&AppContext) -> UiRoot>,
        ctx: &mut AppContext,
        event_loop: &EventLoopWindowTarget<WebRenderEvent>,
        event_loop_proxy: EventLoopProxy<WebRenderEvent>,
        ui_threads: Arc<ThreadPool>,
    ) -> Self {
        let root = new_window(ctx);
        let inner_size = *root.size.get(ctx.vars);
        let inner_size = LogicalSize::new(inner_size.width.into(), inner_size.height.into());
        let clear_color = *root.background_color.get(ctx.vars);

        let window_builder = WindowBuilder::new()
            .with_visible(false) // not visible until first render, to flickering
            .with_inner_size(inner_size);

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

        let dpi_factor = context.window().hidpi_factor() as f32;

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
        let (renderer, sender) = webrender::Renderer::new(gl.clone(), notifier, opts, None).unwrap();
        let api = Arc::new(sender.create_api());

        let id = context.window().id();
        let (state, services) = ctx.new_window(id, &api);

        GlWindow {
            context: Some(unsafe { context.make_not_current().unwrap() }),
            renderer,
            state,
            services,
            api,

            root,
            update: UpdateFlags::LAYOUT | UpdateFlags::RENDER,
            first_draw: true,
        }
    }

    pub fn id(&self) -> WindowId {
        self.context.as_ref().unwrap().window().id()
    }

    pub fn init(&mut self, ctx: &mut AppContext) {
        let id = self.id();
        let root = &mut self.root;

        let update = ctx.window_context(id, &mut self.state, &mut self.services, &self.api, |ctx| {
            ctx.updates.push_layout();
            ctx.updates.push_frame();

            ctx.widget_context(root.id, |ctx| {
                root.child.init(ctx);
            });
        });
        self.update |= update;
    }

    fn root_context(
        &mut self,
        ctx: &mut AppContext,
        f: impl FnOnce(&mut Box<dyn UiNode>, &mut WidgetContext),
    ) -> UpdateFlags {
        let id = self.id();
        let root = &mut self.root;

        ctx.window_context(id, &mut self.state, &mut self.services, &self.api, |ctx| {
            ctx.widget_context(root.id, |ctx| {
                f(&mut root.child, ctx);
            });
        })
    }

    pub fn update_hp(&mut self, ctx: &mut AppContext) {
        let update = self.root_context(ctx, |root, ctx| root.update_hp(ctx));
        self.update |= update;
    }

    pub fn update(&mut self, ctx: &mut AppContext) {
        // do winit window updates
        let window = self.context.as_ref().unwrap().window();
        if let Some(title) = self.root.title.update(ctx.vars) {
            window.set_title(title);
        }

        // do UiNode updates
        let update = self.root_context(ctx, |root, ctx| root.update_hp(ctx));
        self.update |= update;
    }

    pub fn layout(&mut self) {
        if self.update.contains(UpdateFlags::LAYOUT) {
            self.update.remove(UpdateFlags::LAYOUT);

            let available_size = self.context.as_ref().unwrap().window().inner_size();
            let available_size = LayoutSize::new(available_size.width as f32, available_size.height as f32);

            let desired_size = self.root.child.measure(available_size);

            let final_size = desired_size.min(available_size);

            self.root.child.arrange(final_size);
        }
    }

    pub fn render(&mut self) {
        if self.update.contains(UpdateFlags::RENDER) {
            self.update.remove(UpdateFlags::RENDER);

            let mut frame = FrameBuilder::new(self.root.id);
            self.root.child.render(&mut frame);

            todo!()
        }
    }

    /// Notifies the OS to redraw the window, will receive WindowEvent::RedrawRequested
    /// from the OS after calling this.
    pub fn request_redraw(&mut self) {
        let context = self.context.as_ref().unwrap();
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
        let context = unsafe { self.context.take().unwrap().make_current().unwrap() };

        self.renderer.update();

        let size = context.window().inner_size();
        let dpi = context.window().hidpi_factor();
        let device_size = webrender::api::DeviceIntSize::new((size.width * dpi) as i32, (size.height * dpi) as i32);

        self.renderer.render(device_size).unwrap();
        let _ = self.renderer.flush_pipeline_info();

        context.swap_buffers().ok();

        self.context = Some(unsafe { context.make_not_current().unwrap() });
    }

    pub(crate) fn deinit(mut self, ctx: &mut AppContext) {
        self.root_context(ctx, |root, ctx| root.deinit(ctx));

        let context = unsafe { self.context.take().unwrap().make_current().unwrap() };
        self.renderer.deinit();
        unsafe { context.make_not_current().unwrap() };
    }
}

pub struct UiRoot {
    id: WidgetId,
    title: BoxVar<Cow<'static, str>>,
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
