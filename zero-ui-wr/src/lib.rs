//! Windowing and renderer.
//!
//! Zero-Ui isolates all OpenGL related code to a different process to be able to recover from driver errors.
//! This crate contains the `glutin` and `webrender` code that interacts with the actual system. Communication
//! with the app process is done using `ipmpsc`.

use glutin::{
    event::{DeviceEvent, DeviceId, Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop, EventLoopWindowTarget},
    window::WindowId,
};
use ipmpsc::{Receiver, Sender, SharedRingBuffer};
use serde::*;
use std::{env, mem, path::PathBuf, process, thread};

const CHANNEL_VAR: &str = "ZERO_UI_WR_CHANNELS";

/// Call this method before anything else in the app `main` function.
///
/// A second instance of the app executable will be started to run as the windowing and renderer process,
/// in that instance this function highjacks the process and never returns.
///
/// # Examples
///
/// ```
/// # mod zero_ui { pub mod core { pub fn init() } }
/// fn main() {
///     zero_ui::core:::init();
///
///     // .. init app normally.
/// }
/// ```
pub fn init() {
    if let Ok(names) = env::var(CHANNEL_VAR) {
        let mut names = names.splitn(2, ';');
        let request_file = names.next().expect("expected request channel");
        let response_file = names.next().expect("expected response channel");

        run(request_file, response_file);
    }
}

fn run(request_file: &str, response_file: &str) -> ! {
    let receiver = Receiver::new(
        SharedRingBuffer::create(request_file, mem::size_of::<Request>() as u32 * 2).expect("request channel connection failed"),
    );
    let sender = Sender::new(
        SharedRingBuffer::create(response_file, mem::size_of::<Response>() as u32 * 2).expect("response channel connection failed"),
    );

    let event_loop = EventLoop::<AppEvent>::with_user_event();

    let event_sender = event_loop.create_proxy();
    thread::spawn(move || {
        loop {
            match receiver.recv() {
                Ok(req) => {
                    if event_sender.send_event(AppEvent::Request(req)).is_err() {
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

    let mut app = App::new(sender);

    event_loop.run(move |event, target, control| {
        *control = ControlFlow::Wait;
        match event {
            Event::NewEvents(_) => {}
            Event::WindowEvent { window_id, event } => app.on_window_event(window_id, event),
            Event::DeviceEvent { device_id, event } => app.on_device_event(device_id, event),
            Event::UserEvent(ev) => match ev {
                AppEvent::Request(req) => app.on_request(req, target),
                AppEvent::FrameReady(window_id) => app.on_frame_ready(window_id),
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
}

#[derive(Serialize, Deserialize)]
enum Request {
    Start(StartRequest),
    OpenWindow(WindowRequest),
    CloseWindow(u32),
    Shutdown,
}

#[derive(Serialize, Deserialize)]
struct StartRequest {
    device_events: bool,
}

#[derive(Serialize, Deserialize)]
struct WindowRequest {
    title: String,
    pos: (u32, u32),
    size: (u32, u32),
}

#[derive(Serialize, Deserialize)]
enum Response {
    Started,
    WindowOpened(u32),
    WindowResized(u32, (u32, u32)),
    WindowMoved(u32, (i32, i32)),
    WindowCloseRequested(u32),
    WindowClosed(u32),
    WindowNotFound(u32),
}

struct App {
    sender: Sender,

    started: bool,
    device_events: bool,

    window_id_count: u32,
    windows: Vec<Window>,
}
impl App {
    fn new(sender: Sender) -> Self {
        Self {
            sender,
            started: false,
            device_events: false,
            window_id_count: 0,
            windows: vec![],
        }
    }

    pub fn on_request(&mut self, request: Request, target: &EventLoopWindowTarget<AppEvent>) {
        if self.started {
            match request {
                Request::Start(_) => panic!("already started"),
                Request::OpenWindow(req) => self.open_window(req, target),
                Request::CloseWindow(id) => self.close_window(id),
                Request::Shutdown => process::exit(0),
            }
        } else if let Request::Start(r) = request {
            self.started = true;
            self.device_events = r.device_events;
            self.sender.send_when_empty(&Response::Started).unwrap();
        } else {
            panic!("not started");
        }
    }

    pub fn on_window_event(&mut self, window: WindowId, event: WindowEvent) {
        if let Some((i, w)) = self.windows.iter().enumerate().find(|(_, w)|w.window_id == window) {
            let id = w.id;
            match event {
                WindowEvent::Resized(s) => self.sender.send(&Response::WindowResized(id, (s.width, s.height))).unwrap(),
                WindowEvent::Moved(p) => self.sender.send(&Response::WindowMoved(id, (p.x, p.y))).unwrap(),
                WindowEvent::CloseRequested => self.sender.send(&Response::WindowCloseRequested(id)).unwrap(),
                WindowEvent::Destroyed => {
                    self.windows.remove(i);
                    self.sender.send(&Response::WindowClosed(id)).unwrap();
                },
                WindowEvent::DroppedFile(_) => todo!(),
                WindowEvent::HoveredFile(_) => todo!(),
                WindowEvent::HoveredFileCancelled => todo!(),
                WindowEvent::ReceivedCharacter(_) => todo!(),
                WindowEvent::Focused(_) => todo!(),
                WindowEvent::KeyboardInput { device_id, input, is_synthetic } => todo!(),
                WindowEvent::ModifiersChanged(_) => todo!(),
                WindowEvent::CursorMoved { device_id, position, modifiers } => todo!(),
                WindowEvent::CursorEntered { device_id } => todo!(),
                WindowEvent::CursorLeft { device_id } => todo!(),
                WindowEvent::MouseWheel { device_id, delta, phase, modifiers } => todo!(),
                WindowEvent::MouseInput { device_id, state, button, modifiers } => todo!(),
                WindowEvent::TouchpadPressure { device_id, pressure, stage } => todo!(),
                WindowEvent::AxisMotion { device_id, axis, value } => todo!(),
                WindowEvent::Touch(_) => todo!(),
                WindowEvent::ScaleFactorChanged { scale_factor, new_inner_size } => todo!(),
                WindowEvent::ThemeChanged(_) => todo!(),
            }
        }
    }

    pub fn on_device_event(&mut self, device: DeviceId, event: DeviceEvent) {
        if !self.device_events {
            return;
        }
    }

    pub fn on_frame_ready(&mut self, window: WindowId) {}

    pub fn redraw(&mut self, window: WindowId) {
        if let Some(w) = self.windows.iter_mut().find(|w|w.window_id == window) {
            w.redraw();
        }
    }

    fn open_window(&mut self, request: WindowRequest, target: &EventLoopWindowTarget<AppEvent>) {
        self.window_id_count = self.window_id_count.wrapping_add(1);
        let id = self.window_id_count;
        self.windows.push(Window::new(id, request, target));
        self.sender.send(&Response::WindowOpened(id)).unwrap();
    }

    fn close_window(&mut self, id: u32) {
        if let Some(i) = self.windows.iter().position(|w| w.id == id) {
            let _ = self.windows.remove(i);
            self.sender.send(&Response::WindowClosed(id)).unwrap();
        } else {
            self.sender.send(&Response::WindowNotFound(id)).unwrap();
        }
    }
}

struct Window {
    id: u32,
    window_id: WindowId,
}
impl Window {
    fn new(id: u32, request: WindowRequest, target: &EventLoopWindowTarget<AppEvent>) -> Self {
        todo!()
    }    

    fn redraw(&mut self) {

    }
}
