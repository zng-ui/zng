#![allow(clippy::needless_doctest_main)]
#![doc(test(no_crate_inject))]
#![warn(missing_docs)]
#![warn(unused_extern_crates)]

//! View-Process implementation using [`glutin`].
//!
//! This backend supports headed and headless apps and all .
//!
//! # Usage
//!
//! First add this to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! zero-ui = "0.1"
//! zero-ui-view = "0.1"
//! ```
//!
//! Then call [`init`] before any other code in `main` to setup a view-process that uses
//! the same app executable:
//!
//! ```no_run
//! # pub mod zero_ui { pub mod prelude {
//! # pub struct App { }
//! # impl App {
//! # pub fn default() -> Self { unimplemented!() }
//! # pub fn run_window(self, f: impl FnOnce(bool)) { }
//! # } } }
//! use zero_ui::prelude::*;
//!
//! fn main() {
//!     zero_ui_view::init();
//!
//!     App::default().run_window(|ctx| {
//!         unimplemented!()
//!     })
//! }
//! ```
//!
//! When the app is executed `init` setup its startup and returns, `run_window` gets called and
//! internally starts the view-process, using the `init` setup. The current executable is started
//! again, this time configured to be a view-process, `init` detects this and highjacks the process
//! **never returning**.
//!
//! # Software Backend
//!
//! The `webrender/swgl` software renderer can be used as fallback when no native OpenGL 3.2 driver is available, to build it
//! the feature `"software"` must be enabled (it is by default) and on Windows MSVC the `clang-cl` dependency must be installed and
//! associated with the `CC` and `CXX` environment variables, if requirements are not met a warning is emitted and the build fails.
//!
//! To install dependencies on Windows:
//!
//! * Install LLVM (<https://releases.llvm.org/>) and add it to the `PATH` variable:
//! ```bat
//! setx PATH %PATH%;C:\Program Files\LLVM\bin
//! ```
//! * Associate `CC` and `CXX` with `clang-cl`:
//! ```bat
//! setx CC clang-cl
//! setx CXX clang-cl
//! ```
//! Note that you may need to reopen the terminal for the environment variables to be available (setx always requires this).
//!
//! # Pre-built
//!
//! There is a pre-built release of this crate, [`zero-ui-view-prebuilt`], it works as a drop-in replacement
// that dynamically links with a pre-built library, for Windows, Linux and MacOS.
//!
//! In the `Cargo.toml` file:
//!
//! ```toml
//! zero-ui-view-prebuilt = "0.1"
//! ```
//!
//! Then in the `main.rs` file:
//!
//! ```no_run
//! # mod zero_ui_view_prebuilt { pub fn init() { } }
//! use zero_ui_view_prebuilt as zero_ui_view;
//!
//! fn main() {
//!     zero_ui_view::init();
//!     
//!     // App::default().run ..
//! }
//! ```
//!
//! The pre-built crate includes the `"software"` and `"ipc"` features, in fact `ipc` is required, even for running on the same process,
//! you can also configure where the pre-build library is installed, see the [`zero-ui-view-prebuilt`] documentation for details.
//!
//! The pre-build crate does not support [`extensions`].
//!
//! # API Extensions
//!
//! This implementation of the view API provides one extension:
//!
//! * `"zero-ui-view.webrender_debug"`: `{ flags: DebugFlags, profiler_ui: String }`, sets Webrender debug flags.
//!
//! You can also inject your own extensions, see the [`extensions`] module for more details.
//!
//! [`glutin`]: https://docs.rs/glutin/
//! [`zero-ui-view-prebuilt`]: https://docs.rs/zero-ui-view-prebuilt/

use std::{
    fmt, mem, thread,
    time::{Duration, Instant},
};

use extensions::ViewExtensions;
use gl::GlContextManager;
use image_cache::ImageCache;
use util::WinitToPx;
use winit::{
    event::{DeviceEvent, ModifiersState, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy, EventLoopWindowTarget},
    monitor::MonitorHandle,
    platform::run_return::EventLoopExtRunReturn,
};

mod config;
mod gl;
mod image_cache;
mod surface;
mod util;
mod window;
use surface::*;

pub mod extensions;

/// Webrender build used in the view-process.
#[doc(inline)]
pub use webrender;

/// OpenGL bindings used by Webrender.
#[doc(inline)]
pub use gleam;

use webrender::api::*;
use window::Window;
use zero_ui_view_api::{units::*, *};

use rustc_hash::FxHashMap;

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
/// # pub fn default() -> Self { unimplemented!() }
/// # pub fn run_window(self, f: impl FnOnce(bool)) { }
/// # } } }
/// use zero_ui::prelude::*;
///
/// fn main() {
///     zero_ui_view::init();
///
///     App::default().run_window(|ctx| {
///         unimplemented!()
///     })
/// }
/// ```
///
/// # Panics
///
/// Panics if not called in the main thread, this is a requirement of some operating systems.
///
/// If there was an error connecting with the app-process.
///
/// # Aborts
///
/// If called in a view-process a custom panic hook is set that logs panics to `stderr` and exits the process with the
/// default panic exit code `101`. This is done because `webrender` can freeze due to panics in worker threads without propagating
/// the panics to the main thread, this causes the app to stop responding while still receiving
/// event signals, causing the operating system to not detect that the app is frozen.
#[cfg(feature = "ipc")]
pub fn init() {
    init_extended(extensions::ViewExtensions::new)
}

/// Like [`init`] but with custom API extensions.
#[cfg(feature = "ipc")]
pub fn init_extended(ext: fn() -> ViewExtensions) {
    if !is_main_thread::is_main_thread().unwrap_or(true) {
        panic!("only call `init` in the main thread, this is a requirement of some operating systems");
    }

    if let Some(config) = ViewConfig::from_env() {
        std::panic::set_hook(Box::new(init_abort));

        config.assert_version(false);

        let c = connect_view_process(config.server_name).expect("failed to connect to app-process");

        if config.headless {
            App::run_headless(c, ext());
        } else {
            App::run_headed(c, ext());
        }
    } else {
        tracing::trace!("init not in view-process");
    }
}

#[cfg(feature = "ipc")]
#[doc(hidden)]
#[no_mangle]
pub extern "C" fn extern_init() {
    std::panic::set_hook(Box::new(ffi_abort));
    init()
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
/// # pub fn default() -> Self { unimplemented!() }
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
///         unimplemented!()
///     })
/// }
/// ```
///
/// # Panics
///
/// Panics if not called in the main thread, this is a requirement of some operating systems.
///
/// ## Background Panics Warning
///
/// Note that `webrender` can freeze due to panics in worker threads without propagating
/// the panics to the main thread, this causes the app to stop responding while still receiving
/// event signals, causing the operating system to not detect that the app is frozen. It is **strongly recommended**
/// that you build with `panic=abort` or use [`std::panic::set_hook`] to detect these background panics.
pub fn run_same_process(run_app: impl FnOnce() + Send + 'static) {
    run_same_process_extended(run_app, ViewExtensions::new)
}

/// Like [`run_same_process`] but with custom API extensions.
pub fn run_same_process_extended(run_app: impl FnOnce() + Send + 'static, ext: fn() -> ViewExtensions) {
    if !is_main_thread::is_main_thread().unwrap_or(true) {
        panic!("only call `run_same_process` in the main thread, this is a requirement of some operating systems");
    }

    thread::Builder::new().name("app".to_owned()).spawn(run_app).unwrap();

    let config = ViewConfig::wait_same_process();
    config.assert_version(true);

    let c = connect_view_process(config.server_name).expect("failed to connect to app in same process");

    if config.headless {
        App::run_headless(c, ext());
    } else {
        App::run_headed(c, ext());
    }
}

#[cfg(feature = "ipc")]
#[doc(hidden)]
#[no_mangle]
pub extern "C" fn extern_run_same_process(run_app: extern "C" fn()) {
    std::panic::set_hook(Box::new(ffi_abort));

    #[allow(clippy::redundant_closure)]
    run_same_process(move || run_app())
}

fn init_abort(info: &std::panic::PanicInfo) {
    panic_hook(info, "note: aborting to respawn");
}
fn ffi_abort(info: &std::panic::PanicInfo) {
    panic_hook(info, "note: aborting to avoid unwind across FFI");
}
fn panic_hook(info: &std::panic::PanicInfo, details: &str) {
    if crate::util::suppress_panic() {
        return;
    }

    // see `default_hook` in https://doc.rust-lang.org/src/std/panicking.rs.html#182

    let current_thread = std::thread::current();
    let name = current_thread.name().unwrap_or("<unnamed>");

    let (file, line, column) = if let Some(l) = info.location() {
        (l.file(), l.line(), l.column())
    } else {
        ("<unknown>", 0, 0)
    };

    let msg = util::panic_msg(info.payload());

    let backtrace = backtrace::Backtrace::new();

    eprintln!("thread '{name}' panicked at '{msg}', {file}:{line}:{column}\n {details}\n{backtrace:?}");
    std::process::exit(101) // Rust panic exit code.
}

/// The backend implementation.
pub(crate) struct App {
    started: bool,

    headless: bool,

    exts: ViewExtensions,

    gl_manager: GlContextManager,
    window_target: *const EventLoopWindowTarget<AppEvent>,
    app_sender: AppEventSender,
    request_recv: flume::Receiver<RequestEvent>,

    response_sender: ResponseSender,
    event_sender: EventSender,
    image_cache: ImageCache,

    gen: ViewProcessGen,
    device_events: bool,

    windows: Vec<Window>,
    surfaces: Vec<Surface>,

    monitor_id_gen: MonitorId,
    pub monitors: Vec<(MonitorId, MonitorHandle)>,

    device_id_gen: DeviceId,
    devices: Vec<(DeviceId, winit::event::DeviceId)>,

    dialog_id_gen: DialogId,

    resize_frame_wait_id_gen: FrameWaitId,

    coalescing_event: Option<Event>,
    // winit only sends a CursorMove after CursorEntered if the cursor is in a different position,
    // but this makes refreshing hit-tests weird, do we hit-test the previous known point at each CursorEnter?
    //
    // This flag causes a MouseMove at the same previous position if no mouse move was send after CursorEnter and before
    // MainEventsCleared.
    cursor_entered_expect_move: Vec<WindowId>,

    #[cfg(windows)]
    skip_ralt: bool,

    pressed_modifiers: FxHashMap<Key, (DeviceId, KeyCode)>,
    pending_modifiers_update: Option<ModifiersState>,
    pending_modifiers_focus_clear: bool,

    #[cfg(not(windows))]
    arboard: Option<arboard::Clipboard>,

    exited: bool,
}
impl fmt::Debug for App {
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
impl App {
    fn disable_device_events(&mut self, t: Option<&EventLoopWindowTarget<AppEvent>>) {
        self.device_events = false;

        if let Some(t) = t {
            t.set_device_event_filter(winit::event_loop::DeviceEventFilter::Always);
        }

        #[cfg(windows)]
        util::unregister_raw_input();
    }

    pub fn run_headless(c: ViewChannels, ext: ViewExtensions) {
        tracing::info!("running headless view-process");

        gl::warmup();

        let (app_sender, app_receiver) = flume::unbounded();
        let (request_sender, request_receiver) = flume::unbounded();
        let mut app = App::new(
            AppEventSender::Headless(app_sender, request_sender),
            c.response_sender,
            c.event_sender,
            request_receiver,
            ext,
        );
        app.headless = true;

        let winit_span = tracing::trace_span!("winit::EventLoop::new").entered();
        let event_loop = EventLoopBuilder::<AppEvent>::with_user_event().build();

        drop(winit_span);

        let window_target: &EventLoopWindowTarget<AppEvent> = &event_loop;
        app.window_target = window_target as *const _;

        app.start_receiving(c.request_receiver);

        'app_loop: while !app.exited {
            match app_receiver.recv() {
                Ok(app_ev) => match app_ev {
                    AppEvent::Request => {
                        while let Ok(request) = app.request_recv.try_recv() {
                            match request {
                                RequestEvent::Request(request) => {
                                    let response = app.respond(request);
                                    if response.must_be_send() && app.response_sender.send(response).is_err() {
                                        app.exited = true;
                                        break 'app_loop;
                                    }
                                }
                                RequestEvent::FrameReady(id, msg) => {
                                    let r = if let Some(s) = app.surfaces.iter_mut().find(|s| s.id() == id) {
                                        Some(s.on_frame_ready(msg, &mut app.image_cache))
                                    } else {
                                        None
                                    };
                                    if let Some((frame_id, image)) = r {
                                        app.notify(Event::FrameRendered(EventFrameRendered {
                                            window: id,
                                            frame: frame_id,
                                            frame_image: image,
                                        }));
                                    }
                                }
                            }
                        }
                    }
                    AppEvent::Notify(ev) => {
                        if app.event_sender.send(ev).is_err() {
                            app.exited = true;
                            break 'app_loop;
                        }
                    }
                    AppEvent::RefreshMonitors => {
                        panic!("no monitor info in headless mode")
                    }
                    AppEvent::WinitFocused(_, _) => {
                        panic!("no winit event loop in headless mode")
                    }
                    AppEvent::ParentProcessExited => {
                        app.exited = true;
                        break 'app_loop;
                    }
                    AppEvent::ImageLoaded(data) => {
                        app.image_cache.loaded(data);
                    }
                    AppEvent::MonitorPowerChanged => {} // headless
                    AppEvent::DisableDeviceEvents => {
                        app.disable_device_events(None);
                    }
                },
                Err(_) => {
                    app.exited = true;
                    break;
                }
            }
        }
    }

    pub fn run_headed(c: ViewChannels, ext: ViewExtensions) {
        tracing::info!("running headed view-process");

        gl::warmup();

        let winit_span = tracing::trace_span!("winit::EventLoop::new").entered();
        let mut event_loop = EventLoopBuilder::with_user_event().build();
        drop(winit_span);
        let app_sender = event_loop.create_proxy();

        let (request_sender, request_receiver) = flume::unbounded();
        let mut app = App::new(
            AppEventSender::Headed(app_sender, request_sender),
            c.response_sender,
            c.event_sender,
            request_receiver,
            ext,
        );
        app.start_receiving(c.request_receiver);

        #[cfg(windows)]
        config::spawn_listener(app.app_sender.clone());

        struct IdleTrace(Option<tracing::span::EnteredSpan>);
        impl IdleTrace {
            pub fn enter(&mut self) {
                self.0 = Some(tracing::trace_span!("<winit-idle>").entered());
            }
            pub fn exit(&mut self) {
                self.0 = None;
            }
        }
        let mut idle = IdleTrace(None);
        idle.enter();

        event_loop.run_return(move |event, target, flow| {
            idle.exit();

            app.window_target = target;

            *flow = ControlFlow::Wait;

            if app.exited {
                *flow = ControlFlow::Exit;
            } else {
                use winit::event::Event as WEvent;
                match event {
                    WEvent::NewEvents(_) => {}
                    WEvent::WindowEvent { window_id, event } => app.on_window_event(window_id, event),
                    WEvent::DeviceEvent { device_id, event } => app.on_device_event(device_id, event),
                    WEvent::UserEvent(ev) => match ev {
                        AppEvent::Request => {
                            while let Ok(req) = app.request_recv.try_recv() {
                                match req {
                                    RequestEvent::Request(req) => {
                                        let rsp = app.respond(req);
                                        if rsp.must_be_send() && app.response_sender.send(rsp).is_err() {
                                            // lost connection to app-process
                                            app.exited = true;
                                            *flow = ControlFlow::Exit;
                                        }
                                    }
                                    RequestEvent::FrameReady(wid, msg) => app.on_frame_ready(wid, msg),
                                }
                            }
                        }
                        AppEvent::Notify(ev) => app.notify(ev),
                        AppEvent::WinitFocused(window_id, focused) => app.on_window_event(window_id, WindowEvent::Focused(focused)),
                        AppEvent::RefreshMonitors => app.refresh_monitors(),
                        AppEvent::ParentProcessExited => {
                            app.exited = true;
                            *flow = ControlFlow::Exit;
                        }
                        AppEvent::ImageLoaded(data) => {
                            app.image_cache.loaded(data);
                        }
                        AppEvent::MonitorPowerChanged => {
                            // if a window opens in power-off it is blank until redraw.
                            for w in &mut app.windows {
                                w.redraw();
                            }
                        }
                        AppEvent::DisableDeviceEvents => {
                            app.disable_device_events(Some(target));
                        }
                    },
                    WEvent::Suspended => {}
                    WEvent::Resumed => {}
                    WEvent::MainEventsCleared => {
                        app.finish_cursor_entered_move();
                        app.update_modifiers();
                        app.flush_coalesced();
                        #[cfg(windows)]
                        {
                            app.skip_ralt = false;
                        }
                    }
                    WEvent::RedrawRequested(w_id) => app.on_redraw(w_id),
                    WEvent::RedrawEventsCleared => {}
                    WEvent::LoopDestroyed => {}
                }
            }

            app.window_target = std::ptr::null();

            idle.enter();
        });
    }

    fn new(
        app_sender: AppEventSender,
        response_sender: ResponseSender,
        event_sender: EventSender,
        request_recv: flume::Receiver<RequestEvent>,
        mut ext: ViewExtensions,
    ) -> Self {
        ext.renderer("zero-ui-view.webrender_debug", extensions::RendererDebugExt::new);
        ext.init(&app_sender);
        App {
            headless: false,
            started: false,
            exts: ext,
            gl_manager: GlContextManager::default(),
            image_cache: ImageCache::new(app_sender.clone()),
            app_sender,
            request_recv,
            response_sender,
            event_sender,
            window_target: std::ptr::null(),
            gen: ViewProcessGen::INVALID,
            device_events: false,
            windows: vec![],
            surfaces: vec![],
            monitors: vec![],
            monitor_id_gen: MonitorId::INVALID,
            devices: vec![],
            device_id_gen: DeviceId::INVALID,
            dialog_id_gen: DialogId::INVALID,
            resize_frame_wait_id_gen: FrameWaitId::INVALID,
            coalescing_event: None,
            cursor_entered_expect_move: Vec::with_capacity(1),
            exited: false,
            #[cfg(windows)]
            skip_ralt: false,
            pressed_modifiers: FxHashMap::default(),
            pending_modifiers_update: None,
            pending_modifiers_focus_clear: false,

            #[cfg(not(windows))]
            arboard: None,
        }
    }

    fn start_receiving(&mut self, mut request_recv: RequestReceiver) {
        let app_sender = self.app_sender.clone();
        thread::spawn(move || {
            while let Ok(r) = request_recv.recv() {
                if let Err(Disconnected) = app_sender.request(r) {
                    break;
                }
            }
        });
    }

    fn on_window_event(&mut self, window_id: winit::window::WindowId, event: WindowEvent) {
        let i = if let Some((i, _)) = self.windows.iter_mut().enumerate().find(|(_, w)| w.window_id() == window_id) {
            i
        } else {
            return;
        };

        let _s = tracing::trace_span!("on_window_event", ?event).entered();

        let id = self.windows[i].id();
        let scale_factor = self.windows[i].scale_factor();

        #[cfg(any(
            target_os = "linux",
            target_os = "dragonfly",
            target_os = "freebsd",
            target_os = "netbsd",
            target_os = "openbsd"
        ))]
        let modal_dialog_active = self.windows[i].modal_dialog_active();
        #[cfg(any(
            target_os = "linux",
            target_os = "dragonfly",
            target_os = "freebsd",
            target_os = "netbsd",
            target_os = "openbsd"
        ))]
        macro_rules! linux_modal_dialog_bail {
            () => {
                if modal_dialog_active {
                    return;
                }
            };
        }
        #[cfg(not(any(
            target_os = "linux",
            target_os = "dragonfly",
            target_os = "freebsd",
            target_os = "netbsd",
            target_os = "openbsd"
        )))]
        macro_rules! linux_modal_dialog_bail {
            () => {};
        }

        match event {
            WindowEvent::Resized(_) => {
                let size = if let Some(size) = self.windows[i].resized() {
                    size
                } else {
                    return;
                };

                // give the app 300ms to send a new frame, this is the collaborative way to
                // resize, it should reduce the changes of the user seeing the clear color.

                let deadline = Instant::now() + Duration::from_millis(300);

                // await already pending frames.
                if self.windows[i].is_rendering_frame() {
                    tracing::debug!("resize requested while still rendering");

                    // forward requests until webrender finishes or timeout.
                    while let Ok(req) = self.request_recv.recv_deadline(deadline) {
                        match req {
                            RequestEvent::Request(req) => {
                                let rsp = self.respond(req);
                                if rsp.must_be_send() {
                                    let _ = self.response_sender.send(rsp);
                                }
                            }
                            RequestEvent::FrameReady(id, msg) => {
                                self.on_frame_ready(id, msg);
                                if id == self.windows[i].id() {
                                    break;
                                }
                            }
                        }
                    }
                }

                if let Some(state) = self.windows[i].state_change() {
                    self.notify(Event::WindowChanged(WindowChanged::state_changed(id, state, EventCause::System)));
                }

                if let Some(handle) = self.windows[i].monitor_change() {
                    let m_id = self.monitor_handle_to_id(&handle);

                    self.notify(Event::WindowChanged(WindowChanged::monitor_changed(
                        id,
                        m_id,
                        self.windows[i].scale_factor(),
                        EventCause::System,
                    )));
                }

                let wait_id = Some(self.resize_frame_wait_id_gen.incr());

                // send event, the app code should send a frame in the new size as soon as possible.
                self.notify(Event::WindowChanged(WindowChanged::resized(id, size, EventCause::System, wait_id)));

                self.flush_coalesced();

                // "modal" loop, breaks in 300ms or when a frame is received.
                let mut received_frame = false;
                loop {
                    match self.request_recv.recv_deadline(deadline) {
                        Ok(req) => {
                            match req {
                                RequestEvent::Request(req) => {
                                    received_frame = req.is_frame(id, wait_id);
                                    if received_frame || req.affects_window_rect(id) {
                                        // received new frame
                                        let rsp = self.respond(req);
                                        if rsp.must_be_send() {
                                            let _ = self.response_sender.send(rsp);
                                        }
                                        break;
                                    } else {
                                        // received some other request, forward it.
                                        let rsp = self.respond(req);
                                        if rsp.must_be_send() {
                                            let _ = self.response_sender.send(rsp);
                                        }
                                    }
                                }
                                RequestEvent::FrameReady(id, msg) => self.on_frame_ready(id, msg),
                            }
                        }

                        Err(flume::RecvTimeoutError::Timeout) => {
                            // did not receive a new frame in time.
                            break;
                        }
                        Err(flume::RecvTimeoutError::Disconnected) => {
                            unreachable!()
                        }
                    }
                }

                // if we are still within 300ms, await webrender.
                if received_frame && deadline > Instant::now() {
                    // forward requests until webrender finishes or timeout.
                    while let Ok(req) = self.request_recv.recv_deadline(deadline) {
                        match req {
                            RequestEvent::Request(req) => {
                                let rsp = self.respond(req);
                                if rsp.must_be_send() {
                                    let _ = self.response_sender.send(rsp);
                                }
                            }
                            RequestEvent::FrameReady(id, msg) => {
                                self.on_frame_ready(id, msg);
                                if id == self.windows[i].id() {
                                    break;
                                }
                            }
                        }
                    }
                }
            }
            WindowEvent::Moved(_) => {
                let p = if let Some(p) = self.windows[i].moved() {
                    p
                } else {
                    return;
                };

                if let Some(state) = self.windows[i].state_change() {
                    self.notify(Event::WindowChanged(WindowChanged::state_changed(id, state, EventCause::System)));
                }

                self.notify(Event::WindowChanged(WindowChanged::moved(id, p, EventCause::System)));

                if let Some(handle) = self.windows[i].monitor_change() {
                    let m_id = self.monitor_handle_to_id(&handle);

                    self.notify(Event::WindowChanged(WindowChanged::monitor_changed(
                        id,
                        m_id,
                        self.windows[i].scale_factor(),
                        EventCause::System,
                    )));
                }
            }
            WindowEvent::CloseRequested => {
                linux_modal_dialog_bail!();
                self.notify(Event::WindowCloseRequested(id))
            }
            WindowEvent::Destroyed => {
                self.windows.remove(i);
                self.notify(Event::WindowClosed(id));
            }
            WindowEvent::DroppedFile(file) => {
                linux_modal_dialog_bail!();
                self.notify(Event::DroppedFile { window: id, file })
            }
            WindowEvent::HoveredFile(file) => {
                linux_modal_dialog_bail!();
                self.notify(Event::HoveredFile { window: id, file })
            }
            WindowEvent::HoveredFileCancelled => {
                linux_modal_dialog_bail!();
                self.notify(Event::HoveredFileCancelled(id))
            }
            WindowEvent::Focused(mut focused) => {
                if self.windows[i].focused_changed(&mut focused) {
                    if focused {
                        self.notify(Event::FocusChanged { prev: None, new: Some(id) });
                    } else {
                        self.pending_modifiers_focus_clear = true;
                        self.notify(Event::FocusChanged { prev: Some(id), new: None });
                    }
                }
            }
            WindowEvent::KeyboardInput {
                device_id,
                input,
                is_synthetic,
            } => {
                linux_modal_dialog_bail!();

                if !is_synthetic && self.windows[i].is_focused() {
                    #[cfg(windows)]
                    if self.skip_ralt {
                        // see the Window::focus comments.
                        if let Some(winit::event::VirtualKeyCode::RAlt) = input.virtual_keycode {
                            return;
                        }
                    }

                    let state = util::element_state_to_key_state(input.state);
                    let key = input.virtual_keycode.map(util::v_key_to_key);
                    let d_id = self.device_id(device_id);

                    let mut send_event = true;

                    if let Some(key) = key.clone() {
                        if key.is_modifier() {
                            match state {
                                KeyState::Pressed => {
                                    send_event = self
                                        .pressed_modifiers
                                        .insert(key, (d_id, util::scan_code_to_key(input.scancode)))
                                        .is_none();
                                }
                                KeyState::Released => send_event = self.pressed_modifiers.remove(&key).is_some(),
                            }
                        }
                    }

                    if send_event {
                        self.notify(Event::KeyboardInput {
                            window: id,
                            device: d_id,
                            key_code: util::scan_code_to_key(input.scancode),
                            state,
                            key: key.clone(),
                            key_modified: key.clone(),
                            text: String::new(),
                        });
                    }
                }
            }
            WindowEvent::ReceivedCharacter(c) => {
                linux_modal_dialog_bail!();
                // merged with previous key press.
                self.notify(Event::KeyboardInput {
                    window: id,
                    device: DeviceId::INVALID,
                    key_code: KeyCode::Unidentified(NativeKeyCode::Unidentified),
                    state: KeyState::Pressed,
                    key: None,
                    key_modified: None,
                    text: c.to_string(),
                })
            }
            WindowEvent::ModifiersChanged(m) => {
                linux_modal_dialog_bail!();
                if self.windows[i].is_focused() {
                    self.pending_modifiers_update = Some(m);
                }
            }
            WindowEvent::CursorMoved { device_id, position, .. } => {
                linux_modal_dialog_bail!();

                let px_p = position.to_px();
                let p = px_p.to_dip(scale_factor);
                let d_id = self.device_id(device_id);

                let mut is_after_cursor_enter = false;
                if let Some(i) = self.cursor_entered_expect_move.iter().position(|&w| w == id) {
                    self.cursor_entered_expect_move.remove(i);
                    is_after_cursor_enter = true;
                }

                if self.windows[i].cursor_moved(p, d_id) || is_after_cursor_enter {
                    self.notify(Event::CursorMoved {
                        window: id,
                        device: d_id,
                        coalesced_pos: vec![],
                        position: p,
                    });
                }
            }
            WindowEvent::CursorEntered { device_id } => {
                linux_modal_dialog_bail!();
                if self.windows[i].cursor_entered() {
                    let d_id = self.device_id(device_id);
                    self.notify(Event::CursorEntered { window: id, device: d_id });
                    self.cursor_entered_expect_move.push(id);
                }
            }
            WindowEvent::CursorLeft { device_id } => {
                linux_modal_dialog_bail!();
                if self.windows[i].cursor_left() {
                    let d_id = self.device_id(device_id);
                    self.notify(Event::CursorLeft { window: id, device: d_id });

                    // unlikely but possible?
                    if let Some(i) = self.cursor_entered_expect_move.iter().position(|&w| w == id) {
                        self.cursor_entered_expect_move.remove(i);
                    }
                }
            }
            WindowEvent::MouseWheel {
                device_id, delta, phase, ..
            } => {
                linux_modal_dialog_bail!();
                let d_id = self.device_id(device_id);
                self.notify(Event::MouseWheel {
                    window: id,
                    device: d_id,
                    delta: util::winit_mouse_wheel_delta_to_zui(delta),
                    phase: util::winit_touch_phase_to_zui(phase),
                });
            }
            WindowEvent::MouseInput {
                device_id, state, button, ..
            } => {
                linux_modal_dialog_bail!();
                let d_id = self.device_id(device_id);
                self.notify(Event::MouseInput {
                    window: id,
                    device: d_id,
                    state: util::element_state_to_button_state(state),
                    button: util::winit_mouse_button_to_zui(button),
                });
            }
            WindowEvent::TouchpadPressure {
                device_id,
                pressure,
                stage,
            } => {
                linux_modal_dialog_bail!();
                let d_id = self.device_id(device_id);
                self.notify(Event::TouchpadPressure {
                    window: id,
                    device: d_id,
                    pressure,
                    stage,
                });
            }
            WindowEvent::AxisMotion { device_id, axis, value } => {
                linux_modal_dialog_bail!();
                let d_id = self.device_id(device_id);
                self.notify(Event::AxisMotion(id, d_id, AxisId(axis), value));
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
            WindowEvent::TouchpadMagnify { .. } => {
                linux_modal_dialog_bail!();
                // TODO
            }
            WindowEvent::TouchpadRotate { .. } => {
                linux_modal_dialog_bail!();
                // TODO
            }
            WindowEvent::SmartMagnify { .. } => {
                linux_modal_dialog_bail!();
                // TODO
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                let monitor;
                let mut is_monitor_change = false;

                if let Some(new_monitor) = self.windows[i].monitor_change() {
                    monitor = Some(new_monitor);
                    is_monitor_change = true;
                } else {
                    monitor = self.windows[i].monitor();
                }

                let monitor = if let Some(handle) = monitor {
                    self.monitor_handle_to_id(&handle)
                } else {
                    MonitorId::INVALID
                };

                if is_monitor_change {
                    self.notify(Event::WindowChanged(WindowChanged::monitor_changed(
                        id,
                        monitor,
                        scale_factor as f32,
                        EventCause::System,
                    )));
                } else {
                    self.notify(Event::ScaleFactorChanged {
                        monitor,
                        windows: vec![id],
                        scale_factor: scale_factor as f32,
                    });
                }
            }
            WindowEvent::ThemeChanged(t) => self.notify(Event::ColorSchemeChanged(id, util::winit_theme_to_zui(t))),
            WindowEvent::Ime(_) => {
                linux_modal_dialog_bail!();
                // TODO
            }
            WindowEvent::Occluded(_) => {}
        }
    }

    fn monitor_handle_to_id(&mut self, handle: &MonitorHandle) -> MonitorId {
        if let Some((id, _)) = self.monitors.iter().find(|(_, h)| h == handle) {
            *id
        } else {
            self.refresh_monitors();
            if let Some((id, _)) = self.monitors.iter().find(|(_, h)| h == handle) {
                *id
            } else {
                MonitorId::INVALID
            }
        }
    }

    fn update_modifiers(&mut self) {
        // Winit monitors the modifiers keys directly, so this generates events
        // that are not send to the window by the operating system.
        //
        // An Example:
        // In Windows +LShift +RShift -LShift -RShift only generates +LShift +RShift -RShift, notice the missing -LShift.

        if mem::take(&mut self.pending_modifiers_focus_clear) && self.windows.iter().all(|w| !w.is_focused()) {
            self.pressed_modifiers.clear();
        }

        if let Some(m) = self.pending_modifiers_update.take() {
            if let Some(id) = self.windows.iter().find(|w| w.is_focused()).map(|w| w.id()) {
                let mut notify = vec![];
                self.pressed_modifiers.retain(|key, (d_id, s_code)| {
                    let mut retain = true;
                    if matches!(key, Key::Super) && !m.logo() {
                        retain = false;
                        notify.push(Event::KeyboardInput {
                            window: id,
                            device: *d_id,
                            key_code: *s_code,
                            state: KeyState::Released,
                            key: Some(key.clone()),
                            key_modified: Some(key.clone()),
                            text: String::new(),
                        });
                    }
                    if matches!(key, Key::Shift) && !m.shift() {
                        retain = false;
                        notify.push(Event::KeyboardInput {
                            window: id,
                            device: *d_id,
                            key_code: *s_code,
                            state: KeyState::Released,
                            key: Some(key.clone()),
                            key_modified: Some(key.clone()),
                            text: String::new(),
                        });
                    }
                    if matches!(key, Key::Alt | Key::AltGraph) && !m.alt() {
                        retain = false;
                        notify.push(Event::KeyboardInput {
                            window: id,
                            device: *d_id,
                            key_code: *s_code,
                            state: KeyState::Released,
                            key: Some(key.clone()),
                            key_modified: Some(key.clone()),
                            text: String::new(),
                        });
                    }
                    if matches!(key, Key::Ctrl) && !m.ctrl() {
                        retain = false;
                        notify.push(Event::KeyboardInput {
                            window: id,
                            device: *d_id,
                            key_code: *s_code,
                            state: KeyState::Released,
                            key: Some(key.clone()),
                            key_modified: Some(key.clone()),
                            text: String::new(),
                        });
                    }
                    retain
                });

                for ev in notify {
                    self.notify(ev);
                }
            }
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
                .unwrap_or_else(|| self.monitor_id_gen.incr());
            monitors.push((id, handle))
        }

        if added_check || removed_check > 1 {
            self.monitors = monitors;

            let monitors = self.available_monitors();
            self.notify(Event::MonitorsChanged(monitors));
        }
    }

    fn on_frame_ready(&mut self, window_id: WindowId, msg: FrameReadyMsg) {
        let _s = tracing::trace_span!("on_frame_ready").entered();

        if let Some(w) = self.windows.iter_mut().find(|w| w.id() == window_id) {
            let r = w.on_frame_ready(msg, &mut self.image_cache);

            let _ = self.event_sender.send(Event::FrameRendered(EventFrameRendered {
                window: window_id,
                frame: r.frame_id,
                frame_image: r.image,
            }));

            if r.first_frame {
                let size = w.size();
                self.notify(Event::WindowChanged(WindowChanged::resized(window_id, size, EventCause::App, None)));
            }
        } else if let Some(s) = self.surfaces.iter_mut().find(|w| w.id() == window_id) {
            let (frame_id, image) = s.on_frame_ready(msg, &mut self.image_cache);

            self.notify(Event::FrameRendered(EventFrameRendered {
                window: window_id,
                frame: frame_id,
                frame_image: image,
            }))
        }
    }

    pub(crate) fn notify(&mut self, event: Event) {
        if let Some(mut coal) = self.coalescing_event.take() {
            match coal.coalesce(event) {
                Ok(()) => self.coalescing_event = Some(coal),
                Err(event) => match (&mut coal, event) {
                    (
                        Event::KeyboardInput {
                            window,
                            device,
                            state,
                            text,
                            ..
                        },
                        Event::KeyboardInput {
                            window: n_window,
                            device: n_device,
                            text: n_text,
                            ..
                        },
                    ) if !n_text.is_empty() && *window == n_window && *device == n_device && *state == KeyState::Pressed => {
                        // text after key-press
                        if text.is_empty() {
                            *text = n_text;
                        } else {
                            text.push_str(&n_text);
                        };
                        self.coalescing_event = Some(coal);
                    }
                    (_, event) => {
                        let mut error = self.event_sender.send(coal).is_err();
                        error |= self.event_sender.send(event).is_err();

                        if error {
                            let _ = self.app_sender.send(AppEvent::ParentProcessExited);
                        }
                    }
                },
            }
        } else {
            self.coalescing_event = Some(event);
        }

        if self.headless {
            self.flush_coalesced();
        }
    }

    pub(crate) fn finish_cursor_entered_move(&mut self) {
        let mut moves = vec![];
        for window_id in self.cursor_entered_expect_move.drain(..) {
            if let Some(w) = self.windows.iter().find(|w| w.id() == window_id) {
                let (position, device) = w.last_cursor_pos();
                moves.push(Event::CursorMoved {
                    window: w.id(),
                    device,
                    coalesced_pos: vec![],
                    position,
                });
            }
        }
        for ev in moves {
            self.notify(ev);
        }
    }

    /// Send pending coalesced events.
    pub(crate) fn flush_coalesced(&mut self) {
        if let Some(coal) = self.coalescing_event.take() {
            if self.event_sender.send(coal).is_err() {
                let _ = self.app_sender.send(AppEvent::ParentProcessExited);
            }
        }
    }

    fn on_device_event(&mut self, device_id: winit::event::DeviceId, event: DeviceEvent) {
        if self.device_events {
            let _s = tracing::trace_span!("on_device_event", ?event);

            let d_id = self.device_id(device_id);
            match event {
                DeviceEvent::Added => self.notify(Event::DeviceAdded(d_id)),
                DeviceEvent::Removed => self.notify(Event::DeviceRemoved(d_id)),
                DeviceEvent::MouseMotion { delta } => self.notify(Event::DeviceMouseMotion {
                    device: d_id,
                    delta: euclid::vec2(delta.0, delta.1),
                }),
                DeviceEvent::MouseWheel { delta } => self.notify(Event::DeviceMouseWheel {
                    device: d_id,
                    delta: util::winit_mouse_wheel_delta_to_zui(delta),
                }),
                DeviceEvent::Motion { axis, value } => self.notify(Event::DeviceMotion {
                    device: d_id,
                    axis: AxisId(axis),
                    value,
                }),
                DeviceEvent::Button { button, state } => self.notify(Event::DeviceButton {
                    device: d_id,
                    button: ButtonId(button),
                    state: util::element_state_to_button_state(state),
                }),
                DeviceEvent::Key(k) => self.notify(Event::DeviceKey {
                    device: d_id,
                    key_code: util::scan_code_to_key(k.scancode),
                    state: util::element_state_to_key_state(k.state),
                }),
                DeviceEvent::Text { .. } => {}
            }
        }
    }

    fn on_redraw(&mut self, window_id: winit::window::WindowId) {
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
            tracing::error!("headed window `{id:?}` not found, will return fallback result");
            not_found()
        })
    }

    fn monitor_id(&mut self, handle: &MonitorHandle) -> MonitorId {
        if let Some((id, _)) = self.monitors.iter().find(|(_, h)| h == handle) {
            *id
        } else {
            let id = self.monitor_id_gen.incr();
            self.monitors.push((id, handle.clone()));
            id
        }
    }

    fn device_id(&mut self, device_id: winit::event::DeviceId) -> DeviceId {
        if let Some((id, _)) = self.devices.iter().find(|(_, id)| *id == device_id) {
            *id
        } else {
            let id = self.device_id_gen.incr();
            self.devices.push((id, device_id));
            id
        }
    }

    fn available_monitors(&mut self) -> Vec<(MonitorId, MonitorInfo)> {
        let _span = tracing::trace_span!("available_monitors").entered();

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
}
macro_rules! with_window_or_surface {
    ($self:ident, $id:ident, |$el:ident|$action:expr, ||$fallback:expr) => {
        if let Some($el) = $self.windows.iter_mut().find(|w| w.id() == $id) {
            $action
        } else if let Some($el) = $self.surfaces.iter_mut().find(|w| w.id() == $id) {
            $action
        } else {
            tracing::error!("window `{:?}` not found, will return fallback result", $id);
            $fallback
        }
    };
}

impl App {
    fn open_headless_impl(&mut self, config: HeadlessRequest) -> HeadlessOpenData {
        self.assert_started();
        let surf = Surface::open(
            self.gen,
            config,
            unsafe { &*self.window_target },
            &mut self.gl_manager,
            self.exts.new_renderer(),
            self.app_sender.clone(),
        );
        let id_namespace = surf.id_namespace();
        let pipeline_id = surf.pipeline_id();
        let render_mode = surf.render_mode();

        self.surfaces.push(surf);

        HeadlessOpenData {
            id_namespace,
            pipeline_id,
            render_mode,
        }
    }

    #[cfg(not(windows))]
    fn arboard(&mut self) -> Result<&mut arboard::Clipboard, ClipboardError> {
        if self.arboard.is_none() {
            match arboard::Clipboard::new() {
                Ok(c) => self.arboard = Some(c),
                Err(e) => return Err(util::arboard_to_clip(e)),
            }
        }
        Ok(self.arboard.as_mut().unwrap())
    }
}

impl Api for App {
    fn init(&mut self, gen: ViewProcessGen, is_respawn: bool, device_events: bool, headless: bool) {
        if self.started {
            panic!("already started");
        }
        if self.exited {
            panic!("cannot restart exited");
        }
        self.started = true;
        self.gen = gen;
        self.device_events = device_events;
        self.headless = headless;

        if !device_events {
            self.app_sender.send(AppEvent::DisableDeviceEvents).unwrap();
        }

        let available_monitors = self.available_monitors();
        self.notify(Event::Inited {
            generation: gen,
            is_respawn,
            available_monitors,
            color_scheme: config::color_scheme_config(),
            multi_click_config: config::multi_click_config(),
            key_repeat_config: config::key_repeat_config(),
            font_aa: config::font_aa(),
            animations_config: config::animations_config(),
            locale_config: config::locale_config(),
            extensions: self.exts.api_extensions(),
        });
    }

    fn exit(&mut self) {
        self.assert_started();
        self.started = false;
        self.exited = true;
    }

    fn open_window(&mut self, mut config: WindowRequest) {
        let _s = tracing::debug_span!("open_window", ?config).entered();

        config.state.clamp_size();
        config.enforce_kiosk();

        if self.headless {
            let id = config.id;
            let data = self.open_headless_impl(HeadlessRequest {
                id: config.id,
                scale_factor: 1.0,
                size: config.state.restore_rect.size,
                render_mode: config.render_mode,
                extensions: config.extensions,
            });
            let msg = WindowOpenData {
                id_namespace: data.id_namespace,
                pipeline_id: data.pipeline_id,
                render_mode: data.render_mode,
                monitor: None,
                position: DipPoint::zero(),
                size: config.state.restore_rect.size,
                scale_factor: 1.0,
                color_scheme: ColorScheme::Light,
                state: WindowStateAll {
                    state: WindowState::Fullscreen,
                    restore_rect: DipRect::from_size(config.state.restore_rect.size),
                    restore_state: WindowState::Fullscreen,
                    min_size: DipSize::zero(),
                    max_size: DipSize::new(Dip::MAX, Dip::MAX),
                    chrome_visible: false,
                },
            };

            self.notify(Event::WindowOpened(id, msg));
        } else {
            self.assert_started();

            let id = config.id;

            let win = Window::open(
                self.gen,
                config.icon.and_then(|i| self.image_cache.get(i)).and_then(|i| i.icon()),
                config,
                unsafe { &*self.window_target },
                &mut self.gl_manager,
                self.exts.new_renderer(),
                self.app_sender.clone(),
            );

            let msg = WindowOpenData {
                id_namespace: win.id_namespace(),
                pipeline_id: win.pipeline_id(),
                monitor: win.monitor().map(|h| self.monitor_id(&h)),
                position: win.inner_position(),
                size: win.size(),
                scale_factor: win.scale_factor(),
                render_mode: win.render_mode(),
                state: win.state(),
                color_scheme: win.color_scheme(),
            };

            self.windows.push(win);

            self.notify(Event::WindowOpened(id, msg));
        }
    }

    fn open_headless(&mut self, config: HeadlessRequest) {
        let _s = tracing::debug_span!("open_headless", ?config).entered();

        let id = config.id;
        let msg = self.open_headless_impl(config);

        self.notify(Event::HeadlessOpened(id, msg));
    }

    fn close_window(&mut self, id: WindowId) {
        let _s = tracing::debug_span!("close_window", ?id);

        self.assert_started();
        if let Some(i) = self.windows.iter().position(|w| w.id() == id) {
            let _ = self.windows.swap_remove(i);
        }
        if let Some(i) = self.surfaces.iter().position(|w| w.id() == id) {
            let _ = self.surfaces.swap_remove(i);
        }
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

    fn bring_to_top(&mut self, id: WindowId) {
        self.with_window(id, |w| w.bring_to_top(), || ())
    }

    fn set_state(&mut self, id: WindowId, state: WindowStateAll) {
        if let Some(w) = self.windows.iter_mut().find(|w| w.id() == id) {
            if w.set_state(state.clone()) {
                let mut change = WindowChanged::state_changed(id, state, EventCause::App);

                change.size = w.resized();
                change.position = w.moved();
                if let Some(handle) = w.monitor_change() {
                    let scale_factor = w.scale_factor();
                    let monitor = self.monitor_handle_to_id(&handle);
                    change.monitor = Some((monitor, scale_factor));
                }

                let _ = self.app_sender.send(AppEvent::Notify(Event::WindowChanged(change)));
            }
        }
    }

    fn set_headless_size(&mut self, renderer: WindowId, size: DipSize, scale_factor: f32) {
        self.assert_started();
        if let Some(surf) = self.surfaces.iter_mut().find(|s| s.id() == renderer) {
            surf.set_size(size, scale_factor)
        }
    }

    fn set_video_mode(&mut self, id: WindowId, mode: VideoMode) {
        self.with_window(id, |w| w.set_video_mode(mode), || ())
    }

    fn set_icon(&mut self, id: WindowId, icon: Option<ImageId>) {
        let icon = icon.and_then(|i| self.image_cache.get(i)).and_then(|i| i.icon());
        self.with_window(id, |w| w.set_icon(icon), || ())
    }

    fn set_focus_indicator(&mut self, id: WindowId, request: Option<FocusIndicator>) {
        self.with_window(id, |w| w.set_focus_request(request), || ())
    }

    fn focus_window(&mut self, id: WindowId) {
        #[cfg(windows)]
        {
            self.skip_ralt = self.with_window(id, |w| w.focus(), || false);
        }

        #[cfg(not(windows))]
        {
            self.with_window(id, |w| w.focus(), || ());
        }
    }

    fn set_cursor(&mut self, id: WindowId, icon: Option<CursorIcon>) {
        self.with_window(id, |w| w.set_cursor(icon), || ())
    }

    fn image_decoders(&mut self) -> Vec<String> {
        image_cache::DECODERS.iter().map(|&s| s.to_owned()).collect()
    }

    fn image_encoders(&mut self) -> Vec<String> {
        image_cache::ENCODERS.iter().map(|&s| s.to_owned()).collect()
    }

    fn add_image(&mut self, request: ImageRequest<IpcBytes>) -> ImageId {
        self.image_cache.add(request)
    }

    fn add_image_pro(&mut self, request: ImageRequest<IpcBytesReceiver>) -> ImageId {
        self.image_cache.add_pro(request)
    }

    fn forget_image(&mut self, id: ImageId) {
        self.image_cache.forget(id)
    }

    fn encode_image(&mut self, id: ImageId, format: String) {
        self.image_cache.encode(id, format)
    }

    fn use_image(&mut self, id: WindowId, image_id: ImageId) -> ImageKey {
        if let Some(img) = self.image_cache.get(image_id) {
            with_window_or_surface!(self, id, |w| w.use_image(img), || ImageKey::DUMMY)
        } else {
            ImageKey::DUMMY
        }
    }

    fn update_image_use(&mut self, id: WindowId, key: ImageKey, image_id: ImageId) {
        if let Some(img) = self.image_cache.get(image_id) {
            with_window_or_surface!(self, id, |w| w.update_image(key, img), || ())
        }
    }

    fn delete_image_use(&mut self, id: WindowId, key: ImageKey) {
        with_window_or_surface!(self, id, |w| w.delete_image(key), || ())
    }

    fn add_font(&mut self, id: WindowId, bytes: IpcBytes, index: u32) -> FontKey {
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

    fn set_capture_mode(&mut self, id: WindowId, enabled: bool) {
        self.with_window(id, |w| w.set_capture_mode(enabled), || ())
    }

    fn frame_image(&mut self, id: WindowId, mask: Option<ImageMaskMode>) -> ImageId {
        with_window_or_surface!(self, id, |w| w.frame_image(&mut self.image_cache, mask), || ImageId::INVALID)
    }

    fn frame_image_rect(&mut self, id: WindowId, rect: PxRect, mask: Option<ImageMaskMode>) -> ImageId {
        with_window_or_surface!(self, id, |w| w.frame_image_rect(&mut self.image_cache, rect, mask), || {
            ImageId::INVALID
        })
    }

    fn render(&mut self, id: WindowId, frame: FrameRequest) {
        with_window_or_surface!(self, id, |w| w.render(frame), || ())
    }

    fn render_update(&mut self, id: WindowId, frame: FrameUpdateRequest) {
        with_window_or_surface!(self, id, |w| w.render_update(frame), || ())
    }

    fn message_dialog(&mut self, id: WindowId, dialog: MsgDialog) -> DialogId {
        let r_id = self.dialog_id_gen.incr();
        if let Some(s) = self.windows.iter_mut().find(|s| s.id() == id) {
            s.message_dialog(dialog, r_id, self.app_sender.clone());
        } else {
            let r = MsgDialogResponse::Error("window not found".to_owned());
            let _ = self.app_sender.send(AppEvent::Notify(Event::MsgDialogResponse(r_id, r)));
        }
        r_id
    }

    fn file_dialog(&mut self, id: WindowId, dialog: FileDialog) -> DialogId {
        let r_id = self.dialog_id_gen.incr();
        if let Some(s) = self.windows.iter_mut().find(|s| s.id() == id) {
            s.file_dialog(dialog, r_id, self.app_sender.clone());
        } else {
            let r = MsgDialogResponse::Error("window not found".to_owned());
            let _ = self.app_sender.send(AppEvent::Notify(Event::MsgDialogResponse(r_id, r)));
        };
        r_id
    }

    #[cfg(windows)]
    fn read_clipboard(&mut self, data_type: ClipboardType) -> Result<ClipboardData, ClipboardError> {
        match data_type {
            ClipboardType::Text => {
                let _clip = clipboard_win::Clipboard::new_attempts(10).map_err(util::clipboard_win_to_clip)?;

                clipboard_win::get(clipboard_win::formats::Unicode)
                    .map_err(util::clipboard_win_to_clip)
                    .map(ClipboardData::Text)
            }
            ClipboardType::Image => {
                let _clip = clipboard_win::Clipboard::new_attempts(10).map_err(util::clipboard_win_to_clip)?;

                let bitmap = clipboard_win::get(clipboard_win::formats::Bitmap).map_err(util::clipboard_win_to_clip)?;

                let id = self.image_cache.add(ImageRequest {
                    format: ImageDataFormat::FileExtension("bmp".to_owned()),
                    data: IpcBytes::from_vec(bitmap),
                    max_decoded_len: u64::MAX,
                    downscale: None,
                    mask: None,
                });
                Ok(ClipboardData::Image(id))
            }
            ClipboardType::FileList => {
                let _clip = clipboard_win::Clipboard::new_attempts(10).map_err(util::clipboard_win_to_clip)?;

                clipboard_win::get(clipboard_win::formats::FileList)
                    .map_err(util::clipboard_win_to_clip)
                    .map(ClipboardData::FileList)
            }
            ClipboardType::Extension(_) => Err(ClipboardError::NotSupported),
        }
    }

    #[cfg(windows)]
    fn write_clipboard(&mut self, data: ClipboardData) -> Result<(), ClipboardError> {
        match data {
            ClipboardData::Text(t) => {
                let _clip = clipboard_win::Clipboard::new_attempts(10).map_err(util::clipboard_win_to_clip)?;

                clipboard_win::set(clipboard_win::formats::Unicode, t).map_err(util::clipboard_win_to_clip)
            }
            ClipboardData::Image(id) => {
                let _clip = clipboard_win::Clipboard::new_attempts(10).map_err(util::clipboard_win_to_clip)?;

                if let Some(img) = self.image_cache.get(id) {
                    let mut bmp = vec![];
                    img.encode(image::ImageFormat::Bmp, &mut bmp)
                        .map_err(|e| ClipboardError::Other(format!("{e:?}")))?;
                    clipboard_win::set(clipboard_win::formats::Bitmap, bmp).map_err(util::clipboard_win_to_clip)
                } else {
                    Err(ClipboardError::Other("image not found".to_owned()))
                }
            }
            ClipboardData::FileList(l) => {
                use clipboard_win::Setter;
                let _clip = clipboard_win::Clipboard::new_attempts(10).map_err(util::clipboard_win_to_clip)?;

                // clipboard_win does not implement write from PathBuf
                let strs = l.into_iter().map(|p| p.display().to_string()).collect::<Vec<String>>();
                clipboard_win::formats::FileList
                    .write_clipboard(&strs)
                    .map_err(util::clipboard_win_to_clip)
            }
            ClipboardData::Extension { .. } => Err(ClipboardError::NotSupported),
        }
    }

    #[cfg(not(windows))]
    fn read_clipboard(&mut self, data_type: ClipboardType) -> Result<ClipboardData, ClipboardError> {
        match data_type {
            ClipboardType::Text => self.arboard()?.get_text().map_err(util::arboard_to_clip).map(ClipboardData::Text),
            ClipboardType::Image => {
                let bitmap = self.arboard()?.get_image().map_err(util::arboard_to_clip)?;
                let mut data = bitmap.bytes.into_owned();
                for rgba in data.chunks_exact_mut(4) {
                    rgba.swap(0, 2); // to bgra
                }
                let id = self.image_cache.add(ImageRequest {
                    format: ImageDataFormat::Bgra8 {
                        size: PxSize::new(Px(bitmap.width as _), Px(bitmap.height as _)),
                        ppi: None,
                    },
                    data: IpcBytes::from_vec(data),
                    max_decoded_len: u64::MAX,
                    downscale: None,
                });
                Ok(ClipboardData::Image(id))
            }
            ClipboardType::FileList => Err(ClipboardError::NotSupported),
            ClipboardType::Extension(_) => Err(ClipboardError::NotSupported),
        }
    }

    #[cfg(not(windows))]
    fn write_clipboard(&mut self, data: ClipboardData) -> Result<(), ClipboardError> {
        match data {
            ClipboardData::Text(t) => self.arboard()?.set_text(t).map_err(util::arboard_to_clip),
            ClipboardData::Image(id) => {
                self.arboard()?;
                if let Some(img) = self.image_cache.get(id) {
                    let size = img.size();
                    let mut data = img.pixels().clone().to_vec();
                    for rgba in data.chunks_exact_mut(4) {
                        rgba.swap(0, 2); // to rgba
                    }
                    let board = self.arboard()?;
                    let _ = board.set_image(arboard::ImageData {
                        width: size.width.0 as _,
                        height: size.height.0 as _,
                        bytes: std::borrow::Cow::Owned(data),
                    });
                    Ok(())
                } else {
                    Err(ClipboardError::Other("image not found".to_owned()))
                }
            }
            ClipboardData::FileList(_) => Err(ClipboardError::NotSupported),
            ClipboardData::Extension { .. } => Err(ClipboardError::NotSupported),
        }
    }

    fn app_extension(&mut self, extension_id: ApiExtensionId, extension_request: ApiExtensionPayload) -> ApiExtensionPayload {
        self.exts.call_command(extension_id, extension_request)
    }

    fn render_extension(
        &mut self,
        id: WindowId,
        extension_id: ApiExtensionId,
        extension_request: ApiExtensionPayload,
    ) -> ApiExtensionPayload {
        with_window_or_surface!(self, id, |w| w.render_extension(extension_id, extension_request), || {
            ApiExtensionPayload::invalid_request(extension_id, "renderer not found")
        })
    }
}

/// Message inserted in the event loop from the view-process.
#[derive(Debug)]
pub(crate) enum AppEvent {
    /// One or more [`RequestEvent`] are pending in the request channel.
    Request,
    /// Notify an event.
    Notify(Event),
    /// Re-query available monitors and send update event.
    #[cfg_attr(not(windows), allow(unused))]
    RefreshMonitors,

    /// Simulate winit window event Focused.
    #[cfg_attr(not(windows), allow(unused))]
    WinitFocused(winit::window::WindowId, bool),

    /// Lost connection with app-process.
    ParentProcessExited,

    /// Image finished decoding, must call [`ImageCache::loaded`].
    ImageLoaded(ImageLoadedData),

    /// Send after init if `device_events` are not requested.
    DisableDeviceEvents,

    /// Send when monitor was turned on/off by the OS, need to redraw all screens to avoid blank issue.
    MonitorPowerChanged,
}

/// Message inserted in the request loop from the view-process.
///
/// These *events* are detached from [`AppEvent`] so that we can continue receiving requests while
/// the main loop is blocked in a resize operation.
#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
enum RequestEvent {
    /// A request from the [`Api`].
    Request(Request),
    /// Webrender finished rendering a frame, ready for redraw.
    FrameReady(WindowId, FrameReadyMsg),
}

#[derive(Debug)]
pub(crate) struct FrameReadyMsg {
    pub composite_needed: bool,
}

/// Abstraction over channel senders  that can inject [`AppEvent`] in the app loop.
#[derive(Clone)]
pub(crate) enum AppEventSender {
    Headed(EventLoopProxy<AppEvent>, flume::Sender<RequestEvent>),
    Headless(flume::Sender<AppEvent>, flume::Sender<RequestEvent>),
}
impl AppEventSender {
    /// Send an event.
    fn send(&self, ev: AppEvent) -> Result<(), Disconnected> {
        match self {
            AppEventSender::Headed(p, _) => p.send_event(ev).map_err(|_| Disconnected),
            AppEventSender::Headless(p, _) => p.send(ev).map_err(|_| Disconnected),
        }
    }

    /// Send a request.
    fn request(&self, req: Request) -> Result<(), Disconnected> {
        match self {
            AppEventSender::Headed(_, p) => p.send(RequestEvent::Request(req)).map_err(|_| Disconnected),
            AppEventSender::Headless(_, p) => p.send(RequestEvent::Request(req)).map_err(|_| Disconnected),
        }?;
        self.send(AppEvent::Request)
    }

    /// Send a frame-ready.
    fn frame_ready(&self, window_id: WindowId, msg: FrameReadyMsg) -> Result<(), Disconnected> {
        match self {
            AppEventSender::Headed(_, p) => p.send(RequestEvent::FrameReady(window_id, msg)).map_err(|_| Disconnected),
            AppEventSender::Headless(_, p) => p.send(RequestEvent::FrameReady(window_id, msg)).map_err(|_| Disconnected),
        }?;
        self.send(AppEvent::Request)
    }
}

/// Webrender frame-ready notifier.
pub(crate) struct WrNotifier {
    id: WindowId,
    sender: AppEventSender,
}
impl WrNotifier {
    pub fn create(id: WindowId, sender: AppEventSender) -> Box<dyn RenderNotifier> {
        Box::new(WrNotifier { id, sender })
    }
}
impl RenderNotifier for WrNotifier {
    fn clone(&self) -> Box<dyn RenderNotifier> {
        Box::new(Self {
            id: self.id,
            sender: self.sender.clone(),
        })
    }

    fn wake_up(&self, _: bool) {}

    fn new_frame_ready(&self, _document_id: DocumentId, _scrolled: bool, composite_needed: bool, _: FramePublishId) {
        let msg = FrameReadyMsg { composite_needed };
        let _ = self.sender.frame_ready(self.id, msg);
    }
}
