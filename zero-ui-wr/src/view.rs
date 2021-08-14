use crate::{message::*, MODE_VAR, VERSION};

use gleam::gl;
use glutin::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{DeviceEvent, DeviceId, Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop, EventLoopProxy, EventLoopWindowTarget},
    window::{WindowBuilder, WindowId},
    Api as GApi, ContextBuilder, ContextWrapper, NotCurrent,
};
use ipmpsc::{Receiver, Sender, SharedRingBuffer};
use std::{cell::Cell, env, path::PathBuf, process, rc::Rc, thread};
use webrender::{
    api::{
        units::{self, DeviceIntRect, DeviceIntSize, LayoutPoint, LayoutSize},
        BuiltDisplayList, ColorF, DocumentId, DynamicProperties, Epoch, HitTestFlags, HitTestResult, PipelineId, RenderApi, RenderNotifier,
        Transaction,
    },
    euclid, Renderer, RendererKind, RendererOptions,
};

/// Start the app event loop in the View Process.
pub fn run(channel_dir: PathBuf) -> ! {
    if !is_main_thread::is_main_thread().unwrap_or(true) {
        panic!("can only init view-process in the main thread")
    }

    let mode = env::var(MODE_VAR).unwrap_or_else(|_| "headed".to_owned());
    let headless = match mode.as_str() {
        "headed" => false,
        "headless" => true,
        _ => panic!("unknown mode"),
    };

    let request_receiver = Receiver::new(
        SharedRingBuffer::create(&channel_dir.join("request").display().to_string(), MAX_REQUEST_SIZE)
            .expect("request channel connection failed"),
    );
    let response_sender = Sender::new(
        SharedRingBuffer::create(&channel_dir.join("response").display().to_string(), MAX_RESPONSE_SIZE)
            .expect("response channel connection failed"),
    );
    let event_sender = Sender::new(
        SharedRingBuffer::create(&channel_dir.join("event").display().to_string(), MAX_EVENT_SIZE)
            .expect("event channel connection failed"),
    );

    let event_loop = EventLoop::<AppEvent>::with_user_event();

    let request_sender = event_loop.create_proxy();
    thread::spawn(move || {
        loop {
            match request_receiver.recv() {
                Ok(req) => {
                    if request_sender.send_event(AppEvent::Request(req)).is_err() {
                        // event-loop shutdown
                        return;
                    }
                }
                Err(e) => {
                    eprintln!("request channel error:\n{:#?}", e);
                    process::exit(i32::from_ne_bytes(*b"requ"));
                }
            }
        }
    });

    let mut app = App::new(response_sender, event_sender);

    #[cfg(windows)]
    let config_listener = config_listener(event_loop.create_proxy(), &event_loop);

    let el = event_loop.create_proxy();

    event_loop.run(move |event, target, control| {
        *control = ControlFlow::Wait;
        match event {
            Event::NewEvents(_) => {}
            Event::WindowEvent { window_id, event } => {
                #[cfg(windows)]
                if window_id == config_listener.id() {
                    return; // ignore events for this window.
                }

                app.on_window_event(window_id, event)
            }
            Event::DeviceEvent { device_id, event } => app.on_device_event(device_id, event),
            Event::UserEvent(ev) => match ev {
                AppEvent::Request(req) => app.on_request(req, &el, target),
                AppEvent::FrameReady(window_id) => app.on_frame_ready(window_id),
                AppEvent::SystemFontsChanged => app.notify(Ev::FontsChanged),
                AppEvent::SystemTextAaChanged(aa) => app.notify(Ev::TextAaChanged(aa)),
                AppEvent::KeyboardInput(w_id, d_id, k) => app.notify(Ev::KeyboardInput(w_id, d_id, k)),
            },
            Event::Suspended => {}
            Event::Resumed => {}
            Event::MainEventsCleared => {}
            Event::RedrawRequested(w) => app.redraw(w),
            Event::RedrawEventsCleared => {}
            Event::LoopDestroyed => panic!("unexpected event loop shutdown"),
        }
    })
}

/// Custom event loop event.
enum AppEvent {
    Request(Request),
    FrameReady(WindowId),
    SystemFontsChanged,
    SystemTextAaChanged(TextAntiAliasing),
    KeyboardInput(WinId, DevId, KeyboardInput),
}

struct App {
    response_sender: Sender,
    event_sender: Sender,

    started: bool,
    device_events: bool,

    window_id_count: WinId,
    device_id_count: DevId,
    windows: Vec<Window>,
    devices: Vec<Device>,
}
impl App {
    fn new(response_sender: Sender, event_sender: Sender) -> Self {
        Self {
            response_sender,
            event_sender,
            started: false,
            device_events: false,
            window_id_count: 0,
            device_id_count: 0,
            windows: vec![],
            devices: vec![],
        }
    }

    fn respond(&self, response: Response) {
        self.response_sender.send_when_empty(&response).unwrap();
    }

    fn notify(&self, event: Ev) {
        self.event_sender.send_when_empty(&event).unwrap();
    }

    fn device_id(&mut self, device_id: DeviceId) -> DevId {
        if let Some(r) = self.devices.iter().find(|d| d.device_id == device_id) {
            r.id
        } else {
            self.device_id_count = self.device_id_count.wrapping_add(1);
            let id = self.device_id_count;
            self.devices.push(Device { id, device_id });
            id
        }
    }

    pub fn on_request(&mut self, request: Request, event_loop: &EventLoopProxy<AppEvent>, target: &EventLoopWindowTarget<AppEvent>) {
        if self.started {
            match request {
                Request::Start(_) => panic!("already started"),
                Request::OpenWindow(req) => self.open_window(req, event_loop.clone(), target),
                Request::SetWindowTitle(id, title) => self.set_window_title(id, title),
                Request::SetWindowPosition(id, pos) => self.set_window_position(id, pos),
                Request::SetWindowSize(id, size) => self.set_window_size(id, size),
                Request::SetWindowVisible(id, visible) => self.set_window_visible(id, visible),
                Request::AllowAltF4(id, allow) => self.allow_alt_f4(id, allow),
                Request::HitTest(id, point) => self.hit_test(id, point),
                Request::ReadPixels(id, rect) => self.read_pixels(id, rect),
                Request::CloseWindow(id) => self.close_window(id),
                Request::TextAa => self.respond(Response::TextAa(system_text_aa())),
                Request::Shutdown => process::exit(0),
                Request::ProtocolVersion => self.respond(Response::ProtocolVersion(VERSION.to_owned())),
            }
        } else if let Request::Start(r) = request {
            self.started = true;
            self.device_events = r.device_events;
            self.respond(Response::Started);
        } else if let Request::ProtocolVersion = request {
            self.respond(Response::ProtocolVersion(VERSION.to_owned()));
        } else {
            panic!("not started");
        }
    }

    pub fn on_window_event(&mut self, window: WindowId, event: WindowEvent) {
        if let Some((i, w)) = self.windows.iter_mut().enumerate().find(|(_, w)| w.winit_window.id() == window) {
            let id = w.id;
            match event {
                WindowEvent::Resized(s) => {
                    let s = (s.width, s.height);
                    w.resize(s);
                    self.notify(Ev::WindowResized(id, s))
                }
                WindowEvent::Moved(p) => self.notify(Ev::WindowMoved(id, (p.x, p.y))),
                WindowEvent::CloseRequested => self.notify(Ev::WindowCloseRequested(id)),
                WindowEvent::Destroyed => {
                    self.windows.remove(i);
                    self.notify(Ev::WindowClosed(id));
                }
                WindowEvent::DroppedFile(file) => self.notify(Ev::DroppedFile(id, file)),
                WindowEvent::HoveredFile(file) => self.notify(Ev::HoveredFile(id, file)),
                WindowEvent::HoveredFileCancelled => self.notify(Ev::HoveredFileCancelled(id)),
                WindowEvent::ReceivedCharacter(c) => self.notify(Ev::ReceivedCharacter(id, c)),
                WindowEvent::Focused(focused) => self.notify(Ev::Focused(id, focused)),
                WindowEvent::KeyboardInput { device_id, input, .. } => {
                    let d_id = self.device_id(device_id);
                    self.notify(Ev::KeyboardInput(id, d_id, input))
                }
                WindowEvent::ModifiersChanged(m) => self.notify(Ev::ModifiersChanged(id, m)),
                WindowEvent::CursorMoved { device_id, position, .. } => {
                    let d_id = self.device_id(device_id);
                    self.notify(Ev::CursorMoved(id, d_id, (position.x as i32, position.y as i32)));
                }
                WindowEvent::CursorEntered { device_id } => {
                    let d_id = self.device_id(device_id);
                    self.notify(Ev::CursorEntered(id, d_id));
                }
                WindowEvent::CursorLeft { device_id } => {
                    let d_id = self.device_id(device_id);
                    self.notify(Ev::CursorLeft(id, d_id));
                }
                WindowEvent::MouseWheel {
                    device_id, delta, phase, ..
                } => {
                    let d_id = self.device_id(device_id);
                    self.notify(Ev::MouseWheel(id, d_id, delta, phase));
                }
                WindowEvent::MouseInput {
                    device_id, state, button, ..
                } => {
                    let d_id = self.device_id(device_id);
                    self.notify(Ev::MouseInput(id, d_id, state, button));
                }
                WindowEvent::TouchpadPressure {
                    device_id,
                    pressure,
                    stage,
                } => {
                    let d_id = self.device_id(device_id);
                    self.notify(Ev::TouchpadPressure(id, d_id, pressure, stage));
                }
                WindowEvent::AxisMotion { device_id, axis, value } => {
                    let d_id = self.device_id(device_id);
                    self.notify(Ev::AxisMotion(id, d_id, axis, value));
                }
                WindowEvent::Touch(t) => {
                    let d_id = self.device_id(t.device_id);
                    self.notify(Ev::Touch(
                        id,
                        d_id,
                        t.phase,
                        (t.location.x as u32, t.location.y as u32),
                        t.force.map(Into::into),
                        t.id,
                    ));
                }
                WindowEvent::ScaleFactorChanged {
                    scale_factor,
                    new_inner_size,
                } => self.notify(Ev::ScaleFactorChanged(
                    id,
                    scale_factor as f32,
                    (new_inner_size.width, new_inner_size.height),
                )),
                WindowEvent::ThemeChanged(t) => self.notify(Ev::ThemeChanged(id, t.into())),
            }
        }
    }

    pub fn on_device_event(&mut self, device: DeviceId, event: DeviceEvent) {
        if self.device_events {
            let d_id = self.device_id(device);
            match event {
                DeviceEvent::Added => self.notify(Ev::DeviceAdded(d_id)),
                DeviceEvent::Removed => self.notify(Ev::DeviceRemoved(d_id)),
                DeviceEvent::MouseMotion { delta } => self.notify(Ev::DeviceMouseMotion(d_id, delta)),
                DeviceEvent::MouseWheel { delta } => self.notify(Ev::DeviceMouseWheel(d_id, delta)),
                DeviceEvent::Motion { axis, value } => self.notify(Ev::DeviceMotion(d_id, axis, value)),
                DeviceEvent::Button { button, state } => self.notify(Ev::DeviceButton(d_id, button, state)),
                DeviceEvent::Key(k) => self.notify(Ev::DeviceKey(d_id, k)),
                DeviceEvent::Text { codepoint } => self.notify(Ev::DeviceText(d_id, codepoint)),
            }
        }
    }

    pub fn on_frame_ready(&mut self, window: WindowId) {
        if let Some(w) = self.windows.iter_mut().find(|w| w.winit_window.id() == window) {
            w.winit_window.request_redraw();
        }
    }

    pub fn redraw(&mut self, window: WindowId) {
        if let Some(w) = self.windows.iter_mut().find(|w| w.winit_window.id() == window) {
            w.redraw();
        }
    }

    fn open_window(&mut self, request: OpenWindowRequest, event_loop: EventLoopProxy<AppEvent>, target: &EventLoopWindowTarget<AppEvent>) {
        self.window_id_count = self.window_id_count.wrapping_add(1);
        let id = self.window_id_count;
        self.windows.push(Window::new(id, request, event_loop, target));
        self.respond(Response::WindowOpened(id));
    }

    fn set_window_title(&self, id: WinId, title: String) {
        if let Some(w) = self.windows.iter().find(|w| w.id == id) {
            w.winit_window.set_title(&title);
            self.respond(Response::WindowTitleChanged(id));
        } else {
            self.respond(Response::WindowNotFound(id));
        }
    }

    fn set_window_position(&self, id: WinId, (x, y): (i32, i32)) {
        if let Some(w) = self.windows.iter().find(|w| w.id == id) {
            w.winit_window.set_outer_position(PhysicalPosition::new(x, y));
            self.respond(Response::WindowTitleChanged(id));
        } else {
            self.respond(Response::WindowNotFound(id));
        }
    }

    fn set_window_size(&mut self, id: WinId, size: (u32, u32)) {
        if let Some(w) = self.windows.iter_mut().find(|w| w.id == id) {
            w.resize(size);
            self.respond(Response::WindowTitleChanged(id));
        } else {
            self.respond(Response::WindowNotFound(id));
        }
    }

    fn set_window_visible(&mut self, id: WinId, visible: bool) {
        if let Some(w) = self.windows.iter_mut().find(|w| w.id == id) {
            w.set_visible(visible);
            self.respond(Response::WindowVisibilityChanged(id, visible));
        } else {
            self.respond(Response::WindowNotFound(id));
        }
    }

    fn allow_alt_f4(&mut self, id: WinId, allow: bool) {
        if let Some(w) = self.windows.iter_mut().find(|w| w.id == id) {
            w.allow_alt_f4.set(allow);
            self.respond(Response::AllowAltF4Changed(id, allow));
        } else {
            self.respond(Response::WindowNotFound(id));
        }
    }

    fn hit_test(&mut self, id: WinId, point: LayoutPoint) {
        if let Some(w) = self.windows.iter_mut().find(|w| w.id == id) {
            let r = w.hit_test(point);
            self.respond(Response::HitTestResult(id, r));
        } else {
            self.respond(Response::WindowNotFound(id));
        }
    }

    fn read_pixels(&mut self, id: WinId, [x, y, width, height]: [u32; 4]) {
        if let Some(w) = self.windows.iter_mut().find(|w| w.id == id) {
            let r = w.read_pixels(x, y, width, height);
            self.respond(Response::FramePixels(id, r));
        } else {
            self.respond(Response::WindowNotFound(id));
        }
    }

    fn close_window(&mut self, id: WinId) {
        if let Some(i) = self.windows.iter().position(|w| w.id == id) {
            let _ = self.windows.remove(i);
            self.respond(Response::WindowClosed(id));
        } else {
            self.respond(Response::WindowNotFound(id));
        }
    }
}

struct Window {
    id: WinId,
    winit_window: glutin::window::Window,
    context: Option<ContextWrapper<NotCurrent, ()>>,
    gl: Rc<dyn gl::Gl>,
    renderer: Option<Renderer>,
    api: RenderApi,

    pipeline_id: PipelineId,
    document_id: DocumentId,
    clear_color: Option<ColorF>,

    resized: bool,

    visisble: bool,
    waiting_first_frame: bool,

    allow_alt_f4: Rc<Cell<bool>>,
}
impl Window {
    fn new(id: u32, request: OpenWindowRequest, event_loop: EventLoopProxy<AppEvent>, target: &EventLoopWindowTarget<AppEvent>) -> Self {
        // create window and OpenGL context
        let winit = WindowBuilder::new()
            .with_title(request.title)
            .with_position(PhysicalPosition::new(request.pos.0, request.pos.1))
            .with_inner_size(PhysicalSize::new(request.size.0, request.size.1))
            .with_visible(false); // we wait for the first frame to show the window.

        let glutin = ContextBuilder::new().build_windowed(winit, target).unwrap();
        // SAFETY: we drop the context before the window.
        let (context, winit_window) = unsafe { glutin.split() };

        // extend the winit Windows window to only block the Alt+F4 key press if we want it to.
        let allow_alt_f4 = Rc::new(Cell::new(false));
        #[cfg(windows)]
        {
            let allow_alt_f4 = allow_alt_f4.clone();
            let event_loop = event_loop.clone();

            set_raw_windows_event_handler(&winit_window, u32::from_ne_bytes(*b"alf4") as _, move |_, msg, wparam, _| {
                if msg == winapi::um::winuser::WM_SYSKEYDOWN && wparam as i32 == winapi::um::winuser::VK_F4 && allow_alt_f4.get() {
                    let device_id = 0; // TODO recover actual ID

                    #[allow(deprecated)] // `modifiers` is deprecated but there is no other way to init a KeyboardInput
                    let _ = event_loop.send_event(AppEvent::KeyboardInput(
                        id,
                        device_id,
                        KeyboardInput {
                            scancode: wparam as u32,
                            state: ElementState::Pressed,
                            virtual_keycode: Some(VirtualKeyCode::F4),
                            modifiers: ModifiersState::ALT,
                        },
                    ));
                    return Some(0);
                }
                None
            });
        }

        // create renderer and start the first frame.
        let context = unsafe { context.make_current() }.unwrap();

        let gl = match context.get_api() {
            GApi::OpenGl => unsafe { gl::GlFns::load_with(|symbol| context.get_proc_address(symbol) as *const _) },
            GApi::OpenGlEs => unsafe { gl::GlesFns::load_with(|symbol| context.get_proc_address(symbol) as *const _) },
            GApi::WebGl => panic!("WebGl is not supported"),
        };

        let device_size = winit_window.inner_size();
        let device_size = DeviceIntSize::new(device_size.width as i32, device_size.height as i32);

        let opts = RendererOptions {
            device_pixel_ratio: winit_window.scale_factor() as f32,
            renderer_kind: RendererKind::Native,
            clear_color: request.clear_color,
            enable_aa: request.text_aa != TextAntiAliasing::Mono,
            enable_subpixel_aa: request.text_aa == TextAntiAliasing::Subpixel,
            //panic_on_gl_error: true,
            // TODO expose more options to the user.
            ..Default::default()
        };

        let (renderer, sender) = webrender::Renderer::new(
            Rc::clone(&gl),
            Box::new(Notifier(winit_window.id(), event_loop)),
            opts,
            None,
            device_size,
        )
        .unwrap();

        let api = sender.create_api();
        let document_id = api.add_document(device_size, 0);

        let pipeline_id = webrender::api::PipelineId(1, 0);

        let context = unsafe { context.make_not_current() }.unwrap();

        Self {
            id,
            winit_window,
            context: Some(context),
            gl,
            renderer: Some(renderer),
            api,
            document_id,
            pipeline_id,
            resized: false,
            clear_color: request.clear_color,
            waiting_first_frame: false,
            visisble: request.visible,
            allow_alt_f4,
        }
    }

    fn resize(&mut self, (w, h): (u32, u32)) {
        let size = PhysicalSize::new(w, h);
        self.winit_window.set_inner_size(size);
        let ctx = unsafe { self.context.take().unwrap().make_current().unwrap() };
        ctx.resize(size);
        self.context = unsafe { Some(ctx.make_not_current().unwrap()) };
        self.resized = true;
    }

    fn set_visible(&mut self, visible: bool) {
        if !self.waiting_first_frame {
            self.winit_window.set_visible(visible);
        }
        self.visisble = visible;
    }

    /// Start rendering a new frame.
    ///
    /// The [callback](#callback) will be called when the frame is ready to be [presented](Self::present).
    fn render(&mut self, display_list_data: (PipelineId, LayoutSize, BuiltDisplayList), frame_id: Epoch) {
        let scale_factor = self.winit_window.scale_factor() as f32;
        let size = self.winit_window.inner_size();
        let viewport_size = LayoutSize::new(size.width as f32 * scale_factor, size.height as f32 * scale_factor);

        let mut txn = Transaction::new();
        txn.set_display_list(frame_id, self.clear_color, viewport_size, display_list_data, true);
        txn.set_root_pipeline(self.pipeline_id);

        if self.resized {
            self.resized = false;
            txn.set_document_view(
                DeviceIntRect::new(euclid::point2(0, 0), euclid::size2(size.width as i32, size.height as i32)),
                scale_factor,
            );
        }

        txn.generate_frame();
        self.api.send_transaction(self.document_id, txn);
    }

    /// Start rendering a new frame based on the data of the last frame.
    fn render_update(&mut self, updates: DynamicProperties) {
        let mut txn = Transaction::new();
        txn.set_root_pipeline(self.pipeline_id);
        txn.update_dynamic_properties(updates);

        if self.resized {
            self.resized = false;
            let scale_factor = self.winit_window.scale_factor() as f32;
            let size = self.winit_window.inner_size();
            txn.set_document_view(
                DeviceIntRect::new(euclid::point2(0, 0), euclid::size2(size.width as i32, size.height as i32)),
                scale_factor,
            );
        }

        txn.generate_frame();
        self.api.send_transaction(self.document_id, txn);
    }

    fn redraw(&mut self) {
        let ctx = unsafe { self.context.take().unwrap().make_current() }.unwrap();
        let renderer = self.renderer.as_mut().unwrap();
        renderer.update();
        let s = self.winit_window.inner_size();
        renderer.render(DeviceIntSize::new(s.width as i32, s.height as i32)).unwrap();
        ctx.swap_buffers().unwrap();
        self.context = Some(unsafe { ctx.make_not_current() }.unwrap());

        if self.waiting_first_frame {
            self.waiting_first_frame = false;
            if self.visisble {
                self.winit_window.set_visible(true);
            }
        }
    }

    /// Does a hit-test on the current frame.
    fn hit_test(&self, point: LayoutPoint) -> HitTestResult {
        self.api.hit_test(
            self.document_id,
            Some(self.pipeline_id),
            units::WorldPoint::new(point.x, point.y),
            HitTestFlags::all(),
        )
    }

    /// `glReadPixels` a new buffer.
    ///
    /// This is a direct call to `glReadPixels`, `x` and `y` start
    /// at the bottom-left corner of the rectangle and each *stride*
    /// is a row from bottom-to-top and the pixel type is BGRA.
    fn read_pixels(&mut self, x: u32, y: u32, width: u32, height: u32) -> Vec<u8> {
        let ctx = unsafe { self.context.take().unwrap().make_current() }.unwrap();

        let pixels = self
            .gl
            .read_pixels(x as _, y as _, width as _, height as _, gl::BGRA, gl::UNSIGNED_BYTE);
        assert!(self.gl.get_error() == 0);

        self.context = Some(unsafe { ctx.make_not_current() }.unwrap());

        pixels
    }
}
impl Drop for Window {
    fn drop(&mut self) {
        self.renderer.take().unwrap().deinit();
        // context must be dropped before the window.
        drop(self.context.take());
    }
}

struct Notifier(WindowId, EventLoopProxy<AppEvent>);
impl RenderNotifier for Notifier {
    fn clone(&self) -> Box<dyn RenderNotifier> {
        Box::new(Self(self.0, self.1.clone()))
    }

    fn wake_up(&self) {}

    fn new_frame_ready(&self, _: webrender::api::DocumentId, _: bool, _: bool, _: Option<u64>) {
        let _ = self.1.send_event(AppEvent::FrameReady(self.0));
    }
}

struct Device {
    id: DevId,
    device_id: DeviceId,
}

/// Create a hidden window that listen to Windows config change events.
#[cfg(windows)]
fn config_listener(event_proxy: EventLoopProxy<AppEvent>, window_target: &EventLoopWindowTarget<AppEvent>) -> glutin::window::Window {
    let w = WindowBuilder::new()
        .with_title("config-event-listener")
        .with_visible(false)
        .build(window_target)
        .unwrap();

    set_raw_windows_event_handler(&w, u32::from_ne_bytes(*b"cevl") as _, move |_, msg, wparam, _| {
        if msg == winapi::um::winuser::WM_FONTCHANGE {
            let _ = event_proxy.send_event(AppEvent::SystemFontsChanged);
            Some(0)
        } else if msg == winapi::um::winuser::WM_SETTINGCHANGE {
            if wparam == winapi::um::winuser::SPI_GETFONTSMOOTHING as usize
                || wparam == winapi::um::winuser::SPI_GETFONTSMOOTHINGTYPE as usize
            {
                let _ = event_proxy.send_event(AppEvent::SystemTextAaChanged(system_text_aa()));
                Some(0)
            } else {
                None
            }
        } else {
            None
        }
    });

    w
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
#[cfg(windows)]
pub fn set_raw_windows_event_handler<
    H: FnMut(
            winapi::shared::windef::HWND,
            winapi::shared::minwindef::UINT,
            winapi::shared::minwindef::WPARAM,
            winapi::shared::minwindef::LPARAM,
        ) -> Option<winapi::shared::minwindef::LRESULT>
        + 'static,
>(
    window: &glutin::window::Window,
    subclass_id: winapi::shared::basetsd::UINT_PTR,
    handler: H,
) -> bool {
    use glutin::platform::windows::WindowExtWindows;

    let hwnd = window.hwnd() as winapi::shared::windef::HWND;
    let data = Box::new(handler);
    unsafe {
        winapi::um::commctrl::SetWindowSubclass(
            hwnd,
            Some(subclass_raw_event_proc::<H>),
            subclass_id,
            Box::into_raw(data) as winapi::shared::basetsd::DWORD_PTR,
        ) != 0
    }
}
#[cfg(windows)]
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

/// Gets the system text anti-aliasing config.
#[cfg(windows)]
fn system_text_aa() -> TextAntiAliasing {
    use winapi::um::errhandlingapi::GetLastError;
    use winapi::um::winuser::{SystemParametersInfoW, FE_FONTSMOOTHINGCLEARTYPE, SPI_GETFONTSMOOTHING, SPI_GETFONTSMOOTHINGTYPE};

    unsafe {
        let mut enabled = 0;
        let mut smoothing_type: u32 = 0;

        if SystemParametersInfoW(SPI_GETFONTSMOOTHING, 0, &mut enabled as *mut _ as *mut _, 0) == 0 {
            log::error!("SPI_GETFONTSMOOTHING error: {:X}", GetLastError());
            return TextAntiAliasing::Mono;
        }
        if enabled == 0 {
            return TextAntiAliasing::Mono;
        }

        if SystemParametersInfoW(SPI_GETFONTSMOOTHINGTYPE, 0, &mut smoothing_type as *mut _ as *mut _, 0) == 0 {
            log::error!("SPI_GETFONTSMOOTHINGTYPE error: {:X}", GetLastError());
            return TextAntiAliasing::Mono;
        }

        if smoothing_type == FE_FONTSMOOTHINGCLEARTYPE {
            TextAntiAliasing::Subpixel
        } else {
            TextAntiAliasing::Alpha
        }
    }
}
#[cfg(not(windows))]
fn system_text_aa() -> TextAntiAliasing {
    // TODO
    TextAntiAliasing::Subpixel
}
