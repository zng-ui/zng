use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use std::{env, thread};

use crate::units::*;
use glutin::event::{DeviceEvent, DeviceId, Event, WindowEvent};
use glutin::event_loop::{ControlFlow, EventLoop, EventLoopProxy, EventLoopWindowTarget};
use glutin::monitor::MonitorHandle;
use glutin::window::WindowId;
use parking_lot::Condvar;

use crate::headless::ViewHeadless;
use crate::window::ViewWindow;
use crate::{config, ipc, types::*, util, Request, Response, SameProcessConfig, MODE_VAR, SAME_PROCESS_CONFIG, SERVER_NAME_VAR};

#[cfg_attr(doc_nightly, doc(cfg(feature = "full")))]
/// Call this function before anything else in the app `main` function.
///
/// If the process is started with the right environment configuration this function
/// high-jacks the process and turns it into a *View Process*, never returning.
///
/// This function does nothing if the *View Process* environment is not set, you can safely call it more then once.
///
/// # Examples
///
/// ```no_run
/// # use zero_ui_vp::init_view_process;
/// fn main() {
///     init_view_process();
///
///     println!("Only Prints if is not View Process");
///
///     // .. init app here.
/// }
/// ```
pub fn init_view_process() {
    if let Ok(server_name) = env::var(SERVER_NAME_VAR) {
        let mode = env::var(MODE_VAR).unwrap_or_else(|_| "headed".to_owned());
        let headless = match mode.as_str() {
            "headed" => false,
            "headless" => true,
            _ => panic!("unknown mode"),
        };
        run(server_name, headless, None);
    }
}

#[cfg_attr(doc_nightly, doc(cfg(feature = "full")))]
/// Run both View and App in the same process.
///
/// This function must be called in the main thread, it initializes the View and calls `run_app`
/// in a new thread to initialize the App.
///
/// The primary use of this function is debugging the view process code
pub fn run_same_process(run_app: impl FnOnce() + Send + 'static) -> ! {
    if !is_main_thread::is_main_thread().unwrap_or(true) {
        panic!("can only init view in the main thread")
    }

    let mut config = SAME_PROCESS_CONFIG.lock();

    let app_thread = thread::spawn(run_app);

    let waiter = Arc::new(Condvar::new());
    *config = Some(SameProcessConfig {
        waiter: waiter.clone(),
        server_name: String::new(),
        headless: false,
    });

    if cfg!(debug_assertions) {
        waiter.wait(&mut config);
    } else {
        let r = waiter.wait_for(&mut config, Duration::from_secs(10)).timed_out();
        if r {
            panic!("Controller::start was not called in 10 seconds");
        }
    };

    let config = config.take().unwrap();
    run(config.server_name, config.headless, Some(app_thread))
}

/// The View Process.
#[cfg(feature = "full")]
pub(crate) struct ViewApp<E> {
    pub event_loop: E,
    response_chan: ipc::ResponseSender,
    event_chan: ipc::EvSender,

    pub generation: ViewProcessGen,

    redirect_enabled: Arc<AtomicBool>,
    redirect_chan: flume::Receiver<Request>,

    pub started: bool,
    pub device_events: bool,
    pub headless: bool,

    pub window_id_count: WinId,
    pub windows: Vec<ViewWindow>,
    pub headless_views: Vec<ViewHeadless>,

    monitor_id_count: MonId,
    pub monitors: Vec<(MonId, MonitorHandle)>,

    device_id_count: DevId,
    devices: Vec<(DevId, DeviceId)>,

    // if one or more events where send after the last on_events_cleared.
    pending_clear: bool,
}
#[cfg(feature = "full")]
impl<E: AppEventSender> ViewApp<E> {
    pub fn new(
        event_loop: E,
        response_chan: ipc::ResponseSender,
        event_chan: ipc::EvSender,
        redirect_enabled: Arc<AtomicBool>,
        redirect_chan: flume::Receiver<Request>,
        headless: bool,
    ) -> Self {
        Self {
            event_loop,
            response_chan,
            event_chan,
            redirect_enabled,
            redirect_chan,
            generation: 0,
            started: false,
            device_events: false,
            headless,
            window_id_count: u32::from_ne_bytes(*b"zwvp"),
            windows: vec![],
            headless_views: vec![],
            monitor_id_count: u32::from_ne_bytes(*b"zsvp"),
            monitors: vec![],
            device_id_count: u32::from_ne_bytes(*b"zdvp"),
            devices: vec![],
            pending_clear: false,
        }
    }

    pub(crate) fn respond(&mut self, response: Response) {
        if self.response_chan.send(response).is_err() {
            let _ = self.event_loop.send(AppEvent::ParentProcessExited);
        }
    }
    pub(crate) fn notify(&mut self, event: Ev) {
        self.pending_clear = true;
        if self.event_chan.send(event).is_err() {
            let _ = self.event_loop.send(AppEvent::ParentProcessExited);
        }
    }

    pub(crate) fn monitor_id(&mut self, handle: &MonitorHandle) -> MonId {
        if let Some((id, _)) = self.monitors.iter().find(|(_, h)| h == handle) {
            *id
        } else {
            let mut id = self.monitor_id_count.wrapping_add(1);
            if id == 0 {
                id = 1;
            }
            self.monitor_id_count = id;
            self.monitors.push((id, handle.clone()));
            id
        }
    }

    fn device_id(&mut self, device_id: DeviceId) -> DevId {
        if let Some((id, _)) = self.devices.iter().find(|(_, id)| *id == device_id) {
            *id
        } else {
            let mut id = self.device_id_count.wrapping_add(1);
            if id == 0 {
                id = 1;
            }
            self.device_id_count = id;
            self.devices.push((id, device_id));
            id
        }
    }

    pub(crate) fn with_window<R>(&mut self, id: WinId, d: impl FnOnce() -> R, f: impl FnOnce(&mut ViewWindow) -> R) -> R {
        assert!(self.started);
        if let Some(w) = self.windows.iter_mut().find(|w| w.id() == id) {
            f(w)
        } else {
            d()
        }
    }

    pub(crate) fn with_headless<R>(&mut self, id: WinId, d: impl FnOnce() -> R, f: impl FnOnce(&mut ViewHeadless) -> R) -> R {
        assert!(self.started);
        if let Some(w) = self.headless_views.iter_mut().find(|w| w.id() == id) {
            f(w)
        } else {
            d()
        }
    }

    fn on_window_event(&mut self, ctx: &Context<E>, window_id: WindowId, event: WindowEvent) {
        let i = if let Some((i, _)) = self.windows.iter_mut().enumerate().find(|(_, w)| w.is_window(window_id)) {
            i
        } else {
            return;
        };

        let id = self.windows[i].id();
        let scale_factor = self.windows[i].scale_factor();

        match event {
            WindowEvent::Resized(size) => {
                let size = size.to_px().to_dip(scale_factor);

                if !self.windows[i].resized(size) {
                    return;
                }
                // give the app 300ms to send a new frame, this is the collaborative way to
                // resize, it should reduce the changes of the user seeing the clear color.

                let redirect_enabled = self.redirect_enabled.clone();
                redirect_enabled.store(true, Ordering::Relaxed);
                let stop_redirect = util::RunOnDrop::new(|| redirect_enabled.store(false, Ordering::Relaxed));

                self.notify(Ev::WindowResized(id, size, EventCause::System));

                let deadline = Instant::now() + Duration::from_millis(300);

                let mut received_frame = false;
                loop {
                    match self.redirect_chan.recv_deadline(deadline) {
                        Ok(req) => {
                            match &req {
                                // received new frame
                                Request::render { id: r_id, .. } | Request::render_update { id: r_id, .. } if *r_id == id => {
                                    drop(stop_redirect);
                                    received_frame = true;
                                    self.windows[i].on_resized();
                                    self.on_request(ctx, req);
                                    break;
                                }
                                // interrupt redirect
                                Request::set_position { id: r_id, .. }
                                | Request::set_size { id: r_id, .. }
                                | Request::set_min_size { id: r_id, .. }
                                | Request::set_max_size { id: r_id, .. }
                                    if *r_id == id =>
                                {
                                    drop(stop_redirect);
                                    self.windows[i].on_resized();
                                    self.on_request(ctx, req);
                                    break;
                                }
                                // proxy
                                _ => self.on_request(ctx, req),
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

                let drained: Vec<_> = self.redirect_chan.drain().collect();
                for req in drained {
                    self.on_request(ctx, req);
                }

                // if we are still within 1 second, wait webrender, and if a frame was rendered here, notify.
                if received_frame && deadline > Instant::now() && self.windows[i].wait_frame_ready(deadline) {
                    let id = self.windows[i].id();
                    let frame_id = self.windows[i].frame_id();
                    self.notify(Ev::FrameRendered(id, frame_id));
                }
            }
            WindowEvent::Moved(p) => {
                let p = p.to_px().to_dip(scale_factor);

                if !self.windows[i].moved(p) {
                    return;
                }

                self.notify(Ev::WindowMoved(id, p, EventCause::System));
            }
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
                self.notify(Ev::KeyboardInput(
                    id,
                    d_id,
                    input.scancode,
                    input.state.into(),
                    input.virtual_keycode.map(Into::into),
                ));
            }
            WindowEvent::ModifiersChanged(m) => {
                self.refresh_monitors(ctx);
                self.notify(Ev::ModifiersChanged(id, m.into()));
            }
            WindowEvent::CursorMoved { device_id, position, .. } => {
                let px_p = position.to_px();
                let p = px_p.to_dip(scale_factor);
                let d_id = self.device_id(device_id);
                let (f_id, ht) = self.windows[i].hit_test(px_p);
                self.notify(Ev::CursorMoved(id, d_id, p, ht, f_id));
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
                self.notify(Ev::MouseWheel(id, d_id, delta.into(), phase.into()));
            }
            WindowEvent::MouseInput {
                device_id, state, button, ..
            } => {
                let d_id = self.device_id(device_id);
                self.notify(Ev::MouseInput(id, d_id, state.into(), button.into()));
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
                let location = t.location.to_px().to_dip(scale_factor);
                self.notify(Ev::Touch(id, d_id, t.phase.into(), location, t.force.map(Into::into), t.id));
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => self.notify(Ev::ScaleFactorChanged(id, scale_factor as f32)),
            WindowEvent::ThemeChanged(t) => self.notify(Ev::WindowThemeChanged(id, t.into())),
        }
    }

    fn on_device_event(&mut self, device_id: DeviceId, event: DeviceEvent) {
        if self.device_events {
            let d_id = self.device_id(device_id);
            match event {
                DeviceEvent::Added => self.notify(Ev::DeviceAdded(d_id)),
                DeviceEvent::Removed => self.notify(Ev::DeviceRemoved(d_id)),
                DeviceEvent::MouseMotion { delta } => self.notify(Ev::DeviceMouseMotion(d_id, delta)),
                DeviceEvent::MouseWheel { delta } => self.notify(Ev::DeviceMouseWheel(d_id, delta.into())),
                DeviceEvent::Motion { axis, value } => self.notify(Ev::DeviceMotion(d_id, axis, value)),
                DeviceEvent::Button { button, state } => self.notify(Ev::DeviceButton(d_id, button, state.into())),
                DeviceEvent::Key(k) => self.notify(Ev::DeviceKey(d_id, k.scancode, k.state.into(), k.virtual_keycode.map(Into::into))),
                DeviceEvent::Text { codepoint } => self.notify(Ev::DeviceText(d_id, codepoint)),
            }
        }
    }

    fn refresh_monitors(&mut self, ctx: &Context<E>) {
        let mut monitors = Vec::with_capacity(self.monitors.len());

        let mut added_check = false; // set to `true` if a new id is generated.
        let mut removed_check = self.monitors.len(); // `-=1` every existing reused `id`.

        for handle in ctx.window_target.available_monitors() {
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
                    let mut id = self.monitor_id_count.wrapping_add(1);
                    if id == 0 {
                        id += 1;
                    }
                    self.monitor_id_count = id;
                    id
                });
            monitors.push((id, handle))
        }

        if added_check || removed_check > 1 {
            self.monitors = monitors;

            let monitors = self.available_monitors(ctx);
            self.notify(Ev::MonitorsChanged(monitors));
        }
    }

    fn on_frame_ready(&mut self, window_id: WindowId) {
        if let Some(w) = self.windows.iter_mut().find(|w| w.is_window(window_id)) {
            let id = w.id();
            let frame_id = w.frame_id();
            let first_frame = w.request_redraw();

            if first_frame {
                let pos = w.outer_position();
                let size = w.size();

                self.notify(Ev::WindowMoved(id, pos, EventCause::App));
                self.notify(Ev::WindowResized(id, size, EventCause::App));
            }

            self.notify(Ev::FrameRendered(id, frame_id));
        }
    }

    fn on_headless_frame_ready(&mut self, id: WinId) {
        if let Some(v) = self.headless_views.iter_mut().find(|w| w.id() == id) {
            v.redraw();
            let frame_id = v.frame_id();
            self.notify(Ev::FrameRendered(id, frame_id));
        }
    }

    fn on_redraw(&mut self, window_id: WindowId) {
        if let Some(w) = self.windows.iter_mut().find(|w| w.is_window(window_id)) {
            w.redraw();
        }
    }

    fn on_events_cleared(&mut self) {
        if self.pending_clear {
            self.notify(Ev::EventsCleared);
            self.pending_clear = false;
        }
    }
}

/// Start the event loop in the View Process.
fn run(server_name: String, headless: bool, mut same_process_app: Option<JoinHandle<()>>) -> ! {
    if !is_main_thread::is_main_thread().unwrap_or(true) {
        panic!("can only init view-process in the main thread")
    }

    let (mut request_receiver, response_sender, event_sender) = ipc::connect_view_process(server_name);

    let event_loop = EventLoop::<AppEvent>::with_user_event();

    // unless redirecting, for operations like the blocking Resize.
    let redirect_enabled = Arc::new(AtomicBool::new(false));

    let (redirect_sender, redirect_receiver) = flume::unbounded();

    let (headless_app_ev_sender, headless_app_ev_receiver) = flume::unbounded();

    if headless {
        let redirect_enabled = redirect_enabled.clone();
        let headless_app_ev_sender = headless_app_ev_sender.clone();
        let _ = redirect_sender;
        thread::spawn(move || {
            loop {
                match request_receiver.recv() {
                    Ok(req) => {
                        if cfg!(debug_assertions) && redirect_enabled.load(Ordering::Relaxed) {
                            unreachable!("headless apps don't use redirect")
                        } else if headless_app_ev_sender.send(AppEvent::Request(req)).is_err() {
                            // event-loop shutdown
                            return;
                        }
                    }
                    Err(ipc::Disconnected) => {
                        let _ = headless_app_ev_sender.send(AppEvent::ParentProcessExited);
                        return;
                    }
                }
            }
        });
    } else {
        // requests are inserted in the winit event loop.
        let request_sender = event_loop.create_proxy();
        let redirect_enabled = redirect_enabled.clone();
        thread::spawn(move || {
            loop {
                // wait for requests, every second checks if app-process is still running.
                match request_receiver.recv() {
                    Ok(req) => {
                        if redirect_enabled.load(Ordering::Relaxed) {
                            redirect_sender.send(req).expect("redirect_sender error");
                        } else if request_sender.send_event(AppEvent::Request(req)).is_err() {
                            // event-loop shutdown
                            return;
                        }
                    }
                    Err(ipc::Disconnected) => {
                        let _ = request_sender.send(AppEvent::ParentProcessExited);
                        return;
                    }
                }
            }
        });
    }

    let el = event_loop.create_proxy();
    let gl_manager = util::GlContextManager::default();

    if headless {
        let mut app = ViewApp::new(
            headless_app_ev_sender.clone(),
            response_sender,
            event_sender,
            redirect_enabled,
            redirect_receiver,
            headless,
        );

        let ctx = Context {
            event_loop: &el,
            app_ev_sender: &headless_app_ev_sender,
            window_target: &event_loop,
            gl_manager: &gl_manager,
        };

        loop {
            match headless_app_ev_receiver.recv().expect("headless receiver error") {
                AppEvent::Request(req) => app.on_request(&ctx, req),
                AppEvent::FrameReady(_) => unreachable!("headless-app FrameReady"),
                AppEvent::HeadlessFrameReady(id) => app.on_headless_frame_ready(id),
                AppEvent::RefreshMonitors => unreachable!("headless-app RefreshMonitors"),
                AppEvent::Notify(ev) => app.notify(ev),
                AppEvent::ParentProcessExited => {
                    if let Some(app_thread) = same_process_app.take() {
                        if let Err(p) = app_thread.join() {
                            std::panic::resume_unwind(p);
                        }
                    }
                    std::process::exit(0)
                }
            }
        }
    }

    let mut app = ViewApp::new(
        event_loop.create_proxy(),
        response_sender,
        event_sender,
        redirect_enabled,
        redirect_receiver,
        headless,
    );

    #[cfg(windows)]
    let config_listener = config::config_listener(&Context {
        event_loop: &el,
        app_ev_sender: &el,
        window_target: &event_loop,
        gl_manager: &gl_manager,
    });

    event_loop.run(move |event, window_target, control| {
        *control = ControlFlow::Wait; // will wait after current event sequence.

        let ctx = Context {
            event_loop: &el,
            app_ev_sender: &el,
            window_target,
            gl_manager: &gl_manager,
        };

        match event {
            Event::NewEvents(_) => {}
            Event::WindowEvent { window_id, event } => {
                #[cfg(windows)]
                if window_id == config_listener.id() {
                    return; // ignore events for this window.
                }
                app.on_window_event(&ctx, window_id, event)
            }
            Event::DeviceEvent { device_id, event } => app.on_device_event(device_id, event),
            Event::UserEvent(ev) => match ev {
                AppEvent::Request(req) => app.on_request(&ctx, req),
                AppEvent::FrameReady(window_id) => app.on_frame_ready(window_id),
                AppEvent::HeadlessFrameReady(id) => app.on_headless_frame_ready(id),
                AppEvent::RefreshMonitors => app.refresh_monitors(&ctx),
                AppEvent::Notify(ev) => app.notify(ev),
                AppEvent::ParentProcessExited => {
                    *control = ControlFlow::Exit;
                }
            },
            Event::Suspended => {}
            Event::Resumed => {}
            Event::MainEventsCleared => app.on_events_cleared(),
            Event::RedrawRequested(w) => app.on_redraw(w),
            Event::RedrawEventsCleared => {}
            Event::LoopDestroyed => {
                // this happens if we detect the app-process exited,
                // normally the app-process kills the view-process.
                //
                // OR in same_process mode, if the app is shutting-down.

                if let Some(app_thread) = same_process_app.take() {
                    if let Err(p) = app_thread.join() {
                        std::panic::resume_unwind(p);
                    }
                }
            }
        }
    })
}

pub(crate) struct Context<'a, E: AppEventSender> {
    pub event_loop: &'a EventLoopProxy<AppEvent>,
    pub app_ev_sender: &'a E,
    pub window_target: &'a EventLoopWindowTarget<AppEvent>,
    pub gl_manager: &'a util::GlContextManager,
}

/// Custom event loop event.
pub(crate) enum AppEvent {
    Request(Request),
    FrameReady(WindowId),
    HeadlessFrameReady(WinId),
    RefreshMonitors,
    Notify(Ev),
    ParentProcessExited,
}

/// Can be `EventLoopProxy<AppEvent>` or `flume::Sender<AppEvent>` in headless apps.
pub(crate) trait AppEventSender: Send {
    fn clone_boxed(&self) -> Box<dyn AppEventSender>;
    fn send(&self, ev: AppEvent) -> ipc::Result<()>;
}
impl AppEventSender for EventLoopProxy<AppEvent> {
    fn clone_boxed(&self) -> Box<dyn AppEventSender> {
        Box::new(self.clone())
    }
    fn send(&self, ev: AppEvent) -> ipc::Result<()> {
        self.send_event(ev).map_err(|_| ipc::Disconnected)
    }
}
impl AppEventSender for flume::Sender<AppEvent> {
    fn clone_boxed(&self) -> Box<dyn AppEventSender> {
        Box::new(self.clone())
    }
    fn send(&self, ev: AppEvent) -> ipc::Result<()> {
        self.send(ev).map_err(|_| ipc::Disconnected)
    }
}
