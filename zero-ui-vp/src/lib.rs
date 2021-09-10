//! Zero-Ui View Process.
//!
//! Zero-Ui isolates all OpenGL and windowing related code to a different process to be able to recover from driver errors.
//! This crate contains the `glutin` and `webrender` code that interacts with the actual system. Communication
//! with the app process is done using `ipmpsc`.

#![allow(unused_parens)]

use config::*;
use glutin::{
    event::*,
    event_loop::{ControlFlow, EventLoop, EventLoopProxy, EventLoopWindowTarget},
    monitor::MonitorHandle,
    window::WindowId,
};
use headless::ViewHeadless;
use parking_lot::{Condvar, Mutex};
use serde::{Deserialize, Serialize};
use std::{
    env, panic,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};
use types::RunOnDrop;
use util::GlContextManager;
use window::ViewWindow;

mod config;
mod headless;
mod ipc;
mod types;
mod util;
mod window;

const SERVER_NAME_VAR: &str = "ZERO_UI_WR_SERVER";
const MODE_VAR: &str = "ZERO_UI_WR_MODE";

/// Version 0.1.
///
/// The *App Process* and *View Process* must be build using the same exact version of `zero-ui-vp` and this is
/// validated during run-time, causing a panic if the versions don't match. Usually the same executable is used
/// for both processes so this is not a problem.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

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

struct SameProcessConfig {
    waiter: Arc<Condvar>,
    server_name: String,
    headless: bool,
}
static SAME_PROCESS_CONFIG: Mutex<Option<SameProcessConfig>> = parking_lot::const_mutex(None);

/// Run both View and App in the same process.
///
/// This function must be called in the main thread, it initializes the View and calls `run_app`
/// in a new thread to initialize the App.

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

pub use types::{
    AxisId, ButtonId, ByteBuf, CursorIcon, DevId, ElementState, Ev, EventCause, FramePixels, FrameRequest, HeadlessConfig, Icon,
    ModifiersState, MonId, MonitorInfo, MouseButton, MouseScrollDelta, MultiClickConfig, Respawned, Result, ScanCode, TextAntiAliasing,
    VideoMode, ViewProcessGen, VirtualKeyCode, WinId, WindowConfig, WindowTheme,
};

use webrender::api::{
    units::{LayoutPoint, LayoutRect, LayoutSize},
    DynamicProperties, Epoch, FontInstanceKey, FontKey, HitTestResult, IdNamespace, ImageKey, PipelineId, ResourceUpdate,
};

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
    let gl_manager = GlContextManager::default();

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
    pub gl_manager: &'a GlContextManager,
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

/// Declares the `Request` and `Response` enums, and two methods in `Controller` and `ViewApp`, in the
/// controller it packs and sends the request and receives and unpacks the response. In the view it implements
/// the method.
macro_rules! declare_ipc {
    (
        $(
            $(#[$doc:meta])*
            $vis:vis fn $method:ident(&mut $self:ident, $ctx:ident: &Context $(, $input:ident : $RequestType:ty)* $(,)?) -> $ResponseType:ty {
                $($impl:tt)*
            }
        )*
    ) => {
        #[derive(Debug, Serialize, Deserialize)]
        #[allow(non_camel_case_types)]
        #[allow(clippy::large_enum_variant)]
        #[repr(u32)]
        pub(crate) enum Request {
            $(
                $method { $($input: $RequestType),* },
            )*
        }

        #[derive(Debug, Serialize, Deserialize)]
        #[allow(non_camel_case_types)]
        #[repr(u32)]
        pub(crate) enum Response {
            $(
                $method($ResponseType),
            )*
        }

        #[allow(unused_parens)]
        impl Controller {
            $(
                $(#[$doc])*
                #[allow(clippy::too_many_arguments)]
                $vis fn $method(&mut self $(, $input: $RequestType)*) -> Result<$ResponseType> {
                    match self.talk(Request::$method { $($input),* })? {
                        Response::$method(r) => Ok(r),
                        _ => panic!("view-process did not respond correctly")
                    }
                }
            )*
        }

        #[allow(unused_parens)]
        impl<E: AppEventSender> ViewApp<E> {
            pub fn on_request(&mut self, ctx: &Context<E>, request: Request) {
                match request {
                    $(
                        Request::$method { $($input),* } => {
                            let r = self.$method(ctx, $($input),*);
                            self.respond(Response::$method(r));
                        }
                    )*
                }
            }

            $(
                #[allow(clippy::too_many_arguments)]
                fn $method(&mut $self, $ctx: &Context<E> $(, $input: $RequestType)*) -> $ResponseType {
                    $($impl)*
                }
            )*
        }
    };
}

/// The listener returns the closure on join for reuse in respawn.
type EventListenerJoin = JoinHandle<Box<dyn FnMut(Ev) + Send>>;

/// View Process controller, used in the App Process.
///
/// # Shutdown
///
/// The View Process is [killed] when the controller is dropped, if the app is running in [same process mode]
/// then the current process [exits] with code 0 on drop.
///
/// [killed]: std::process::Child::kill
/// [same process mode]: run_same_process
/// [exits]: std::process::exit
pub struct Controller {
    process: Option<Child>,
    generation: ViewProcessGen,
    view_process_exe: PathBuf,
    request_sender: ipc::RequestSender,
    response_receiver: ipc::ResponseReceiver,
    event_listener: Option<EventListenerJoin>,
    headless: bool,
    same_process: bool,
    device_events: bool,
    last_respawn: Instant,
    fast_respawn_count: u8,
}
impl Controller {
    /// Start with a custom view process.
    ///
    /// The `view_process_exe` must be an executable that calls [`init_view_process`], if not set
    /// the [`current_exe`] is used. Note that the [`VERSION`] of this crate must match in both executables.
    ///
    /// The `on_event` closure is called in another thread every time the app receives an event.
    ///
    /// # Tests
    ///
    /// The [`current_exe`] cannot be used in tests, you should set an external view-process executable. Unfortunately there
    /// is no way to check if `start` was called in a test so we cannot provide an error message for this.
    /// If the test is hanging in debug builds or has a timeout error in release builds this is probably the reason.
    ///
    /// Also is unlikely that you can use [`run_same_process`], because it must be run in the main thread.
    ///
    /// [`current_exe`]: std::env::current_exe
    /// [`init_view_process`]: crate::init_view_process
    /// [`VERSION`]: crate::VERSION
    pub fn start<F>(view_process_exe: Option<PathBuf>, device_events: bool, headless: bool, mut on_event: F) -> Self
    where
        F: FnMut(Ev) + Send + 'static,
    {
        let view_process_exe = view_process_exe.unwrap_or_else(|| {
            std::env::current_exe().expect("failed to get the current exetuable, consider using an external view-process exe")
        });

        let (process, request_sender, response_receiver, mut event_receiver) =
            Self::spawn_view_process(&view_process_exe, headless).expect("failed to spawn or connecto to view-process");

        let ev = thread::spawn(move || {
            while let Ok(ev) = event_receiver.recv() {
                on_event(ev);
            }
            on_event(Ev::Disconnected(1));

            // return to reuse in respawn.
            let t: Box<dyn FnMut(Ev) + Send> = Box::new(on_event);
            t
        });

        let mut c = Controller {
            same_process: process.is_none(),
            process,
            view_process_exe,
            request_sender,
            response_receiver,
            event_listener: Some(ev),
            headless,
            device_events,
            generation: 1,
            last_respawn: Instant::now(),
            fast_respawn_count: 0,
        };

        if let Err(Respawned) = c.try_startup() {
            panic!("respawn on startup"); // TODO recover from this
        }

        c
    }
    fn try_startup(&mut self) -> Result<()> {
        if crate::VERSION != self.version()? {
            panic!("app-process and view-process must be build using the same exact version of zero-ui-vp");
        }
        assert!(self.startup(self.generation, self.device_events, self.headless)?);
        Ok(())
    }

    /// View-process generation.
    pub fn generation(&self) -> ViewProcessGen {
        self.generation
    }

    /// If is running in headless mode.
    #[inline]
    pub fn headless(&self) -> bool {
        self.headless
    }

    /// If device events are enabled.
    #[inline]
    pub fn device_events(&self) -> bool {
        self.device_events
    }

    /// If is running both view and app in the same process.
    #[inline]
    pub fn same_process(&self) -> bool {
        self.same_process
    }

    fn try_talk(&mut self, req: Request) -> ipc::Result<Response> {
        self.request_sender.send(req)?;
        self.response_receiver.recv()
    }

    fn talk(&mut self, req: Request) -> Result<Response> {
        match self.try_talk(req) {
            Ok(r) => return Ok(r),
            Err(ipc::Disconnected) => {
                self.handle_disconnect(self.generation);
            }
        }
        Err(Respawned)
    }

    fn spawn_view_process(
        view_process_exe: &Path,
        headless: bool,
    ) -> util::AnyResult<(Option<Child>, ipc::RequestSender, ipc::ResponseReceiver, ipc::EvReceiver)> {
        let init = ipc::AppInit::new();

        // create process and spawn it, unless is running in same process mode.
        let process = if let Some(config) = &mut *SAME_PROCESS_CONFIG.lock() {
            config.server_name = init.name().to_owned();
            config.headless = headless;
            config.waiter.notify_one();
            None
        } else {
            let process = Command::new(&view_process_exe)
                .env(SERVER_NAME_VAR, init.name())
                .env(MODE_VAR, if headless { "headless" } else { "headed" })
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()?;
            Some(process)
        };

        let (req, rsp, ev) = match init.connect() {
            Ok(r) => r,
            Err(e) => {
                if let Some(mut p) = process {
                    if let Err(e) = p.kill() {
                        log::error!(
                            "failed to kill new view-process after failing to connect to it\n connection error: {:?}\n kill error: {:?}",
                            p,
                            e
                        );
                    }
                }
                return Err(e);
            }
        };

        Ok((process, req, rsp, ev))
    }

    /// Handle an [`Ev::Disconnected`].
    ///
    /// The process will exit if the view-process was killed by the user.
    pub fn handle_disconnect(&mut self, gen: ViewProcessGen) {
        if gen == self.generation {
            log::error!(target: "vp_respawn", "channel disconnect, will try respawn");
            self.respawn_impl(false)
        }
    }

    /// Reopen the view-process, causing an [`Ev::Respawned`].
    pub fn respawn(&mut self) {
        self.respawn_impl(true);
    }
    fn respawn_impl(&mut self, respawn: bool) {
        let mut process = if let Some(p) = self.process.take() {
            p
        } else {
            if self.same_process {
                log::error!("cannot recover in same_process mode");
            }
            return;
        };

        let t = Instant::now();
        if t - self.last_respawn < Duration::from_millis(500) {
            self.fast_respawn_count += 1;
            if self.fast_respawn_count > 5 {
                panic!("respawn requested 5 times less than 500ms apart");
            }
        } else {
            self.fast_respawn_count = 0;
        }
        self.last_respawn = t;

        // try exit
        let _ = process.kill();
        let code_and_output = match process.wait_with_output() {
            Ok(c) => Some(c),
            Err(e) => {
                log::error!(target: "vp_respawn", "view-process could not be heaped, will abandon running, {:?}", e);
                None
            }
        };

        // try print stdout/err and exit code.
        if let Some(c) = code_and_output {
            log::info!(target: "vp_respawn", "view-process reaped");

            let code = c.status.code();

            if !respawn {
                // check if user killed the view-process, in this case we exit too.

                #[cfg(windows)]
                if code == Some(1) {
                    log::warn!(target: "vp_respawn", "view-process exit code is `1`, probably killed by the Task Manager, \
                                        will exit app-process with the same code");
                    std::process::exit(1);
                }

                #[cfg(unix)]
                if code == None {
                    log::warn!(target: "vp_respawn", "view-process exited by signal, probably killed by the user, \
                                        will exit app-process too");
                    std::process::exit(1);
                }
            }

            let code = code.unwrap();

            if !respawn {
                log::error!(target: "vp_respawn", "view-process exit_code: {:x}", code);
            }

            match String::from_utf8(c.stderr) {
                Ok(s) => {
                    if !s.is_empty() {
                        log::error!(target: "vp_respawn", "view-process stderr:\n```stderr\n{}\n```", s)
                    }
                }
                Err(e) => log::error!(target: "vp_respawn", "failed to read view-process stderr: {}", e),
            }

            match String::from_utf8(c.stdout) {
                Ok(s) => {
                    if !s.is_empty() {
                        log::info!(target: "vp_respawn", "view-process stdout:\n```stdout\n{}\n```", s)
                    }
                }
                Err(e) => log::error!(target: "vp_respawn", "failed to read view-process stdout: {}", e),
            }
        } else {
            log::error!(target: "vp_respawn", "failed to reap view-process, will abandon it running and spawn a new one");
        }

        // recover event listener closure (in a box).
        let mut on_event = match self.event_listener.take().unwrap().join() {
            Ok(fn_) => fn_,
            Err(p) => panic::resume_unwind(p),
        };

        // respawn
        let mut retries = 3;
        let (new_process, request, response, mut event) = loop {
            match Self::spawn_view_process(&self.view_process_exe, self.headless) {
                Ok(r) => break r,
                Err(e) => {
                    log::error!(target: "vp_respawn",  "failed to respawn, {:?}", e);
                    retries -= 1;
                    if retries == 0 {
                        panic!("failed to respawn `view-process` after 3 retries");
                    }
                    log::info!(target: "vp_respawn", "retrying respawn");
                }
            }
        };

        // update connections
        self.process = new_process;
        self.request_sender = request;
        self.response_receiver = response;

        let mut next_id = self.generation.wrapping_add(1);
        if next_id == 0 {
            next_id = 1;
        }
        self.generation = next_id;

        if let Err(Respawned) = self.try_startup() {
            panic!("respawn on respawn startup"); // TODO recover from this?
        }

        let ev = thread::spawn(move || {
            // notify a respawn.
            on_event(Ev::Respawned(next_id));
            while let Ok(ev) = event.recv() {
                on_event(ev);
            }
            on_event(Ev::Disconnected(next_id));

            on_event
        });
        self.event_listener = Some(ev);
    }
}
impl Drop for Controller {
    /// Kills the View Process, unless it is running in the same process.
    fn drop(&mut self) {
        if let Some(mut process) = self.process.take() {
            let _ = process.kill();
        } else {
            let _ = self.exit_same_process();
        }
    }
}

/// The View Process.
pub(crate) struct ViewApp<E> {
    event_loop: E,
    response_chan: ipc::ResponseSender,
    event_chan: ipc::EvSender,

    generation: ViewProcessGen,

    redirect_enabled: Arc<AtomicBool>,
    redirect_chan: flume::Receiver<Request>,

    started: bool,
    device_events: bool,
    headless: bool,

    window_id_count: WinId,
    windows: Vec<ViewWindow>,
    headless_views: Vec<ViewHeadless>,

    monitor_id_count: MonId,
    monitors: Vec<(MonId, MonitorHandle)>,

    device_id_count: DevId,
    devices: Vec<(DevId, DeviceId)>,

    // if one or more events where send after the last on_events_cleared.
    pending_clear: bool,
}
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

    fn respond(&mut self, response: Response) {
        if self.response_chan.send(response).is_err() {
            let _ = self.event_loop.send(AppEvent::ParentProcessExited);
        }
    }
    fn notify(&mut self, event: Ev) {
        self.pending_clear = true;
        if self.event_chan.send(event).is_err() {
            let _ = self.event_loop.send(AppEvent::ParentProcessExited);
        }
    }

    fn monitor_id(&mut self, handle: &MonitorHandle) -> MonId {
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

    fn with_window<R>(&mut self, id: WinId, d: impl FnOnce() -> R, f: impl FnOnce(&mut ViewWindow) -> R) -> R {
        assert!(self.started);
        if let Some(w) = self.windows.iter_mut().find(|w| w.id() == id) {
            f(w)
        } else {
            d()
        }
    }

    fn with_headless<R>(&mut self, id: WinId, d: impl FnOnce() -> R, f: impl FnOnce(&mut ViewHeadless) -> R) -> R {
        assert!(self.started);
        if let Some(w) = self.headless_views.iter_mut().find(|w| w.id() == id) {
            f(w)
        } else {
            d()
        }
    }
}
macro_rules! with_window_or_headless {
    ($self:ident, $id:ident, $default:expr, |$w:ident| $($expr:tt)+) => {
        if !$self.started {
            panic!("expected `self.started`");
        } else if let Some($w) = $self.windows.iter_mut().find(|w| w.id() == $id) {
            $($expr)+
        } else if let Some($w) = $self.headless_views.iter_mut().find(|w| w.id() == $id) {
            $($expr)+
        } else {
            $default
        }
    }
}
declare_ipc! {
    fn version(&mut self, _ctx: &Context) -> String {
        crate::VERSION.to_string()
    }

    fn startup(&mut self, _ctx: &Context, gen: ViewProcessGen, device_events: bool, headless: bool) -> bool {
        assert!(!self.started, "view-process already started");

        self.generation = gen;
        self.device_events = device_events;

        assert!(self.headless == headless, "view-process environemt and startup do not agree");

        self.started = true;
        true
    }

    fn exit_same_process(&mut self, _ctx: &Context) -> () {
        let _ = self.event_loop.send(AppEvent::ParentProcessExited);
    }

    /// Returns the primary monitor if there is any or the first available monitor or none if no monitor was found.
    pub fn primary_monitor(&mut self, ctx: &Context) -> Option<(MonId, MonitorInfo)> {
        ctx.window_target
        .primary_monitor()
        .or_else(|| ctx.window_target.available_monitors().next())
        .map(|m| {
            let id = self.monitor_id(&m);
            let mut info = MonitorInfo::from(m);
            info.is_primary = true;
            (id, info)
        })
    }

    /// Returns information about the specific monitor, if it exists.
    pub fn monitor_info(&mut self, ctx: &Context, id: MonId) -> Option<MonitorInfo> {
        self.monitors.iter().find(|(i, _)| *i == id).map(|(_, h)| {
            let mut info = MonitorInfo::from(h);
            info.is_primary = ctx.window_target
                .primary_monitor()
                .map(|p| &p == h)
                .unwrap_or(false);
            info
        })
    }

    /// Returns all available monitors.
    pub fn available_monitors(&mut self, ctx: &Context) -> Vec<(MonId, MonitorInfo)> {
        let primary = ctx.window_target.primary_monitor();

        ctx.window_target
        .available_monitors()
        .map(|m| {
            let id = self.monitor_id(&m);
            let is_primary = primary.as_ref().map(|h|h == &m).unwrap_or(false);
            let mut info = MonitorInfo::from(m);
            info.is_primary = is_primary;
            (id, info)
        })
        .collect()
    }

    /// Open a window.
    ///
    /// Returns the window id, and renderer ids.
    pub fn open_window(
        &mut self,
        ctx: &Context,
        config: WindowConfig,
    ) -> (WinId, IdNamespace, PipelineId) {
        assert!(self.started);

        let mut id = self.window_id_count.wrapping_add(1);
        if id == 0 {
            id = 1;
        }
        self.window_id_count = id;

        let window = ViewWindow::new(ctx, self.generation, id, config);
        let namespace = window.namespace_id();
        let pipeline = window.pipeline_id();

        self.windows.push(window);

        (id, namespace, pipeline)
    }

    /// Open a headless surface.
    ///
    /// This is a real renderer but not connected to any window, you can requests pixels to get the
    /// rendered frames.
    ///
    /// The surface is identified with a "window" id, but no window is created, also returns the renderer ids.
    pub fn open_headless(&mut self, ctx: &Context, config: HeadlessConfig) -> (WinId, IdNamespace, PipelineId) {
        assert!(self.started);

        let mut id = self.window_id_count.wrapping_add(1);
        if id == 0 {
            id = 1;
        }
        self.window_id_count = id;

        let view = ViewHeadless::new(ctx, self.generation, id, config);
        let namespace = view.namespace_id();
        let pipeline = view.pipeline_id();

        self.headless_views.push(view);

        (id, namespace, pipeline)
    }

    /// Close the window or headless surface.
    pub fn close_window(&mut self, _ctx: &Context, id: WinId) -> () {
        assert!(self.started);

        if let Some(i) = self.windows.iter().position(|w|w.id() == id) {
            self.windows.remove(i);
        } else if let Some(i) = self.headless_views.iter().position(|h|h.id() == id) {
            self.headless_views.remove(i);
        }
    }

    /// Reads the default text anti-aliasing.
    ///
    /// # TODO
    ///
    /// Only implemented for Windows, other systems return `TextAntiAliasing::Subpixel`.
    pub fn text_aa(&mut self, _ctx: &Context) -> TextAntiAliasing {
        text_aa()
    }

    /// Reads the system "double-click" config.
    ///
    /// # TODO
    ///
    /// Only implemented for Windows, other systems return [`MultiClickConfig::default`].
    pub fn multi_click_config(&mut self, _ctx: &Context) -> MultiClickConfig {
        multi_click_config()
    }

    /// Returns `true` if animations are enabled in the operating system.
    ///
    /// People with photosensitive epilepsy usually disable animations system wide.
    ///
    /// # TODO
    ///
    /// Only implemented for Windows, other systems return `true`.
    pub fn animation_enabled(&mut self, _ctx: &Context) -> bool {
        animation_enabled()
    }

    /// Retrieves the keyboard repeat-delay setting from the operating system.
    ///
    /// If the user holds a key pressed a new key-press event will happen every time this delay is elapsed.
    /// Note, depending on the hardware the real delay can be slightly different.
    ///
    /// There is no repeat flag in the `winit` key press event, so as a general rule we consider a second key-press
    /// without any other keyboard event within the window of time of twice this delay as a repeat.
    ///
    /// This delay can also be used as the text-boxes caret blink rate.
    ///
    /// # TODO
    ///
    /// Only implemented for Windows, other systems return `600ms`.
    pub fn key_repeat_delay(&mut self, _ctx: &Context) -> Duration {
        key_repeat_delay()
    }

    /// Set window title.
    pub fn set_title(&mut self, _ctx: &Context, id: WinId, title: String) -> () {
        self.with_window(id, ||(), |w| w.set_title(title))
    }

    /// Set window visible.
    pub fn set_visible(&mut self, _ctx: &Context, id: WinId, visible: bool) -> () {
        self.with_window(id, ||(), |w| w.set_visible(visible))
    }

    /// Set if the window is "top-most".
    pub fn set_always_on_top(&mut self, _ctx: &Context, id: WinId, always_on_top: bool) -> () {
        self.with_window(id, ||(), |w| w.set_always_on_top(always_on_top))
    }

    /// Set if the user can drag-move the window.
    pub fn set_movable(&mut self, _ctx: &Context, id: WinId, movable: bool) -> () {
        self.with_window(id, ||(), |w| w.set_movable(movable))
    }

    /// Set if the user can resize the window.
    pub fn set_resizable(&mut self, _ctx: &Context, id: WinId, resizable: bool) -> () {
        self.with_window(id, ||(), |w| w.set_resizable(resizable))
    }

    /// Set the window taskbar icon visibility.
    pub fn set_taskbar_visible(&mut self, _ctx: &Context, id: WinId, visible: bool) -> () {
        self.with_window(id, ||(), |w| w.set_taskbar_visible(visible))
    }

    /// Set the window parent and if `self` blocks the parent events while open (`modal`).
    pub fn set_parent(&mut self, _ctx: &Context, id: WinId, parent: Option<WinId>, modal: bool) -> () {
        if let Some(parent_id) = parent {
            if let Some(parent_id) = self.windows.iter().find(|w|w.id() == parent_id).map(|w|w.actual_id()) {
                self.with_window(id, ||(), |w|w.set_parent(Some(parent_id), modal))
            } else {
                self.with_window(id, ||(), |w| w.set_parent(None, modal))
            }
        } else {
            self.with_window(id, ||(), |w| w.set_parent(None, modal))
        }
    }

    /// Set if the window is see-through.
    pub fn set_transparent(&mut self, _ctx: &Context, id: WinId, transparent: bool) -> () {
        self.with_window(id, ||(), |w| w.set_transparent(transparent))
    }

    /// Set the window system border and title visibility.
    pub fn set_chrome_visible(&mut self, _ctx: &Context, id: WinId, visible: bool) -> () {
        self.with_window(id, ||(), |w|w.set_chrome_visible(visible))
    }

    /// Set the window top-left offset, includes the window chrome (outer-position).
    pub fn set_position(&mut self, _ctx: &Context, id: WinId, pos: LayoutPoint) -> () {
        if self.with_window(id, ||false, |w|w.set_outer_pos(pos)) {
            self.notify(Ev::WindowMoved(id, pos, EventCause::App));
        }
    }

    /// Set the window content area size (inner-size).
    pub fn set_size(&mut self, _ctx: &Context, id: WinId, size: LayoutSize, frame: FrameRequest) -> () {
        let frame_id = frame.id;
        let (resized, rendered) = self.with_window(id, ||(false, false), |w|w.resize_inner(size, frame));
        if resized {
            self.notify(Ev::WindowResized(id, size, EventCause::App));
            if rendered {
                self.notify(Ev::FrameRendered(id, frame_id))
            }
        }
    }

    /// Set the headless surface are size (viewport size).
    pub fn set_headless_size(&mut self, _ctx: &Context, id: WinId, size: LayoutSize, scale_factor: f32) -> () {
        self.with_headless(id, ||(), |h|h.set_size(size, scale_factor))
    }

    /// Set the window minimum content area size.
    pub fn set_min_size(&mut self, _ctx: &Context, id: WinId, size: LayoutSize) -> () {
        self.with_window(id, ||(), |w|w.set_min_inner_size(size))
    }
    /// Set the window maximum content area size.
    pub fn set_max_size(&mut self, _ctx: &Context, id: WinId, size: LayoutSize) -> () {
        self.with_window(id, ||(), |w|w.set_max_inner_size(size))
    }

    /// Set the window icon.
    pub fn set_icon(&mut self, _ctx: &Context, id: WinId, icon: Option<Icon>) -> () {
        self.with_window(id, ||(), |w|w.set_icon(icon))
    }

    /// Gets the root pipeline ID.
    pub fn pipeline_id(&mut self, _ctx: &Context, id: WinId) -> PipelineId {
        with_window_or_headless!(self, id, PipelineId::dummy(), |w|w.pipeline_id())
    }

    /// Gets the resources namespace.
    pub fn namespace_id(&mut self, _ctx: &Context, id: WinId) -> IdNamespace {
        with_window_or_headless!(self, id, IdNamespace(0), |w|w.namespace_id())
    }

    /// New image resource key.
    pub fn generate_image_key(&mut self, _ctx: &Context, id: WinId) -> ImageKey {
        with_window_or_headless!(self, id, ImageKey::DUMMY, |w|w.generate_image_key())
    }

    /// New font resource key.
    pub fn generate_font_key(&mut self, _ctx: &Context, id: WinId) -> FontKey {
        with_window_or_headless!(self, id, FontKey(IdNamespace(0), 0), |w|w.generate_font_key())
    }

    /// New font instance key.
    pub fn generate_font_instance_key(&mut self, _ctx: &Context, id: WinId) -> FontInstanceKey {
        with_window_or_headless!(self, id, FontInstanceKey(IdNamespace(0), 0), |w|w.generate_font_instance_key())
    }

    /// Gets the window content are size.
    pub fn size(&mut self, _ctx: &Context, id: WinId) -> LayoutSize {
        with_window_or_headless!(self, id, LayoutSize::zero(), |w|w.size())
    }

    /// Gets the window content are size.
    pub fn scale_factor(&mut self, _ctx: &Context, id: WinId) -> f32 {
        with_window_or_headless!(self, id, 1.0, |w|w.scale_factor())
    }

    /// In Windows, set if the `Alt+F4` should not cause a window close request and instead generate a key-press event.
    pub fn set_allow_alt_f4(&mut self, _ctx: &Context, id: WinId, allow: bool) -> () {
        self.with_window(id, ||(), |w|w.set_allow_alt_f4(allow))
    }

    /// Read all pixels of the current frame.
    ///
    /// This is a call to `glReadPixels`, the first pixel row order is bottom-to-top and the pixel type is BGRA.
    pub fn read_pixels(&mut self, _ctx: &Context, id: WinId) -> FramePixels {
        with_window_or_headless!(self, id, FramePixels::default(), |w|w.read_pixels())
    }

    /// `glReadPixels` a new buffer.
    ///
    /// This is a call to `glReadPixels`, the first pixel row order is bottom-to-top and the pixel type is BGRA.
    pub fn read_pixels_rect(&mut self, _ctx: &Context, id: WinId, rect: LayoutRect) -> FramePixels {
        with_window_or_headless!(self, id, FramePixels::default(), |w|w.read_pixels_rect(rect))
    }

    /// Get display items of the last rendered frame that intercept the `point`.
    ///
    /// Returns the frame ID and all hits from front-to-back.
    pub fn hit_test(&mut self, _ctx: &Context, id: WinId, point: LayoutPoint) -> (Epoch, HitTestResult) {
        with_window_or_headless!(self, id, (Epoch(0), HitTestResult::default()), |w|w.hit_test(point))
    }

    /// Set the text anti-aliasing used in the window renderer.
    pub fn set_text_aa(&mut self, _ctx: &Context, id: WinId, aa: TextAntiAliasing) -> () {
        with_window_or_headless!(self, id, (), |w|w.set_text_aa(aa))
    }

    /// Render a new frame.
    pub fn render(&mut self, _ctx: &Context, id: WinId, frame: FrameRequest) -> () {
        with_window_or_headless!(self, id, (), |w|w.render(frame))
    }

    /// Update the current frame and re-render it.
    pub fn render_update(&mut self, _ctx: &Context, id: WinId, updates: DynamicProperties) -> () {
        with_window_or_headless!(self, id, (), |w|w.render_update(updates))
    }

    /// Add/remove/update resources such as images and fonts.
    pub fn update_resources(&mut self, _ctx: &Context, id: WinId, updates: Vec<ResourceUpdate>) -> () {
        with_window_or_headless!(self, id, (), |w| w.update_resources(updates))
    }

    /// Can be used to profile the ipc-channel.
    pub fn ping(&mut self, _ctx: &Context, _bytes: ByteBuf) -> () {
        todo!()
    }
}
impl<E: AppEventSender> ViewApp<E> {
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
                if !self.windows[i].resized(size) {
                    return;
                }
                // give the app 300ms to send a new frame, this is the collaborative way to
                // resize, it should reduce the changes of the user seeing the clear color.

                let size = LayoutSize::new(size.width as f32 / scale_factor, size.height as f32 / scale_factor);

                let redirect_enabled = self.redirect_enabled.clone();
                redirect_enabled.store(true, Ordering::Relaxed);
                let stop_redirect = RunOnDrop::new(|| redirect_enabled.store(false, Ordering::Relaxed));

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
                if !self.windows[i].moved(p) {
                    return;
                }

                let p = LayoutPoint::new(p.x as f32 / scale_factor, p.y as f32 / scale_factor);
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
                self.notify(Ev::KeyboardInput(id, d_id, input))
            }
            WindowEvent::ModifiersChanged(m) => {
                self.refresh_monitors(ctx);
                self.notify(Ev::ModifiersChanged(id, m));
            }
            WindowEvent::CursorMoved { device_id, position, .. } => {
                let p = LayoutPoint::new(position.x as f32 / scale_factor, position.y as f32 / scale_factor);
                let d_id = self.device_id(device_id);
                let (f_id, ht) = self.windows[i].hit_test(p);
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
                let location = LayoutPoint::new(t.location.x as f32 / scale_factor, t.location.y as f32 / scale_factor);
                self.notify(Ev::Touch(id, d_id, t.phase, location, t.force.map(Into::into), t.id));
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => self.notify(Ev::ScaleFactorChanged(id, scale_factor as f32)),
            WindowEvent::ThemeChanged(t) => self.notify(Ev::ThemeChanged(id, t.into())),
        }
    }

    fn on_device_event(&mut self, device_id: DeviceId, event: DeviceEvent) {
        if self.device_events {
            let d_id = self.device_id(device_id);
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
