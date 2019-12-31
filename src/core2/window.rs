use super::*;
use gleam::gl;
use glutin::dpi::LogicalSize;
use glutin::event_loop::{EventLoopProxy, EventLoopWindowTarget};
use glutin::window::WindowBuilder;
use glutin::{Api, ContextBuilder, GlRequest};
use glutin::{NotCurrent, WindowedContext};
use rayon::{ThreadPool, ThreadPoolBuilder};
use std::borrow::Cow;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Instant;
use webrender::api::{DocumentId, RenderNotifier};

pub use webrender::api::ColorF;

/// New window event.
pub enum NewWindow {}

/// [NewWindow] event args.
#[derive(Debug, Clone)]
pub struct NewWindowArgs {
    pub timestamp: Instant,
    pub window_id: WindowId,
}
impl EventArgs for NewWindowArgs {
    fn timestamp(&self) -> Instant {
        self.timestamp
    }
}
impl Event for NewWindow {
    type Args = NewWindowArgs;
}

/// Windows management [AppExtension].
pub(crate) struct AppWindows {
    event_loop_proxy: EventLoopProxy<WebRenderEvent>,
    ui_threads: Arc<ThreadPool>,
    service: Windows,
    windows: Vec<GlWindow>,
    new_window: EventEmitter<NewWindowArgs>,
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
            service: Windows::default(),
            windows: Vec::with_capacity(1),
            new_window: EventEmitter::new(false),
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
}

impl AppExtension for AppWindows {
    fn register(&mut self, r: &mut AppRegister) {
        r.register_service::<Windows>(self.service.clone());
        r.register_event::<NewWindow>(self.new_window.listener());
    }

    fn on_window_event(&mut self, window_id: WindowId, event: &WindowEvent, _ctx: &mut EventContext) {
        if let WindowEvent::RedrawRequested = event {
            for window in self.windows.iter_mut() {
                if window.id() == window_id {
                    window.redraw();
                    break;
                }
            }
        }
    }

    fn respond(&mut self, r: &mut EventContext) {
        let requests = std::mem::replace(&mut *self.service.requests.borrow_mut(), Vec::default());

        for request in requests {
            let w = GlWindow::new(
                request.new,
                r.app_ctx(),
                r.event_loop(),
                self.event_loop_proxy.clone(),
                Arc::clone(&self.ui_threads),
            );

            todo!()
        }
    }
}

struct NewWindowRequest {
    new: Box<dyn FnOnce(&AppContext) -> UiRoot>,
    notifier: EventEmitter<NewWindowArgs>,
}

/// Windows service.
#[derive(Clone, Default)]
pub struct Windows {
    requests: Rc<RefCell<Vec<NewWindowRequest>>>,
}

impl Service for Windows {}

impl Windows {
    /// Requests a new window. Returns a notice that gets updated once
    /// when the window is launched.
    pub fn new_window(&self, new_window: impl FnOnce(&AppContext) -> UiRoot + 'static) -> EventListener<NewWindowArgs> {
        let request = NewWindowRequest {
            new: Box::new(new_window),
            notifier: EventEmitter::new(false),
        };
        let notice = request.notifier.listener();
        self.requests.borrow_mut().push(request);
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

    root: UiRoot,
    update: UpdateFlags,
    first_draw: bool,
}

impl GlWindow {
    pub fn new(
        new_window: impl FnOnce(&AppContext) -> UiRoot,
        ctx: &AppContext,
        event_loop: &EventLoopWindowTarget<WebRenderEvent>,
        event_loop_proxy: EventLoopProxy<WebRenderEvent>,
        ui_threads: Arc<ThreadPool>,
    ) -> Self {
        let root = new_window(ctx);
        let inner_size = *root.size.get(ctx);
        let inner_size = LogicalSize::new(inner_size.width.into(), inner_size.height.into());
        let clear_color = *root.background_color.get(ctx);

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
        let api = sender.create_api();

        todo!()
    }

    pub fn id(&self) -> WindowId {
        self.context.as_ref().unwrap().window().id()
    }

    pub fn update_hp(&mut self, ctx: &mut AppContext) {
        let update = ctx.window_update(|ctx| self.root.child.update_hp(ctx));
        self.update |= update;
    }

    pub fn update(&mut self, ctx: &mut AppContext) {
        // do winit window updates
        let window = self.context.as_ref().unwrap().window();
        if let Some(title) = self.root.title.update(&ctx) {
            window.set_title(title);
        }

        // do UiNode updates
        let update = ctx.window_update(|ctx| self.root.child.update(ctx));
        self.update |= update;
    }

    pub fn layout(&mut self) {
        if self.update.contains(UpdateFlags::LAYOUT) {
            self.update.remove(UpdateFlags::LAYOUT);

            //self.root.child.measure()
            todo!()
        }
    }

    pub fn render(&mut self) {
        if self.update.contains(UpdateFlags::RENDER) {
            self.update.remove(UpdateFlags::RENDER);

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
}

pub struct UiRoot {
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
