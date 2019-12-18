use super::*;
use crate::app::Focused;
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
pub(crate) enum WebRenderEvent {
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

pub struct NewWindow {
    pub content: Box<dyn FnOnce(&mut NextUpdate) -> Box<dyn Ui>>,
    pub clear_color: ColorF,
    pub inner_size: LayoutSize,
}

pub(crate) struct Window {
    context: Option<WindowedContext<NotCurrent>>,
    renderer: webrender::Renderer,
    root: UiRoot,

    first_draw: bool,
    pub redraw: bool,
    pub close: bool,
}

impl Window {
    pub fn new(
        new_window: NewWindow,
        event_loop: &EventLoopWindowTarget<WebRenderEvent>,
        event_loop_proxy: EventLoopProxy<WebRenderEvent>,
        ui_threads: Arc<ThreadPool>,
    ) -> Self {
        let inner_size = new_window.inner_size;
        let clear_color = new_window.clear_color;

        let window_builder = WindowBuilder::new()
            .with_visible(false)
            .with_inner_size(LogicalSize::new(
                f64::from(inner_size.width),
                f64::from(inner_size.height),
            ));

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

        let root = UiRoot::new(api, sender, inner_size, dpi_factor, new_window.content);

        Window {
            context: Some(unsafe { context.make_not_current().unwrap() }),
            renderer,
            root,
            first_draw: true,
            redraw: false,
            close: false,
        }
    }

    pub fn take_new_window_requests(&mut self) -> Vec<NewWindow> {
        self.root.take_new_window_requests()
    }

    pub fn take_var_changes(&mut self) -> Vec<Box<dyn ValueMutCommit>> {
        self.root.take_var_changes()
    }

    /// Processes window event, no action is done in this method, just sets flags of what needs to be done.
    pub fn event(&mut self, event: WindowEvent) -> bool {
        let mut has_update = false;

        match event {
            WindowEvent::Resized(new_size) => {
                self.root
                    .resize(LayoutSize::new(new_size.width as f32, new_size.height as f32));
            }
            WindowEvent::HiDpiFactorChanged(new_dpi_factor) => {
                self.root.set_dpi_factor(new_dpi_factor as f32);
            }
            WindowEvent::RedrawRequested => {
                self.redraw = true;
                has_update = true;
            }
            WindowEvent::CloseRequested => {
                self.close = true;
                has_update = true;
            }

            WindowEvent::KeyboardInput { input, .. } => {
                self.root
                    .keyboard_input(input.scancode, input.state, input.virtual_keycode, input.modifiers)
            }
            WindowEvent::CursorMoved {
                position, modifiers, ..
            } => {
                self.root
                    .mouse_move(LayoutPoint::new(position.x as f32, position.y as f32), modifiers);
            }
            WindowEvent::CursorEntered { .. } => {
                self.root.mouse_entered();
            }
            WindowEvent::CursorLeft { .. } => {
                self.root.mouse_left();
            }
            WindowEvent::MouseInput {
                state,
                button,
                modifiers,
                ..
            } => {
                self.root.mouse_input(state, button, modifiers);
            }
            WindowEvent::Focused(focused) => {
                self.root.window_focused(focused);
            }
            _ => {}
        }

        if let Some(cursor) = self.root.take_set_cursor() {
            self.context.as_ref().unwrap().window().set_cursor_icon(cursor);
        }

        has_update || self.root.has_update()
    }

    pub fn update(&mut self, values_changed: bool, focused: Focused) -> bool {
        match self.root.update(values_changed, focused, self.first_draw) {
            UiUpdateResult::Completed => false,
            UiUpdateResult::CausedMoreUpdates => true,
        }
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
        self.renderer.render(self.root.device_size()).unwrap();
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
