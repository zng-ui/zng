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
pub struct NewWindow;

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

    pub fn new_windows(&mut self, event_loop: &EventLoopWindowTarget<WebRenderEvent>, r: &mut EventContext) {
        let requests = std::mem::replace(&mut *self.service.requests.borrow_mut(), Vec::default());

        for request in requests {
            let w = GlWindow::new(
                request.new,
                r.app_context(),
                event_loop,
                self.event_loop_proxy.clone(),
                Arc::clone(&self.ui_threads),
            );
        }
    }
}

impl AppExtension for AppWindows {
    fn register(&mut self, r: &mut AppRegister) {
        r.register_service::<Windows>(self.service.clone());
        r.register_event::<NewWindow>(self.new_window.listener());
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

    pub fn update(&mut self, ctx: &mut AppContext) {
        let window = self.context.as_ref().unwrap().window();

        if let Some(title) = self.root.title.update(&ctx) {
            window.set_title(title);
        }

        self.root.child.update(ctx);
    }
}

pub struct UiRoot {
    title: BoxVar<Cow<'static, str>>,
    size: SharedVar<LayoutSize>,
    background_color: BoxVar<ColorF>,
    child: Box<dyn UiNode>,
}

fn window(
    child: impl UiNode,
    title: impl IntoVar<Cow<'static, str>>,
    size: impl Into<SharedVar<LayoutSize>>,
    background_color: impl IntoVar<ColorF>,
) -> UiRoot {
    UiRoot {
        title: Box::new(title.into_var()),
        size: size.into(),
        background_color: Box::new(background_color.into_var()),
        child: Box::new(child),
    }
}
