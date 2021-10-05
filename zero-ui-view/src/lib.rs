//! View-Process implementation using [`glutin`].
//!
//! This backend supports both headed and headless apps
//!
//! # Examples
//!
//! Call [`init`] before any other code in `main` to setup a view-process that uses
//! the same app executable:
//!
//! ```no_run
//! # pub mod zero_ui { pub mod prelude {
//! # pub struct App { }
//! # impl App {
//! # pub fn default() -> Self { todo!() }
//! # pub fn run_window(self, f: impl FnOnce(bool)) { }
//! # } } }
//! use zero_ui::prelude::*;
//!
//! fn main() {
//!     zero_ui_view::init();
//!
//!     App::default().run_window(|ctx| {
//!         todo!()
//!     })
//! }
//! ```
//!
//! When the app is executed `init` setup its startup and returns, `run_window` gets called and
//! internally starts the view-process, using the `init` setup. The current executable is started
//! again, this time configured to be a view-process, `init` detects this and highjacks the process
//! **never returning**.
//!
//! [`glutin`]: https://docs.rs/glutin/

use std::{
    fmt, process,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::{Duration, Instant},
};

use glutin::{
    event::{DeviceEvent, WindowEvent},
    event_loop::{ControlFlow, EventLoop, EventLoopProxy, EventLoopWindowTarget},
    monitor::MonitorHandle,
};
use image_cache::ImageCache;
use util::{GlContextManager, WinitToPx};

/// Doc-only `webrender` re-export.
///
#[cfg(doc)]
#[doc(inline)]
pub use webrender;

mod config;
mod image_cache;
mod surface;
mod util;
mod window;
use surface::*;

use webrender::api::*;
use window::Window;
use zero_ui_view_api::{units::*, *};

/// Runs the view-process server if called in the environment of a view-process.
///
/// If this function is called in a process not configured to be a view-process it will return
/// immediately, with the expectation that the app will be started. If called in a view-process
/// if will highjack the process **never returning**.
///
/// # Examples
///
/// ```no_run
/// # pub mod zero_ui { pub mod prelude {
/// # pub struct App { }
/// # impl App {
/// # pub fn default() -> Self { todo!() }
/// # pub fn run_window(self, f: impl FnOnce(bool)) { }
/// # } } }
/// use zero_ui::prelude::*;
///
/// fn main() {
///     zero_ui_view::init();
///
///     App::default().run_window(|ctx| {
///         todo!()
///     })
/// }
/// ```
///
/// # Panics
///
/// Panics if not called in the main thread, this is a requirement of OpenGL.
///
/// If there was an error connecting with the app-process.
pub fn init() {
    if !is_main_thread::is_main_thread().unwrap_or(true) {
        panic!("only call `init` in the main thread, this is a requirement of OpenGL");
    }

    if let Some(config) = ViewConfig::from_env() {
        let c = connect_view_process(config.server_name).expect("failed to connect to app-process");

        if config.headless {
            App::run_headless(c);
        } else {
            App::run_headed(c);
        }
    }
}

/// Runs the view-process server in the current process and calls `run_app` to also
/// run the app in the current process. Note that `run_app` will be called in a different thread
/// so it must be [`Send`].
///
/// In this mode the app only uses a single process, reducing the memory footprint, but it is also not
/// resilient to video driver crashes, the view server **does not** respawn in this mode.
///
/// # Examples
///
/// The example demonstrates a setup that runs the view server in the same process in debug builds and
/// runs
///
/// ```no_run
/// # pub mod zero_ui { pub mod prelude {
/// # pub struct App { }
/// # impl App {
/// # pub fn default() -> Self { todo!() }
/// # pub fn run_window(self, f: impl FnOnce(bool)) { }
/// # } } }
/// use zero_ui::prelude::*;
///
/// fn main() {
///     if cfg!(debug_assertions) {
///         zero_ui_view::run_same_process(app_main);
///     } else {
///         zero_ui_view::init();
///         app_main();
///     }
/// }
///
/// fn app_main() {
///     App::default().run_window(|ctx| {
///         todo!()
///     })
/// }
/// ```
///
/// # Panics
///
/// Panics if not called in the main thread, this is a requirement o OpenGL.
pub fn run_same_process(run_app: impl FnOnce() + Send + 'static) -> ! {
    if !is_main_thread::is_main_thread().unwrap_or(true) {
        panic!("only call `run_same_process` in the main thread, this is a requirement of OpenGL");
    }

    thread::spawn(run_app);

    let config = ViewConfig::wait_same_process();

    let c = connect_view_process(config.server_name).expect("failed to connect to app in same process");

    if config.headless {
        App::run_headless(c);
    } else {
        App::run_headed(c);
    }
}

/// The backend implementation.
pub(crate) struct App<S> {
    started: bool,

    headless: bool,

    gl_manager: GlContextManager,
    window_target: *const EventLoopWindowTarget<AppEvent>,
    app_sender: S,
    redirect_enabled: Arc<AtomicBool>,
    redirect_recv: flume::Receiver<Request>,
    response_sender: ResponseSender,
    event_sender: EventSender,
    image_cache: ImageCache<S>,

    gen: ViewProcessGen,
    device_events: bool,

    windows: Vec<Window>,
    surfaces: Vec<Surface>,

    monitor_id_gen: MonitorId,
    pub monitors: Vec<(MonitorId, MonitorHandle)>,

    device_id_gen: DeviceId,
    devices: Vec<(DeviceId, glutin::event::DeviceId)>,

    // if one or more events where send after the last on_events_cleared.
    pending_clear: bool,

    exited: bool,
}
impl<S> fmt::Debug for App<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HeadlessBackend")
            .field("started", &self.started)
            .field("gen", &self.gen)
            .field("device_events", &self.device_events)
            .field("windows", &self.windows)
            .field("surfaces", &self.surfaces)
            .finish_non_exhaustive()
    }
}
impl App<()> {
    pub fn run_headless(c: ViewChannels) -> ! {
        let (app_sender, app_receiver) = flume::unbounded();
        let (redirect_sender, redirect_receiver) = flume::unbounded();
        let mut app = App::new(app_sender, c.response_sender, c.event_sender, redirect_receiver);
        app.headless = true;
        let event_loop = EventLoop::<AppEvent>::with_user_event();
        let window_target: &EventLoopWindowTarget<AppEvent> = &event_loop;
        app.window_target = window_target as *const _;

        app.start_receiving(c.request_receiver, redirect_sender);

        while !app.exited {
            match app_receiver.recv() {
                Ok(app_ev) => match app_ev {
                    AppEvent::Request(request) => {
                        let response = app.respond(request);
                        if app.response_sender.send(response).is_err() {
                            app.exited = true;
                            break;
                        }
                    }
                    AppEvent::FrameReady(id) => {
                        let frame_id = if let Some(s) = app.surfaces.iter_mut().find(|s| s.id() == id) {
                            s.redraw();
                            Some(s.frame_id())
                        } else {
                            None
                        };
                        if let Some(frame_id) = frame_id {
                            app.notify(Event::FrameRendered(id, frame_id));
                        }
                    }
                    AppEvent::Notify(ev) => {
                        if app.event_sender.send(ev).is_err() {
                            app.exited = true;
                            break;
                        }
                    }
                    AppEvent::RefreshMonitors => {
                        panic!("no monitor info in headless mode")
                    }
                    AppEvent::ParentProcessExited => {
                        app.exited = true;
                        break;
                    }
                    AppEvent::ImageLoaded(id, bgra8, size, dpi, opaque) => {
                        app.image_cache.loaded(id, bgra8, size, dpi, opaque);
                    }
                },
                Err(_) => {
                    app.exited = true;
                    break;
                }
            }
        }

        process::exit(0)
    }

    pub fn run_headed(c: ViewChannels) -> ! {
        let event_loop = EventLoop::with_user_event();
        let app_sender = event_loop.create_proxy();
        let (redirect_sender, redirect_receiver) = flume::unbounded();
        let mut app = App::new(app_sender, c.response_sender, c.event_sender, redirect_receiver);
        app.start_receiving(c.request_receiver, redirect_sender);

        #[cfg(windows)]
        let config_listener = config::config_listener(app.app_sender.clone(), &event_loop);

        event_loop.run(move |event, target, flow| {
            app.window_target = target;

            if app.exited {
                *flow = ControlFlow::Exit;
            } else {
                use glutin::event::Event as GEvent;
                match event {
                    GEvent::NewEvents(_) => {}
                    GEvent::WindowEvent { window_id, event } => {
                        #[cfg(windows)]
                        if window_id == config_listener.id() {
                            return; // ignore events for this window.
                        }
                        app.on_window_event(window_id, event)
                    }
                    GEvent::DeviceEvent { device_id, event } => app.on_device_event(device_id, event),
                    GEvent::UserEvent(ev) => match ev {
                        AppEvent::Request(req) => {
                            let rsp = app.respond(req);
                            if app.response_sender.send(rsp).is_err() {
                                // lost connection to app-process
                                app.exited = true;
                                *flow = ControlFlow::Exit;
                            }
                        }
                        AppEvent::Notify(ev) => {
                            if app.event_sender.send(ev).is_err() {
                                // lost connection to app-process
                                app.exited = true;
                                *flow = ControlFlow::Exit;
                            }
                        }
                        AppEvent::FrameReady(wid) => app.on_frame_ready(wid),
                        AppEvent::RefreshMonitors => app.refresh_monitors(),
                        AppEvent::ParentProcessExited => {
                            app.exited = true;
                            *flow = ControlFlow::Exit;
                        }
                        AppEvent::ImageLoaded(id, bgra8, size, dpi, opaque) => {
                            app.image_cache.loaded(id, bgra8, size, dpi, opaque);
                        }
                    },
                    GEvent::Suspended => {}
                    GEvent::Resumed => {}
                    GEvent::MainEventsCleared => app.on_events_cleared(),
                    GEvent::RedrawRequested(w_id) => app.on_redraw(w_id),
                    GEvent::RedrawEventsCleared => {}
                    GEvent::LoopDestroyed => {}
                }
            }

            app.window_target = std::ptr::null();
        })
    }
}
impl<S: AppEventSender> App<S> {
    fn new(app_sender: S, response_sender: ResponseSender, event_sender: EventSender, redirect_recv: flume::Receiver<Request>) -> Self {
        App {
            headless: false,
            started: false,
            gl_manager: GlContextManager::default(),
            image_cache: ImageCache::new(app_sender.clone()),
            app_sender,
            redirect_enabled: Arc::default(),
            redirect_recv,
            response_sender,
            event_sender,
            window_target: std::ptr::null(),
            gen: 0,
            device_events: false,
            windows: vec![],
            surfaces: vec![],
            monitors: vec![],
            monitor_id_gen: 0,
            devices: vec![],
            device_id_gen: 0,
            pending_clear: false,
            exited: false,
        }
    }

    fn start_receiving(&mut self, mut request_recv: RequestReceiver, redirect_sender: flume::Sender<Request>) {
        let app_sender = self.app_sender.clone();
        let redirect_enabled = self.redirect_enabled.clone();
        thread::spawn(move || {
            while let Ok(r) = request_recv.recv() {
                let disconnected = if redirect_enabled.load(Ordering::Relaxed) {
                    redirect_sender.send(r).is_err()
                } else {
                    app_sender.send(AppEvent::Request(r)).is_err()
                };
                if disconnected {
                    break;
                }
            }
        });
    }

    fn on_window_event(&mut self, window_id: glutin::window::WindowId, event: WindowEvent) {
        let i = if let Some((i, _)) = self.windows.iter_mut().enumerate().find(|(_, w)| w.window_id() == window_id) {
            i
        } else {
            return;
        };

        let id = self.windows[i].id();
        let scale_factor = self.windows[i].scale_factor();

        match event {
            WindowEvent::Resized(size) => {
                let size = size.to_px().to_dip(scale_factor);

                if let Some(state) = self.windows[i].state_change() {
                    self.notify(Event::WindowStateChanged(id, state, EventCause::System));
                }

                if !self.windows[i].resized(size) {
                    return;
                }
                // give the app 300ms to send a new frame, this is the collaborative way to
                // resize, it should reduce the changes of the user seeing the clear color.

                let redirect_enabled = self.redirect_enabled.clone();
                redirect_enabled.store(true, Ordering::Relaxed);
                let stop_redirect = util::RunOnDrop::new(|| redirect_enabled.store(false, Ordering::Relaxed));

                self.notify(Event::WindowResized(id, size, EventCause::System));

                let deadline = Instant::now() + Duration::from_millis(300);

                let mut received_frame = false;
                loop {
                    match self.redirect_recv.recv_deadline(deadline) {
                        Ok(req) => {
                            received_frame = req.is_frame(id);
                            if received_frame || req.is_move_or_resize(id) {
                                // received new frame
                                drop(stop_redirect);
                                self.windows[i].on_resized();
                                let rsp = self.respond(req);
                                let _ = self.response_sender.send(rsp);
                                break;
                            } else {
                                let rsp = self.respond(req);
                                let _ = self.response_sender.send(rsp);
                            }
                        }

                        Err(flume::RecvTimeoutError::Timeout) => {
                            drop(stop_redirect);
                            self.windows[i].on_resized();
                            break;
                        }
                        Err(flume::RecvTimeoutError::Disconnected) => {
                            unreachable!()
                        }
                    }
                }

                let drained: Vec<_> = self.redirect_recv.drain().collect();
                for req in drained {
                    let _ = self.app_sender.send(AppEvent::Request(req));
                }

                // if we are still within 1 second, wait webrender, and if a frame was rendered here, notify.
                if received_frame && deadline > Instant::now() && self.windows[i].wait_frame_ready(deadline) {
                    let id = self.windows[i].id();
                    let frame_id = self.windows[i].frame_id();
                    self.notify(Event::FrameRendered(id, frame_id));
                }
            }
            WindowEvent::Moved(p) => {
                let p = p.to_px().to_dip(scale_factor);

                if !self.windows[i].moved(p) {
                    return;
                }

                self.notify(Event::WindowMoved(id, p, EventCause::System));
            }
            WindowEvent::CloseRequested => self.notify(Event::WindowCloseRequested(id)),
            WindowEvent::Destroyed => {
                self.windows.remove(i);
                self.notify(Event::WindowClosed(id));
            }
            WindowEvent::DroppedFile(file) => self.notify(Event::DroppedFile(id, file)),
            WindowEvent::HoveredFile(file) => self.notify(Event::HoveredFile(id, file)),
            WindowEvent::HoveredFileCancelled => self.notify(Event::HoveredFileCancelled(id)),
            WindowEvent::ReceivedCharacter(c) => self.notify(Event::ReceivedCharacter(id, c)),
            WindowEvent::Focused(focused) => self.notify(Event::Focused(id, focused)),
            WindowEvent::KeyboardInput { device_id, input, .. } => {
                let d_id = self.device_id(device_id);
                self.notify(Event::KeyboardInput(
                    id,
                    d_id,
                    input.scancode,
                    util::element_state_to_key_state(input.state),
                    input.virtual_keycode.map(util::v_key_to_key),
                ));
            }
            WindowEvent::ModifiersChanged(m) => {
                self.refresh_monitors();
                self.notify(Event::ModifiersChanged(id, util::winit_modifiers_state_to_zui(m)));
            }
            WindowEvent::CursorMoved { device_id, position, .. } => {
                let px_p = position.to_px();
                let p = px_p.to_dip(scale_factor);
                let d_id = self.device_id(device_id);
                let (f_id, ht) = self.windows[i].hit_test(px_p);
                self.notify(Event::CursorMoved(id, d_id, p, ht, f_id));
            }
            WindowEvent::CursorEntered { device_id } => {
                let d_id = self.device_id(device_id);
                self.notify(Event::CursorEntered(id, d_id));
            }
            WindowEvent::CursorLeft { device_id } => {
                let d_id = self.device_id(device_id);
                self.notify(Event::CursorLeft(id, d_id));
            }
            WindowEvent::MouseWheel {
                device_id, delta, phase, ..
            } => {
                let d_id = self.device_id(device_id);
                self.notify(Event::MouseWheel(
                    id,
                    d_id,
                    util::winit_mouse_wheel_delta_to_zui(delta),
                    util::winit_touch_phase_to_zui(phase),
                ));
            }
            WindowEvent::MouseInput {
                device_id, state, button, ..
            } => {
                let d_id = self.device_id(device_id);
                self.notify(Event::MouseInput(
                    id,
                    d_id,
                    util::element_state_to_button_state(state),
                    util::winit_mouse_button_to_zui(button),
                ));
            }
            WindowEvent::TouchpadPressure {
                device_id,
                pressure,
                stage,
            } => {
                let d_id = self.device_id(device_id);
                self.notify(Event::TouchpadPressure(id, d_id, pressure, stage));
            }
            WindowEvent::AxisMotion { device_id, axis, value } => {
                let d_id = self.device_id(device_id);
                self.notify(Event::AxisMotion(id, d_id, axis, value));
            }
            WindowEvent::Touch(t) => {
                let d_id = self.device_id(t.device_id);
                let location = t.location.to_px().to_dip(scale_factor);
                self.notify(Event::Touch(
                    id,
                    d_id,
                    util::winit_touch_phase_to_zui(t.phase),
                    location,
                    t.force.map(util::winit_force_to_zui),
                    t.id,
                ));
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => self.notify(Event::ScaleFactorChanged(id, scale_factor as f32)),
            WindowEvent::ThemeChanged(t) => self.notify(Event::WindowThemeChanged(id, util::winit_theme_to_zui(t))),
        }
    }

    fn refresh_monitors(&mut self) {
        let mut monitors = Vec::with_capacity(self.monitors.len());

        let mut added_check = false; // set to `true` if a new id is generated.
        let mut removed_check = self.monitors.len(); // `-=1` every existing reused `id`.

        let window_target = unsafe { &*self.window_target };

        for handle in window_target.available_monitors() {
            let id = self
                .monitors
                .iter()
                .find_map(|(id, h)| {
                    if h == &handle {
                        removed_check = removed_check.checked_sub(1).unwrap();
                        Some(*id)
                    } else {
                        added_check = true;
                        None
                    }
                })
                .unwrap_or_else(|| {
                    let mut id = self.monitor_id_gen.wrapping_add(1);
                    if id == 0 {
                        id += 1;
                    }
                    self.monitor_id_gen = id;
                    id
                });
            monitors.push((id, handle))
        }

        if added_check || removed_check > 1 {
            self.monitors = monitors;

            let monitors = self.available_monitors();
            self.notify(Event::MonitorsChanged(monitors));
        }
    }

    fn on_frame_ready(&mut self, window_id: WindowId) {
        if let Some(w) = self.windows.iter_mut().find(|w| w.id() == window_id) {
            let id = w.id();
            let frame_id = w.frame_id();
            let first_frame = w.request_redraw();

            if first_frame {
                let pos = w.outer_position();
                let size = w.size();
                let scale_factor = w.scale_factor();

                self.notify(Event::WindowMoved(id, pos, EventCause::App));
                self.notify(Event::WindowResized(id, size, EventCause::App));
                self.notify(Event::ScaleFactorChanged(id, scale_factor));
            }

            self.notify(Event::FrameRendered(id, frame_id));
        }
    }

    pub(crate) fn notify(&mut self, event: Event) {
        self.pending_clear = true;
        if self.event_sender.send(event).is_err() {
            let _ = self.app_sender.send(AppEvent::ParentProcessExited);
        }
    }

    fn on_device_event(&mut self, device_id: glutin::event::DeviceId, event: DeviceEvent) {
        if self.device_events {
            let d_id = self.device_id(device_id);
            match event {
                DeviceEvent::Added => self.notify(Event::DeviceAdded(d_id)),
                DeviceEvent::Removed => self.notify(Event::DeviceRemoved(d_id)),
                DeviceEvent::MouseMotion { delta } => self.notify(Event::DeviceMouseMotion(d_id, delta)),
                DeviceEvent::MouseWheel { delta } => {
                    self.notify(Event::DeviceMouseWheel(d_id, util::winit_mouse_wheel_delta_to_zui(delta)))
                }
                DeviceEvent::Motion { axis, value } => self.notify(Event::DeviceMotion(d_id, axis, value)),
                DeviceEvent::Button { button, state } => {
                    self.notify(Event::DeviceButton(d_id, button, util::element_state_to_button_state(state)))
                }
                DeviceEvent::Key(k) => self.notify(Event::DeviceKey(
                    d_id,
                    k.scancode,
                    util::element_state_to_key_state(k.state),
                    k.virtual_keycode.map(util::v_key_to_key),
                )),
                DeviceEvent::Text { codepoint } => self.notify(Event::DeviceText(d_id, codepoint)),
            }
        }
    }

    fn on_events_cleared(&mut self) {
        if self.pending_clear {
            self.notify(Event::EventsCleared);
            self.pending_clear = false;
        }
    }

    fn on_redraw(&mut self, window_id: glutin::window::WindowId) {
        if let Some(w) = self.windows.iter_mut().find(|w| w.window_id() == window_id) {
            w.redraw();
        }
    }

    fn assert_started(&self) {
        if !self.started {
            panic!("not started")
        }
    }

    fn with_window<R>(&mut self, id: WindowId, action: impl FnOnce(&mut Window) -> R, not_found: impl FnOnce() -> R) -> R {
        self.assert_started();
        self.windows.iter_mut().find(|w| w.id() == id).map(action).unwrap_or_else(|| {
            log::error!("headed window `{}` not found, will return fallback result", id);
            not_found()
        })
    }

    fn with_surface<R>(&mut self, id: WindowId, action: impl FnOnce(&mut Surface) -> R, not_found: impl FnOnce() -> R) -> R {
        self.assert_started();
        self.surfaces.iter_mut().find(|w| w.id() == id).map(action).unwrap_or_else(|| {
            log::error!("headless window `{}` not found, will return fallback result", id);
            not_found()
        })
    }

    fn monitor_id(&mut self, handle: &MonitorHandle) -> MonitorId {
        if let Some((id, _)) = self.monitors.iter().find(|(_, h)| h == handle) {
            *id
        } else {
            let mut id = self.monitor_id_gen.wrapping_add(1);
            if id == 0 {
                id = 1;
            }
            self.monitor_id_gen = id;
            self.monitors.push((id, handle.clone()));
            id
        }
    }

    fn device_id(&mut self, device_id: glutin::event::DeviceId) -> DeviceId {
        if let Some((id, _)) = self.devices.iter().find(|(_, id)| *id == device_id) {
            *id
        } else {
            let mut id = self.device_id_gen.wrapping_add(1);
            if id == 0 {
                id = 1;
            }
            self.device_id_gen = id;
            self.devices.push((id, device_id));
            id
        }
    }
}
macro_rules! with_window_or_surface {
    ($self:ident, $id:ident, |$el:ident|$action:expr, ||$fallback:expr) => {
        if let Some($el) = $self.windows.iter_mut().find(|w| w.id() == $id) {
            $action
        } else if let Some($el) = $self.surfaces.iter_mut().find(|w| w.id() == $id) {
            $action
        } else {
            log::error!("window `{}` not found, will return fallback result", $id);
            $fallback
        }
    };
}
impl<S: AppEventSender> Api for App<S> {
    fn api_version(&mut self) -> String {
        VERSION.to_owned()
    }

    fn startup(&mut self, gen: ViewProcessGen, device_events: bool, headless: bool) {
        if self.started {
            panic!("already started");
        }
        if self.exited {
            panic!("cannot restart exited")
        }
        self.started = true;
        self.gen = gen;
        self.device_events = device_events;
        self.headless = headless;
    }

    fn exit(&mut self) {
        self.assert_started();
        self.started = false;
        self.exited = true;
    }

    fn primary_monitor(&mut self) -> Option<(MonitorId, MonitorInfo)> {
        self.assert_started();

        let window_target = unsafe { &*self.window_target };

        window_target
            .primary_monitor()
            .or_else(|| window_target.available_monitors().next())
            .map(|m| {
                let id = self.monitor_id(&m);
                let mut info = util::monitor_handle_to_info(&m);
                info.is_primary = true;
                (id, info)
            })
    }

    fn monitor_info(&mut self, id: MonitorId) -> Option<MonitorInfo> {
        self.assert_started();

        let window_target = unsafe { &*self.window_target };

        self.monitors.iter().find(|(i, _)| *i == id).map(|(_, h)| {
            let mut info = util::monitor_handle_to_info(h);
            info.is_primary = window_target.primary_monitor().map(|p| &p == h).unwrap_or(false);
            info
        })
    }

    fn available_monitors(&mut self) -> Vec<(MonitorId, MonitorInfo)> {
        self.assert_started();

        let window_target = unsafe { &*self.window_target };

        let primary = window_target.primary_monitor();

        window_target
            .available_monitors()
            .map(|m| {
                let id = self.monitor_id(&m);
                let is_primary = primary.as_ref().map(|h| h == &m).unwrap_or(false);
                let mut info = util::monitor_handle_to_info(&m);
                info.is_primary = is_primary;
                (id, info)
            })
            .collect()
    }

    fn open_window(&mut self, config: WindowConfig) -> (webrender_api::IdNamespace, webrender_api::PipelineId) {
        if self.headless {
            self.open_headless(HeadlessConfig {
                id: config.id,
                scale_factor: 1.0,
                size: config.size,
                text_aa: config.text_aa,
            })
        } else {
            self.assert_started();
            let win = Window::open(
                self.gen,
                config.icon.and_then(|i| self.image_cache.get(i)).and_then(|i| i.icon()),
                config,
                unsafe { &*self.window_target },
                &mut self.gl_manager,
                self.app_sender.clone(),
            );

            let namespace = win.namespace_id();
            let pipeline = win.pipeline_id();

            self.windows.push(win);

            (namespace, pipeline)
        }
    }

    fn open_headless(&mut self, config: HeadlessConfig) -> (webrender_api::IdNamespace, webrender_api::PipelineId) {
        self.assert_started();
        let surf = Surface::open(
            self.gen,
            config,
            unsafe { &*self.window_target },
            &mut self.gl_manager,
            self.app_sender.clone(),
        );
        let namespace = surf.namespace_id();
        let pipeline = surf.pipeline_id();

        self.surfaces.push(surf);

        (namespace, pipeline)
    }

    fn close_window(&mut self, id: WindowId) {
        self.assert_started();
        if let Some(i) = self.windows.iter().position(|w| w.id() == id) {
            let _ = self.windows.swap_remove(i);
        }
        if let Some(i) = self.surfaces.iter().position(|w| w.id() == id) {
            let _ = self.surfaces.swap_remove(i);
        }
    }

    fn text_aa(&mut self) -> TextAntiAliasing {
        self.assert_started();
        config::text_aa()
    }

    fn multi_click_config(&mut self) -> MultiClickConfig {
        self.assert_started();
        config::multi_click_config()
    }

    fn animation_enabled(&mut self) -> bool {
        self.assert_started();
        config::animation_enabled()
    }

    fn key_repeat_delay(&mut self) -> Duration {
        self.assert_started();
        config::key_repeat_delay()
    }

    fn set_title(&mut self, id: WindowId, title: String) {
        self.with_window(id, |w| w.set_title(title), || ())
    }

    fn set_visible(&mut self, id: WindowId, visible: bool) {
        self.with_window(id, |w| w.set_visible(visible), || ())
    }

    fn set_always_on_top(&mut self, id: WindowId, always_on_top: bool) {
        self.with_window(id, |w| w.set_always_on_top(always_on_top), || ())
    }

    fn set_movable(&mut self, id: WindowId, movable: bool) {
        self.with_window(id, |w| w.set_movable(movable), || ())
    }

    fn set_resizable(&mut self, id: WindowId, resizable: bool) {
        self.with_window(id, |w| w.set_resizable(resizable), || ())
    }

    fn set_taskbar_visible(&mut self, id: WindowId, visible: bool) {
        self.with_window(id, |w| w.set_taskbar_visible(visible), || ())
    }

    fn set_parent(&mut self, id: WindowId, parent: Option<WindowId>, modal: bool) {
        let parent = parent.and_then(|id| self.windows.iter().find(|w| w.id() == id).map(|w| w.window_id()));
        self.with_window(id, |w| w.set_parent(parent, modal), || ())
    }

    fn set_transparent(&mut self, id: WindowId, transparent: bool) {
        with_window_or_surface!(self, id, |w| w.set_transparent(transparent), || ())
    }

    fn set_chrome_visible(&mut self, id: WindowId, visible: bool) {
        self.with_window(id, |w| w.set_chrome_visible(visible), || ())
    }

    fn set_position(&mut self, id: WindowId, pos: DipPoint) {
        if self.with_window(id, |w| w.set_outer_pos(pos), || false) {
            let _ = self.app_sender.send(AppEvent::Notify(Event::WindowMoved(id, pos, EventCause::App)));
        }
    }

    fn set_size(&mut self, id: WindowId, size: DipSize, frame: FrameRequest) {
        self.with_surface(
            id,
            |w| {
                w.set_size(size, w.scale_factor());
                w.render(frame);
            },
            || (),
        );
    }

    fn set_state(&mut self, id: WindowId, state: WindowState) {
        if self.with_window(id, |w| w.set_state(state), || false) {
            let _ = self
                .app_sender
                .send(AppEvent::Notify(Event::WindowStateChanged(id, state, EventCause::App)));
        }
    }

    fn set_headless_size(&mut self, id: WindowId, size: DipSize, scale_factor: f32) {
        self.with_surface(
            id,
            |w| {
                w.set_size(size, scale_factor);
            },
            || (),
        )
    }

    fn set_video_mode(&mut self, id: WindowId, mode: VideoMode) {
        self.with_window(id, |w| w.set_video_mode(mode), || ())
    }

    fn set_min_size(&mut self, id: WindowId, size: DipSize) {
        self.with_window(id, |w| w.set_min_inner_size(size), || ())
    }

    fn set_max_size(&mut self, id: WindowId, size: DipSize) {
        self.with_window(id, |w| w.set_max_inner_size(size), || ())
    }

    fn set_icon(&mut self, id: WindowId, icon: Option<ImageId>) {
        let icon = icon.and_then(|i| self.image_cache.get(i)).and_then(|i| i.icon());
        self.with_window(id, |w| w.set_icon(icon), || ())
    }

    fn pipeline_id(&mut self, id: WindowId) -> PipelineId {
        with_window_or_surface!(self, id, |w| w.pipeline_id(), || PipelineId::dummy())
    }

    fn namespace_id(&mut self, id: WindowId) -> IdNamespace {
        with_window_or_surface!(self, id, |w| w.namespace_id(), || IdNamespace(0))
    }

    fn add_image(&mut self, format: ImageDataFormat, data: IpcSharedMemory) -> ImageId {
        self.image_cache.add(data, format)
    }

    fn forget_image(&mut self, id: ImageId) {
        self.image_cache.forget(id)
    }

    fn use_image(&mut self, id: WindowId, image_id: ImageId) -> ImageKey {
        if let Some(img) = self.image_cache.get(image_id) {
            with_window_or_surface!(self, id, |w| w.use_image(img), || ImageKey::DUMMY)
        } else {
            ImageKey::DUMMY
        }
    }

    fn update_image(&mut self, id: WindowId, key: ImageKey, image_id: ImageId) {
        if let Some(img) = self.image_cache.get(image_id) {
            with_window_or_surface!(self, id, |w| w.update_image(key, img), || ())
        }
    }

    fn delete_image(&mut self, id: WindowId, key: ImageKey) {
        with_window_or_surface!(self, id, |w| w.delete_image(key), || ())
    }

    fn read_img_pixels(&mut self, id: ImageId, response: IpcSender<ImagePixels>) -> bool {
        self.image_cache
            .get(id)
            .map(|img| {
                img.read_pixels(response);
                true
            })
            .unwrap_or(false)
    }

    fn read_img_pixels_rect(&mut self, id: ImageId, rect: PxRect, response: IpcSender<ImagePixels>) -> bool {
        self.image_cache
            .get(id)
            .map(|img| {
                img.read_pixels_rect(rect, response);
                true
            })
            .unwrap_or(false)
    }

    fn add_font(&mut self, id: WindowId, bytes: ByteBuf, index: u32) -> FontKey {
        with_window_or_surface!(self, id, |w| w.add_font(bytes.to_vec(), index), || FontKey(IdNamespace(0), 0))
    }

    fn delete_font(&mut self, id: WindowId, key: FontKey) {
        with_window_or_surface!(self, id, |w| w.delete_font(key), || ())
    }

    fn add_font_instance(
        &mut self,
        id: WindowId,
        font_key: FontKey,
        glyph_size: Px,
        options: Option<FontInstanceOptions>,
        plataform_options: Option<FontInstancePlatformOptions>,
        variations: Vec<FontVariation>,
    ) -> FontInstanceKey {
        with_window_or_surface!(
            self,
            id,
            |w| w.add_font_instance(font_key, glyph_size, options, plataform_options, variations),
            || FontInstanceKey(IdNamespace(0), 0)
        )
    }

    fn delete_font_instance(&mut self, id: WindowId, instance_key: FontInstanceKey) {
        with_window_or_surface!(self, id, |w| w.delete_font_instance(instance_key), || ())
    }

    fn size(&mut self, id: WindowId) -> DipSize {
        with_window_or_surface!(self, id, |w| w.size(), || DipSize::zero())
    }

    fn scale_factor(&mut self, id: WindowId) -> f32 {
        with_window_or_surface!(self, id, |w| w.scale_factor(), || 1.0)
    }

    fn set_allow_alt_f4(&mut self, id: WindowId, allow: bool) {
        self.with_window(id, |w| w.set_allow_alt_f4(allow), || ())
    }

    fn read_pixels(&mut self, id: WindowId, response: IpcSender<FramePixels>) -> bool {
        with_window_or_surface!(
            self,
            id,
            |w| {
                w.read_pixels(response);
                true
            },
            || false
        )
    }

    fn read_pixels_rect(&mut self, id: WindowId, rect: PxRect, response: IpcSender<FramePixels>) -> bool {
        with_window_or_surface!(
            self,
            id,
            |w| {
                w.read_pixels_rect(rect, response);
                true
            },
            || false
        )
    }

    fn hit_test(&mut self, id: WindowId, point: PxPoint) -> (Epoch, HitTestResult) {
        with_window_or_surface!(self, id, |w| w.hit_test(point), || (Epoch(0), HitTestResult::default()))
    }

    fn set_text_aa(&mut self, id: WindowId, aa: TextAntiAliasing) {
        with_window_or_surface!(self, id, |w| w.set_text_aa(aa), || ())
    }

    fn render(&mut self, id: WindowId, frame: FrameRequest) {
        with_window_or_surface!(self, id, |w| w.render(frame), || ())
    }

    fn render_update(&mut self, id: WindowId, updates: DynamicProperties, clear_color: Option<ColorF>) {
        with_window_or_surface!(self, id, |w| w.render_update(updates, clear_color), || ())
    }

    #[cfg(debug_assertions)]
    fn crash(&mut self) {
        panic!("CRASH")
    }
}

/// Message inserted in the event loop from the view-process.
pub(crate) enum AppEvent {
    /// A request.
    Request(Request),
    /// Notify an event.
    Notify(Event),
    /// A frame is ready for redraw.
    FrameReady(WindowId),
    /// Re-query available monitors and send update event.
    RefreshMonitors,
    /// Lost connection with app-process.
    ParentProcessExited,

    /// Image finished decoding, must call [`ImageCache::loaded`].
    ImageLoaded(ImageId, IpcSharedMemory, PxSize, ImagePpi, bool),
}

/// Abstraction over channel senders  that can inject [`AppEvent`] in the app loop.
pub(crate) trait AppEventSender: Clone + Send + 'static {
    /// Send an event.
    fn send(&self, ev: AppEvent) -> Result<(), Disconnected>;
}
/// headless
impl AppEventSender for flume::Sender<AppEvent> {
    fn send(&self, ev: AppEvent) -> Result<(), Disconnected> {
        self.send(ev).map_err(|_| Disconnected)
    }
}
/// headed
impl AppEventSender for EventLoopProxy<AppEvent> {
    fn send(&self, ev: AppEvent) -> Result<(), Disconnected> {
        self.send_event(ev).map_err(|_| Disconnected)
    }
}
