use crate::ui::{AnyUi, InitContext, RenderContext, Ui};
use gleam::gl;
use glutin::dpi::LogicalSize;
use glutin::event::WindowEvent;
use glutin::event_loop::{EventLoopProxy, EventLoopWindowTarget};
use glutin::window::{WindowBuilder, WindowId};
use glutin::{Api, ContextBuilder, GlRequest};
use glutin::{NotCurrent, WindowedContext};
use rayon::ThreadPool;
use std::sync::Arc;
use webrender::api::*;

#[derive(Debug)]
pub enum WebRenderEvent {
    NewFrameReady(WindowId),
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

pub struct NextUpdate {
    update_layout: bool,
    render_frame: bool,
    _request_close: bool,
}
impl NextUpdate {
    fn update_layout(&mut self) {
        self.update_layout = true;
    }
    fn render_frame(&mut self) {
        self.render_frame = true;
    }

    //-------idea---------
    //
    //pub fn close_app(&mut self) {
    //    self.close = Some(CloseRequest::App);
    //}

    //pub fn cancel_close(&mut self) {
    //    self.cancel_close = true;
    //}

    //pub fn set_window_title(&mut self, title: String) {
    //    self.new_window_title = Some(title);
    //}

    //pub fn start_work(&mut self, work: impl FnOnce() + 'static) -> WorkKey {
    //    let key = self.next_work_key;
    //    self.new_work.push((key, Box::new(work)));
    //    self.next_work_key = WorkKey(key.0.wrapping_add(1));
    //    key
    //}

    //pub fn cancel_work(&mut self, work_key: WorkKey) {
    //    self.cancel_work.push(work_key)
    //}
}

pub struct Window {
    context: Option<WindowedContext<NotCurrent>>,

    api: RenderApi,
    document_id: DocumentId,
    latest_frame_id: Epoch,
    pipeline_id: PipelineId,
    renderer: webrender::Renderer,

    dpi_factor: f32,
    inner_size: LayoutSize,

    content: AnyUi,
    content_size: LayoutSize,

    first_draw: bool,

    pub next_update: NextUpdate,
    pub redraw: bool,

    pub close: bool,
}

impl Window {
    pub fn new(
        name: String,
        clear_color: ColorF,
        inner_size: LayoutSize,
        content: impl Fn(&mut InitContext) -> AnyUi,
        event_loop: &EventLoopWindowTarget<WebRenderEvent>,
        event_loop_proxy: EventLoopProxy<WebRenderEvent>,
        ui_threads: Arc<ThreadPool>,
    ) -> Self {
        let window_builder = WindowBuilder::new()
            .with_title(name)
            .with_visible(false)
            .with_inner_size(LogicalSize::new(inner_size.width as f64, inner_size.height as f64));

        let context = ContextBuilder::new()
            .with_gl(GlRequest::GlThenGles {
                opengl_version: (3, 2),
                opengles_version: (3, 0),
            })
            .build_windowed(window_builder, &event_loop)
            .unwrap();

        let context = unsafe { context.make_current().unwrap() };

        let gl = match context.get_api() {
            Api::OpenGl => unsafe { gl::GlFns::load_with(|symbol| context.get_proc_address(symbol) as *const _) },
            Api::OpenGlEs => unsafe { gl::GlesFns::load_with(|symbol| context.get_proc_address(symbol) as *const _) },
            Api::WebGl => unimplemented!(),
        };

        let dpi_factor = context.window().hidpi_factor() as f32;
        let device_size = {
            let size: LayoutSize = inner_size * euclid::TypedScale::new(dpi_factor);
            DeviceIntSize::new(size.width as i32, size.height as i32)
        };

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
        let document_id = api.add_document(device_size, 0);
        let latest_frame_id = Epoch(0);
        let pipeline_id = PipelineId(0, 0);

        let mut init_ctx = InitContext::new(api, document_id);

        let content = content(&mut init_ctx);
        Window {
            context: Some(unsafe { context.make_not_current().unwrap() }),

            api: init_ctx.api,
            document_id,
            latest_frame_id,
            pipeline_id,
            renderer,

            dpi_factor,
            inner_size,

            content,
            content_size: LayoutSize::default(),

            first_draw: true,
            next_update: NextUpdate {
                update_layout: true,
                render_frame: true,
                _request_close: false,
            },
            redraw: false,

            close: false,
        }
    }

    /// Processes window event, no action is done in this method, just sets flags of what needs to be done.
    pub fn event(&mut self, event: WindowEvent) -> bool {
        let mut has_update = true;
        match event {
            WindowEvent::Resized(new_size) => {
                // open issue on resize delay: https://github.com/servo/webrender/issues/1640
                self.inner_size = LayoutSize::new(new_size.width as f32, new_size.height as f32);
                self.next_update.update_layout();
            }
            WindowEvent::HiDpiFactorChanged(new_dpi_factor) => {
                self.dpi_factor = new_dpi_factor as f32;
                self.next_update.update_layout();
            }
            WindowEvent::RedrawRequested => self.redraw = true,
            WindowEvent::CloseRequested => self.close = true,

            WindowEvent::KeyboardInput { input, .. } => self.content.keyboard_input(&input, &mut self.next_update),

            _ => has_update = false,
        }
        has_update
    }

    fn device_size(&self) -> DeviceIntSize {
        let size: LayoutSize = self.inner_size * euclid::TypedScale::new(self.dpi_factor);
        DeviceIntSize::new(size.width as i32, size.height as i32)
    }

    pub fn update(&mut self) {
        self.update_layout();
        self.send_render_frame();
    }

    /// Updates the content layout and flags `render_frame`.
    fn update_layout(&mut self) {
        if !self.next_update.update_layout {
            return;
        }
        self.next_update.update_layout = false;

        let device_size = self.device_size();

        self.api.set_window_parameters(
            self.document_id,
            device_size,
            DeviceIntRect::from_size(device_size),
            self.dpi_factor,
        );

        self.content_size = self.content.measure(self.inner_size).min(self.inner_size);
        self.content.arrange(self.content_size);

        self.next_update.render_frame();
    }

    /// Generates window content display list and sends a new frame request to webrender.
    /// Webrender will request a redraw when the frame is done.
    fn send_render_frame(&mut self) {
        if !self.next_update.render_frame {
            return;
        }
        self.next_update.render_frame = false;

        let mut txn = Transaction::new();
        let mut builder = DisplayListBuilder::new(self.pipeline_id, self.inner_size);

        self.content.render(&mut RenderContext::new(
            &mut builder,
            SpatialId::root_reference_frame(self.pipeline_id),
            self.content_size,
        ));

        self.latest_frame_id = Epoch({
            let mut next = self.latest_frame_id.0.wrapping_add(1);
            if next == Epoch::invalid().0 {
                next = next.wrapping_add(1);
            }
            next
        });

        txn.set_display_list(self.latest_frame_id, None, self.inner_size, builder.finalize(), true);
        txn.set_root_pipeline(self.pipeline_id);
        txn.generate_frame();
        self.api.send_transaction(self.document_id, txn);
    }

    /// Redraws the last ready frame and swaps buffers.
    ///
    /// **`swap_buffers` Warning**: if you enabled vsync, this function will block until the
    /// next time the screen is refreshed. However drivers can choose to
    /// override your vsync settings, which means that you can't know in
    /// advance whether `swap_buffers` will block or not.
    pub fn redraw_and_swap_buffers(&mut self) {
        assert!(self.redraw);
        self.redraw = false;

        let context = unsafe { self.context.take().unwrap().make_current().unwrap() };
        self.renderer.update();
        self.renderer.render(self.device_size()).unwrap();
        let _ = self.renderer.flush_pipeline_info();
        context.swap_buffers().ok();
        self.context = Some(unsafe { context.make_not_current().unwrap() });
    }

    pub fn request_redraw(&mut self) {
        let context = self.context.as_ref().unwrap();
        if self.first_draw {
            context.window().set_visible(true); // OS generates a RequestRedraw here
            self.first_draw = false;
        } else {
            context.window().request_redraw();
        }
    }

    pub fn deinit(mut self) {
        let context = unsafe { self.context.take().unwrap().make_current().unwrap() };
        self.renderer.deinit();
        unsafe { context.make_not_current().unwrap() };
    }

    pub fn id(&self) -> WindowId {
        self.context.as_ref().unwrap().window().id()
    }
}
