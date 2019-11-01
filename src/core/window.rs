use super::*;
use crate::app::Focused;
use crate::primitive::{FocusScope, FocusScopeExt};
use gleam::gl;
use glutin::dpi::LogicalSize;
use glutin::event::{ElementState, ScanCode, WindowEvent};
use glutin::event_loop::{EventLoopProxy, EventLoopWindowTarget};
use glutin::window::{CursorIcon, WindowBuilder, WindowId};
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

    latest_frame_id: Epoch,
    pipeline_id: PipelineId,
    renderer: webrender::Renderer,

    dpi_factor: f32,
    inner_size: LayoutSize,

    focus_map: FocusMap,
    content: FocusScope<Box<dyn Ui>>,
    content_size: LayoutSize,

    first_draw: bool,

    pub next_update: NextUpdate,
    pub redraw: bool,

    pub close: bool,

    mouse_pos: LayoutPoint,
    key_down: Option<ScanCode>,
    cursor: CursorIcon,

    ui_values: UiValues,
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

        let mut ui_values = UiValues::new();
        let mut next_update = NextUpdate::new(api, document_id);

        let mut content =
            (new_window.content)(&mut next_update).focus_scope(KeyNavigation::Both, Some(CaptureMode::Cycle));
        content.init(&mut ui_values, &mut next_update);

        Window {
            context: Some(unsafe { context.make_not_current().unwrap() }),

            latest_frame_id,
            pipeline_id,
            renderer,

            dpi_factor,
            inner_size,

            focus_map: FocusMap::new(),
            content,
            content_size: LayoutSize::default(),

            first_draw: true,
            ui_values,
            next_update,
            redraw: false,

            close: false,

            mouse_pos: LayoutPoint::new(-1., -1.),
            key_down: None,
            cursor: CursorIcon::Default,
        }
    }

    /// Processes window event, no action is done in this method, just sets flags of what needs to be done.
    pub fn event(&mut self, event: WindowEvent) -> bool {
        // has update outsize of self.next_update.
        let mut has_update = false;

        match event {
            WindowEvent::Resized(new_size) => {
                // open issue on resize delay: https://github.com/servo/webrender/issues/1640
                let new_size = LayoutSize::new(new_size.width as f32, new_size.height as f32);
                if self.inner_size != new_size {
                    self.inner_size = new_size;
                    self.next_update.update_layout();
                }
            }
            WindowEvent::HiDpiFactorChanged(new_dpi_factor) => {
                let new_dpi_factor = new_dpi_factor as f32;
                if (self.dpi_factor - new_dpi_factor).abs() > 0.01 {
                    self.dpi_factor = new_dpi_factor;
                    self.next_update.update_layout();
                }
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
                let is_pressed = input.state == ElementState::Pressed;
                // check if is auto repeat.
                let mut repeat = false;
                if is_pressed {
                    if self.key_down != Some(input.scancode) {
                        self.key_down = Some(input.scancode);
                    } else {
                        repeat = true;
                    }
                } else {
                    self.key_down = None;
                }

                // notify content
                self.content.keyboard_input(
                    &KeyboardInput {
                        scancode: input.scancode,
                        state: input.state,
                        virtual_keycode: input.virtual_keycode,
                        modifiers: input.modifiers,
                        repeat,
                    },
                    &mut self.ui_values,
                    &mut self.next_update,
                );

                // do default focus navigation
                if is_pressed
                    && self.next_update.focus_request.is_none()
                    && self.ui_values.child(*FOCUS_HANDLED).is_none()
                {
                    let request = match input.virtual_keycode {
                        Some(VirtualKeyCode::Tab) => Some(if input.modifiers.shift {
                            FocusRequest::Prev
                        } else {
                            FocusRequest::Next
                        }),
                        Some(VirtualKeyCode::Left) => Some(FocusRequest::Left),
                        Some(VirtualKeyCode::Right) => Some(FocusRequest::Right),
                        Some(VirtualKeyCode::Up) => Some(FocusRequest::Up),
                        Some(VirtualKeyCode::Down) => Some(FocusRequest::Down),
                        Some(VirtualKeyCode::Escape) => Some(FocusRequest::Escape),
                        _ => None,
                    };
                    if let Some(request) = request {
                        self.next_update.focus(request);
                    }
                }

                // clear all child values
                self.ui_values.clear_child_values();
            }
            WindowEvent::CursorMoved {
                position, modifiers, ..
            } => {
                let position = LayoutPoint::new(position.x as f32, position.y as f32);
                if self.mouse_pos != position {
                    let hit = self.hit_test(self.mouse_pos);
                    self.mouse_pos = position;
                    self.set_cursor(hit.cursor());
                    self.content.mouse_move(
                        &UiMouseMove { position, modifiers },
                        &hit,
                        &mut self.ui_values,
                        &mut self.next_update,
                    );

                    self.ui_values.clear_child_values();
                }
            }
            WindowEvent::CursorEntered { .. } => {
                self.content.mouse_entered(&mut self.ui_values, &mut self.next_update);
            }
            WindowEvent::CursorLeft { .. } => {
                self.set_cursor(CursorIcon::Default);
                self.content.mouse_left(&mut self.ui_values, &mut self.next_update);
            }
            WindowEvent::MouseInput {
                state,
                button,
                modifiers,
                ..
            } => {
                self.content.mouse_input(
                    &MouseInput {
                        state,
                        button,
                        modifiers,
                        position: self.mouse_pos,
                    },
                    &self.hit_test(self.mouse_pos),
                    &mut self.ui_values,
                    &mut self.next_update,
                );
                self.ui_values.clear_child_values()
            }
            WindowEvent::Focused(focused) => {
                if focused {
                    self.next_update.focus(FocusRequest::Direct(self.content.key()));
                } else {
                    self.key_down = None;
                }
                self.content
                    .window_focused(focused, &mut self.ui_values, &mut self.next_update);

                self.ui_values.clear_child_values();
            }
            _ => {}
        }

        has_update || self.next_update.has_update
    }

    pub fn new_window_requests(&mut self) -> Vec<NewWindow> {
        std::mem::replace(&mut self.next_update.windows, vec![])
    }

    pub fn value_changes(&mut self) -> Vec<Box<dyn VarChange>> {
        std::mem::replace(&mut self.next_update.value_changes, vec![])
    }

    fn hit_test(&self, point: LayoutPoint) -> Hits {
        Hits::new(self.next_update.api.hit_test(
            self.next_update.document_id,
            Some(self.pipeline_id),
            WorldPoint::new(point.x, point.y),
            HitTestFlags::FIND_ALL,
        ))
    }

    fn device_size(&self) -> DeviceIntSize {
        let size: LayoutSize = self.inner_size * euclid::TypedScale::new(self.dpi_factor);
        DeviceIntSize::new(size.width as i32, size.height as i32)
    }

    fn set_cursor(&mut self, cursor: CursorIcon) {
        if self.cursor != cursor {
            self.cursor = cursor;
            self.context.as_ref().unwrap().window().set_cursor_icon(cursor);
        }
    }

    pub fn update(&mut self, values_changed: bool, focused: Focused) -> bool {
        if self.next_update.has_update || values_changed {
            self.next_update.has_update = false;
            if values_changed {
                self.content.value_changed(&mut self.ui_values, &mut self.next_update);
            }
            self.update_focus(focused);
            if self.next_update.has_update {
                return true;
            }
            self.update_layout();
            self.send_render_frame();
        }

        false
    }

    fn update_focus(&mut self, focused: Focused) {
        if let Some(request) = self.next_update.focus_request.take() {
            let new_focused = self.focus_map.focus(focused.get(), request);
            if new_focused != focused.get() {
                self.activate();
                self.content.focus_changed(
                    &FocusChange::new(focused.get(), new_focused),
                    &mut self.ui_values,
                    &mut self.next_update,
                );
                focused.set(new_focused);
            }
        }
    }

    /// Bring window to foreground.
    pub fn activate(&self) {
        use raw_window_handle::*;
        use winapi::shared::windef::HWND;

        match self.context.as_ref().unwrap().window().raw_window_handle() {
            RawWindowHandle::Windows(h) => unsafe {
                winapi::um::winuser::SetForegroundWindow(h.hwnd as HWND);
            },
            _ => unimplemented!("Activate window not implemented for this OS"),
        }
    }

    /// Updates the content layout and flags `render_frame`.
    fn update_layout(&mut self) {
        if !self.next_update.update_layout {
            return;
        }
        self.next_update.update_layout = false;

        let device_size = self.device_size();

        self.next_update.api.set_window_parameters(
            self.next_update.document_id,
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
        let mut frame = NextFrame::new(
            DisplayListBuilder::new(self.pipeline_id, self.inner_size),
            SpatialId::root_reference_frame(self.pipeline_id),
            self.content_size,
        );

        self.content.render(&mut frame);

        self.latest_frame_id = Epoch({
            let mut next = self.latest_frame_id.0.wrapping_add(1);
            if next == Epoch::invalid().0 {
                next = next.wrapping_add(1);
            }
            next
        });

        let (display_list_data, focus_map) = frame.finalize();
        self.focus_map = focus_map;

        txn.set_display_list(self.latest_frame_id, None, self.inner_size, display_list_data, true);
        txn.set_root_pipeline(self.pipeline_id);
        txn.generate_frame();
        self.next_update.api.send_transaction(self.next_update.document_id, txn);
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
