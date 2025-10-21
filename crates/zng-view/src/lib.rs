#![doc(html_favicon_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo-icon.png")]
#![doc(html_logo_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo.png")]
//!
//! View-Process implementation.
//!
//! This implementation supports headed and headless apps in Windows, Linux and MacOS.
//!
//! # Usage
//!
//! First add this to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! zng = "0.18.2"
//! zng-view = "0.13.2"
//! ```
//!
//! Then call `zng::env::init` before any other code in `main` to setup a view-process that uses
//! the same app executable:
//!
//! ```
//! # macro_rules! _demo {()=>{
//! use zng::prelude::*;
//!
//! fn main() {
//!     zng::env::init!();
//!
//!     APP.defaults().run_window(|ctx| unimplemented!())
//! }
//! # }}
//! ```
//!
//! When the app is executed `run_window` gets called and internally starts the view-process.
//! The current executable is started this time configured to be a view-process, `init` detects this and highjacks the process
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
//! There is a pre-built release of this crate, [`zng-view-prebuilt`], it works as a drop-in replacement
// that dynamically links with a pre-built library, for Windows, Linux and MacOS.
//!
//! In the `Cargo.toml` file:
//!
//! ```toml
//! zng-view-prebuilt = "0.1"
//! ```
//!
//! The pre-built crate includes the `"software"` and `"ipc"` features, in fact `ipc` is required, even for running on the same process,
//! you can also configure where the pre-build library is installed, see the [`zng-view-prebuilt`] documentation for details.
//!
//! The pre-build crate does not support [`extensions`].
//!
//! # API Extensions
//!
//! This implementation of the view API provides these extensions:
//!
//! * `"zng-view.webrender_debug"`: `{ flags: DebugFlags, profiler_ui: String }`, sets Webrender debug flags.
//!     - The `zng-wgt-webrender-debug` crate implements a property that uses this extension.
//! * `"zng-view.prefer_angle": bool`, on Windows, prefer ANGLE(EGL) over WGL if the `libEGL.dll` and `libGLESv2.dll`
//!    libraries can by dynamically loaded. The `extend-view` example demonstrates this extension.
//!
//! You can also inject your own extensions, see the [`extensions`] module for more details.
//!
//! [`zng-view-prebuilt`]: https://crates.io/crates/zng-view-prebuilt/
//!
//! # Crate
//!
#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]
#![doc(test(no_crate_inject))]
#![warn(missing_docs)]
#![warn(unused_extern_crates)]

use std::{
    fmt, mem,
    path::PathBuf,
    thread,
    time::{Duration, Instant},
};

use extensions::ViewExtensions;
use gl::GlContextManager;
use image_cache::ImageCache;
use keyboard::KeyLocation;
use util::WinitToPx;
use winit::{
    event::{DeviceEvent, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy},
    keyboard::ModifiersState,
    monitor::MonitorHandle,
};

#[cfg(not(target_os = "android"))]
use winit::platform::modifier_supplement::KeyEventExtModifierSupplement;

#[cfg(target_os = "android")]
use winit::platform::android::EventLoopBuilderExtAndroid;

mod config;
mod display_list;
mod gl;
mod image_cache;
#[cfg(windows)]
mod input_device_info;
mod low_memory;
mod px_wr;
mod surface;
mod util;
mod window;

use surface::*;

pub mod extensions;

pub mod platform;

/// Webrender build used in the view-process.
#[doc(no_inline)]
pub use webrender;

/// OpenGL bindings used by Webrender.
#[doc(no_inline)]
pub use gleam;

use webrender::api::*;
use window::Window;
use zng_txt::Txt;
use zng_unit::{Dip, DipPoint, DipRect, DipSideOffsets, DipSize, Factor, Px, PxPoint, PxRect, PxToDip};
use zng_view_api::{
    Inited,
    api_extension::{ApiExtensionId, ApiExtensionPayload},
    dialog::{DialogId, FileDialog, MsgDialog, MsgDialogResponse},
    drag_drop::*,
    font::{FontFaceId, FontId, FontOptions, FontVariationName},
    image::{ImageId, ImageLoadedData, ImageMaskMode, ImageRequest, ImageTextureId},
    ipc::{IpcBytes, IpcBytesReceiver},
    keyboard::{Key, KeyCode, KeyState},
    mouse::ButtonId,
    raw_input::{InputDeviceCapability, InputDeviceEvent, InputDeviceId, InputDeviceInfo},
    touch::{TouchId, TouchUpdate},
    window::{
        CursorIcon, CursorImage, EventCause, EventFrameRendered, FocusIndicator, FrameRequest, FrameUpdateRequest, FrameWaitId,
        HeadlessOpenData, HeadlessRequest, MonitorId, MonitorInfo, VideoMode, WindowChanged, WindowId, WindowOpenData, WindowRequest,
        WindowState, WindowStateAll,
    },
    *,
};

use rustc_hash::FxHashMap;

#[cfg(ipc)]
zng_env::on_process_start!(|args| {
    if std::env::var("ZNG_VIEW_NO_INIT_START").is_err() {
        if args.yield_count == 0 {
            // give tracing handlers a chance to observe the view-process
            return args.yield_once();
        }

        view_process_main();
    }
});

/// Runs the view-process server.
///
/// Note that this only needs to be called if the view-process is not built on the same executable, if
/// it is you only need to call [`zng_env::init!`] at the beginning of the executable main.
///
/// You can also disable start on init by setting the `"ZNG_VIEW_NO_INIT_START"` environment variable. In this
/// case you must manually call this function.
#[cfg(ipc)]
pub fn view_process_main() {
    let config = match ViewConfig::from_env() {
        Some(c) => c,
        None => return,
    };

    zng_env::set_process_name("view-process");

    std::panic::set_hook(Box::new(init_abort));
    config.assert_version(false);
    let c = ipc::connect_view_process(config.server_name).expect("failed to connect to app-process");

    let mut ext = ViewExtensions::new();
    for e in extensions::VIEW_EXTENSIONS {
        e(&mut ext);
    }

    if config.headless {
        App::run_headless(c, ext);
    } else {
        App::run_headed(c, ext);
    }

    zng_env::exit(0)
}

#[cfg(ipc)]
#[doc(hidden)]
#[unsafe(no_mangle)] // SAFETY: minimal risk of name collision, nothing else to do
pub extern "C" fn extern_view_process_main(patch: &StaticPatch) {
    std::panic::set_hook(Box::new(ffi_abort));

    // SAFETY:
    // safe because it is called before any view related code in the library.
    unsafe {
        patch.install();
    }

    view_process_main()
}

/// Runs the view-process server in the current process and calls `run_app` to also
/// run the app in the current process. Note that `run_app` will be called in a different thread.
///
/// In this mode the app only uses a single process, reducing the memory footprint, but it is also not
/// resilient to video driver crashes, the view server **does not** respawn in this mode.
///
/// # Panics
///
/// Panics if not called in the main thread, this is a requirement of some operating systems.
///
/// ## Background Panics Warning
///
/// Note that `webrender` can freeze due to panics in worker threads without propagating
/// the panics to the main thread, this causes the app to stop responding while still receiving
/// event signals, causing the operating system to not detect that the app is frozen. It is recommended
/// that you build with `panic=abort` or use [`std::panic::set_hook`] to detect these background panics.
///
/// # Android
///
/// In Android builds `android::init_android_app` must be called before this function, otherwise it will panic.
pub fn run_same_process(run_app: impl FnOnce() + Send + 'static) {
    run_same_process_extended(run_app, ViewExtensions::new)
}

/// Like [`run_same_process`] but with custom API extensions.
///
/// Note that any linked [`view_process_extension!`] extensions are also run, after `ext`.
pub fn run_same_process_extended(run_app: impl FnOnce() + Send + 'static, ext: fn() -> ViewExtensions) {
    let app_thread = thread::Builder::new()
        .name("app".to_owned())
        .spawn(move || {
            // SAFETY: we exit the process in case of panic.
            if let Err(e) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(run_app)) {
                thread::Builder::new()
                    .name("ensure-exit".into())
                    .stack_size(256 * 1024)
                    .spawn(|| {
                        // Sometimes the channel does not disconnect on panic,
                        // observed this issue on a panic in `AppExtension::init`.
                        //
                        // This workaround ensures that we don't become a zombie process.
                        thread::sleep(std::time::Duration::from_secs(5));
                        eprintln!("run_same_process did not exit after 5s of a fatal panic, exiting now");
                        zng_env::exit(101);
                    })
                    .expect("failed to spawn thread");
                // Propagate panic in case the normal disconnect/shutdown handler works.
                std::panic::resume_unwind(e);
            }
        })
        .unwrap();

    let config = ViewConfig::wait_same_process();
    config.assert_version(true);

    let c = ipc::connect_view_process(config.server_name).expect("failed to connect to app in same process");

    let mut ext = ext();
    for e in extensions::VIEW_EXTENSIONS {
        e(&mut ext);
    }

    if config.headless {
        App::run_headless(c, ext);
    } else {
        App::run_headed(c, ext);
    }

    if let Err(p) = app_thread.join() {
        std::panic::resume_unwind(p);
    }
}

#[cfg(ipc)]
#[doc(hidden)]
#[unsafe(no_mangle)] // SAFETY minimal risk of name collision, nothing else to do
pub extern "C" fn extern_run_same_process(patch: &StaticPatch, run_app: extern "C" fn()) {
    std::panic::set_hook(Box::new(ffi_abort));

    // SAFETY:
    // safe because it is called before any view related code in the library.
    unsafe {
        patch.install();
    }

    #[expect(clippy::redundant_closure)] // false positive
    run_same_process(move || run_app())
}
#[cfg(ipc)]
fn init_abort(info: &std::panic::PanicHookInfo) {
    panic_hook(info, "note: aborting to respawn");
}
#[cfg(ipc)]
fn ffi_abort(info: &std::panic::PanicHookInfo) {
    panic_hook(info, "note: aborting to avoid unwind across FFI");
}
#[cfg(ipc)]
fn panic_hook(info: &std::panic::PanicHookInfo, details: &str) {
    // see `default_hook` in https://doc.rust-lang.org/src/std/panicking.rs.html#182

    let panic = util::SuppressedPanic::from_hook(info, std::backtrace::Backtrace::force_capture());

    if crate::util::suppress_panic() {
        crate::util::set_suppressed_panic(panic);
    } else {
        eprintln!("{panic}\n{details}");
        zng_env::exit(101) // Rust panic exit code.
    }
}

/// The backend implementation.
pub(crate) struct App {
    headless: bool,

    exts: ViewExtensions,

    gl_manager: GlContextManager,
    winit_loop: util::WinitEventLoop,
    idle: IdleTrace,
    app_sender: AppEventSender,
    request_recv: flume::Receiver<RequestEvent>,

    response_sender: ipc::ResponseSender,
    event_sender: ipc::EventSender,
    image_cache: ImageCache,

    generation: ViewProcessGen,
    device_events_filter: DeviceEventsFilter,

    windows: Vec<Window>,
    surfaces: Vec<Surface>,

    monitor_id_gen: MonitorId,
    pub monitors: Vec<(MonitorId, MonitorHandle)>,

    device_id_gen: InputDeviceId,
    devices: Vec<(InputDeviceId, winit::event::DeviceId, InputDeviceInfo)>,

    dialog_id_gen: DialogId,

    resize_frame_wait_id_gen: FrameWaitId,

    coalescing_event: Option<(Event, Instant)>,
    // winit only sends a CursorMove after CursorEntered if the cursor is in a different position,
    // but this makes refreshing hit-tests weird, do we hit-test the previous known point at each CursorEnter?
    //
    // This flag causes a MouseMove at the same previous position if no mouse move was send after CursorEnter and before
    // MainEventsCleared.
    cursor_entered_expect_move: Vec<WindowId>,

    #[cfg(windows)]
    skip_ralt: bool,

    pressed_modifiers: FxHashMap<(Key, KeyLocation), (InputDeviceId, KeyCode)>,
    pending_modifiers_update: Option<ModifiersState>,
    pending_modifiers_focus_clear: bool,

    #[cfg(not(any(windows, target_os = "android")))]
    arboard: Option<arboard::Clipboard>,

    low_memory_watcher: Option<low_memory::LowMemoryWatcher>,

    config_listener_exit: Option<Box<dyn FnOnce()>>,

    app_state: AppState,
    drag_drop_hovered: Option<(WindowId, DipPoint)>,
    drag_drop_next_move: Option<(Instant, PathBuf)>,
    exited: bool,
}
impl fmt::Debug for App {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HeadlessBackend")
            .field("app_state", &self.app_state)
            .field("generation", &self.generation)
            .field("device_events_filter", &self.device_events_filter)
            .field("windows", &self.windows)
            .field("surfaces", &self.surfaces)
            .finish_non_exhaustive()
    }
}
impl winit::application::ApplicationHandler<AppEvent> for App {
    fn resumed(&mut self, winit_loop: &ActiveEventLoop) {
        if let AppState::Suspended = self.app_state {
            let mut winit_loop_guard = self.winit_loop.set(winit_loop);

            self.exts.resumed();
            self.generation = self.generation.next();

            self.init(self.generation.next(), true, self.headless);

            winit_loop_guard.unset(&mut self.winit_loop);
        } else {
            self.exts.init(&self.app_sender);
        }
        self.app_state = AppState::Resumed;

        self.update_memory_watcher(winit_loop);
    }

    fn window_event(&mut self, winit_loop: &ActiveEventLoop, window_id: winit::window::WindowId, event: WindowEvent) {
        let i = if let Some((i, _)) = self.windows.iter_mut().enumerate().find(|(_, w)| w.window_id() == window_id) {
            i
        } else {
            return;
        };

        let _s = tracing::trace_span!("on_window_event", ?event).entered();

        let mut winit_loop_guard = self.winit_loop.set(winit_loop);

        self.windows[i].on_window_event(&event);

        let id = self.windows[i].id();
        let scale_factor = self.windows[i].scale_factor();

        // Linux dialog is not actually modal, the parent window can continue to receive interaction events,
        // this macro return early when a modal dialog is open in Linux.
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
                    winit_loop_guard.unset(&mut self.winit_loop);
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
            WindowEvent::RedrawRequested => self.windows[i].redraw(),
            WindowEvent::Resized(_) => {
                let size = if let Some(size) = self.windows[i].resized() {
                    size
                } else {
                    winit_loop_guard.unset(&mut self.winit_loop);
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

                    self.notify(Event::WindowChanged(WindowChanged::monitor_changed(id, m_id, EventCause::System)));
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
                            winit_loop_guard.unset(&mut self.winit_loop);
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
                let (global_position, position) = if let Some(p) = self.windows[i].moved() {
                    p
                } else {
                    winit_loop_guard.unset(&mut self.winit_loop);
                    return;
                };

                if let Some(state) = self.windows[i].state_change() {
                    self.notify(Event::WindowChanged(WindowChanged::state_changed(id, state, EventCause::System)));
                }

                self.notify(Event::WindowChanged(WindowChanged::moved(
                    id,
                    global_position,
                    position,
                    EventCause::System,
                )));

                if let Some(handle) = self.windows[i].monitor_change() {
                    let m_id = self.monitor_handle_to_id(&handle);

                    self.notify(Event::WindowChanged(WindowChanged::monitor_changed(id, m_id, EventCause::System)));
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
            WindowEvent::HoveredFile(file) => {
                linux_modal_dialog_bail!();

                // winit does not provide mouse move events during drag/drop,
                // so we enable device events to get mouse move, and use native APIs to get
                // the cursor position on the window.
                if self.device_events_filter.input.is_empty() {
                    winit_loop.listen_device_events(winit::event_loop::DeviceEvents::Always);
                }
                self.drag_drop_hovered = Some((id, DipPoint::splat(Dip::new(-1000))));
                self.notify(Event::DragHovered {
                    window: id,
                    data: vec![DragDropData::Path(file)],
                    allowed: DragDropEffect::all(),
                });
            }
            WindowEvent::DroppedFile(file) => {
                linux_modal_dialog_bail!();

                if self.device_events_filter.input.is_empty() {
                    winit_loop.listen_device_events(winit::event_loop::DeviceEvents::Never);
                }

                let mut delay_to_next_move = true;

                // some systems (x11) don't receive even device mouse move
                if let Some(position) = self.windows[i].drag_drop_cursor_pos() {
                    self.notify(Event::DragMoved {
                        window: id,
                        coalesced_pos: vec![],
                        position,
                    });
                    delay_to_next_move = false;
                } else if let Some((_, pos)) = self.drag_drop_hovered {
                    delay_to_next_move = pos.x < Dip::new(0);
                }

                if delay_to_next_move {
                    self.drag_drop_next_move = Some((Instant::now(), file));
                } else {
                    self.notify(Event::DragDropped {
                        window: id,
                        data: vec![DragDropData::Path(file)],
                        allowed: DragDropEffect::all(),
                        drop_id: DragDropId(0),
                    });
                }
            }
            WindowEvent::HoveredFileCancelled => {
                linux_modal_dialog_bail!();

                self.drag_drop_hovered = None;
                if self.device_events_filter.input.is_empty() {
                    winit_loop.listen_device_events(winit::event_loop::DeviceEvents::Never);
                }

                if self.drag_drop_next_move.is_none() {
                    // x11 sends a cancelled after drop
                    self.notify(Event::DragCancelled { window: id });
                }
            }
            WindowEvent::Focused(mut focused) => {
                if self.windows[i].focused_changed(&mut focused) {
                    if focused {
                        self.notify(Event::FocusChanged { prev: None, new: Some(id) });

                        // some platforms (Wayland) don't change size on minimize/restore, so we check here too.
                        if let Some(state) = self.windows[i].state_change() {
                            self.notify(Event::WindowChanged(WindowChanged::state_changed(id, state, EventCause::System)));
                        }
                    } else {
                        self.pending_modifiers_focus_clear = true;
                        self.notify(Event::FocusChanged { prev: Some(id), new: None });
                    }
                }
            }
            WindowEvent::KeyboardInput {
                device_id,
                event,
                is_synthetic,
            } => {
                linux_modal_dialog_bail!();

                if !is_synthetic && self.windows[i].is_focused() {
                    // see the Window::focus comments.
                    #[cfg(windows)]
                    if self.skip_ralt
                        && let winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::AltRight) = event.physical_key
                    {
                        winit_loop_guard.unset(&mut self.winit_loop);
                        return;
                    }

                    let state = util::element_state_to_key_state(event.state);
                    #[cfg(not(target_os = "android"))]
                    let key = util::winit_key_to_key(event.key_without_modifiers());
                    let key_modified = util::winit_key_to_key(event.logical_key);
                    #[cfg(target_os = "android")]
                    let key = key_modified.clone();
                    let key_code = util::winit_physical_key_to_key_code(event.physical_key);
                    let key_location = util::winit_key_location_to_zng(event.location);
                    let d_id = self.input_device_id(device_id, InputDeviceCapability::KEY);

                    let mut send_event = true;

                    if key.is_modifier() {
                        match state {
                            KeyState::Pressed => {
                                send_event = self
                                    .pressed_modifiers
                                    .insert((key.clone(), key_location), (d_id, key_code))
                                    .is_none();
                            }
                            KeyState::Released => send_event = self.pressed_modifiers.remove(&(key.clone(), key_location)).is_some(),
                        }
                    }

                    if send_event {
                        self.notify(Event::KeyboardInput {
                            window: id,
                            device: d_id,
                            key_code,
                            key_location,
                            state,
                            text: match event.text {
                                Some(s) => Txt::from_str(s.as_str()),
                                #[cfg(target_os = "android")]
                                None => match (state, &key) {
                                    (KeyState::Pressed, Key::Char(c)) => Txt::from(*c),
                                    (KeyState::Pressed, Key::Str(s)) => s.clone(),
                                    _ => Txt::default(),
                                },
                                #[cfg(not(target_os = "android"))]
                                None => Txt::default(),
                            },
                            key,
                            key_modified,
                        });
                    }
                }
            }
            WindowEvent::ModifiersChanged(m) => {
                linux_modal_dialog_bail!();
                if self.windows[i].is_focused() {
                    self.pending_modifiers_update = Some(m.state());
                }
            }
            WindowEvent::CursorMoved { device_id, position, .. } => {
                linux_modal_dialog_bail!();

                let px_p = position.to_px();
                let p = px_p.to_dip(scale_factor);
                let d_id = self.input_device_id(device_id, InputDeviceCapability::POINTER_MOTION);

                let mut is_after_cursor_enter = false;
                if let Some(i) = self.cursor_entered_expect_move.iter().position(|&w| w == id) {
                    self.cursor_entered_expect_move.remove(i);
                    is_after_cursor_enter = true;
                }

                if self.windows[i].cursor_moved(p, d_id) || is_after_cursor_enter {
                    self.notify(Event::MouseMoved {
                        window: id,
                        device: d_id,
                        coalesced_pos: vec![],
                        position: p,
                    });
                }

                if let Some((drop_moment, file)) = self.drag_drop_next_move.take()
                    && drop_moment.elapsed() < Duration::from_millis(300)
                {
                    let window_id = self.windows[i].id();
                    self.notify(Event::DragMoved {
                        window: window_id,
                        coalesced_pos: vec![],
                        position: p,
                    });
                    self.notify(Event::DragDropped {
                        window: window_id,
                        data: vec![DragDropData::Path(file)],
                        allowed: DragDropEffect::all(),
                        drop_id: DragDropId(0),
                    });
                }
            }
            WindowEvent::CursorEntered { device_id } => {
                linux_modal_dialog_bail!();
                if self.windows[i].cursor_entered() {
                    let d_id = self.input_device_id(device_id, InputDeviceCapability::POINTER_MOTION);
                    self.notify(Event::MouseEntered { window: id, device: d_id });
                    self.cursor_entered_expect_move.push(id);
                }
            }
            WindowEvent::CursorLeft { device_id } => {
                linux_modal_dialog_bail!();
                if self.windows[i].cursor_left() {
                    let d_id = self.input_device_id(device_id, InputDeviceCapability::POINTER_MOTION);
                    self.notify(Event::MouseLeft { window: id, device: d_id });

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
                let d_id = self.input_device_id(device_id, InputDeviceCapability::SCROLL_MOTION);
                self.notify(Event::MouseWheel {
                    window: id,
                    device: d_id,
                    delta: util::winit_mouse_wheel_delta_to_zng(delta),
                    phase: util::winit_touch_phase_to_zng(phase),
                });
            }
            WindowEvent::MouseInput {
                device_id, state, button, ..
            } => {
                linux_modal_dialog_bail!();
                let d_id = self.input_device_id(device_id, InputDeviceCapability::BUTTON);
                self.notify(Event::MouseInput {
                    window: id,
                    device: d_id,
                    state: util::element_state_to_button_state(state),
                    button: util::winit_mouse_button_to_zng(button),
                });
            }
            WindowEvent::TouchpadPressure {
                device_id,
                pressure,
                stage,
            } => {
                linux_modal_dialog_bail!();
                let d_id = self.input_device_id(device_id, InputDeviceCapability::empty());
                self.notify(Event::TouchpadPressure {
                    window: id,
                    device: d_id,
                    pressure,
                    stage,
                });
            }
            WindowEvent::AxisMotion { device_id, axis, value } => {
                linux_modal_dialog_bail!();
                let d_id = self.input_device_id(device_id, InputDeviceCapability::AXIS_MOTION);
                self.notify(Event::AxisMotion {
                    window: id,
                    device: d_id,
                    axis: AxisId(axis),
                    value,
                });
            }
            WindowEvent::Touch(t) => {
                let d_id = self.input_device_id(t.device_id, InputDeviceCapability::empty());
                let position = t.location.to_px().to_dip(scale_factor);

                let notify = match t.phase {
                    winit::event::TouchPhase::Moved => self.windows[i].touch_moved(position, d_id, t.id),
                    winit::event::TouchPhase::Started => true,
                    winit::event::TouchPhase::Ended | winit::event::TouchPhase::Cancelled => {
                        self.windows[i].touch_end(d_id, t.id);
                        true
                    }
                };

                if notify {
                    self.notify(Event::Touch {
                        window: id,
                        device: d_id,
                        touches: vec![TouchUpdate::new(
                            TouchId(t.id),
                            util::winit_touch_phase_to_zng(t.phase),
                            position,
                            t.force.map(util::winit_force_to_zng),
                        )],
                    });
                }
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
                        EventCause::System,
                    )));
                }
                self.notify(Event::ScaleFactorChanged {
                    monitor,
                    windows: vec![id],
                    scale_factor: scale_factor as f32,
                });

                if let Some(size) = self.windows[i].resized() {
                    self.notify(Event::WindowChanged(WindowChanged::resized(id, size, EventCause::System, None)));
                }
            }
            WindowEvent::Ime(ime) => {
                linux_modal_dialog_bail!();

                match ime {
                    winit::event::Ime::Preedit(s, c) => {
                        let caret = c.unwrap_or((s.len(), s.len()));
                        let ime = Ime::Preview(s.into(), caret);
                        self.notify(Event::Ime { window: id, ime });
                    }
                    winit::event::Ime::Commit(s) => {
                        let ime = Ime::Commit(s.into());
                        self.notify(Event::Ime { window: id, ime });
                    }
                    winit::event::Ime::Enabled => {}
                    winit::event::Ime::Disabled => {}
                }
            }
            WindowEvent::ThemeChanged(_) => {}
            WindowEvent::Occluded(_) => {}
            WindowEvent::ActivationTokenDone { .. } => {}
            WindowEvent::PinchGesture { .. } => {}
            WindowEvent::RotationGesture { .. } => {}
            WindowEvent::DoubleTapGesture { .. } => {}
            WindowEvent::PanGesture { .. } => {}
        }

        winit_loop_guard.unset(&mut self.winit_loop);
    }

    fn new_events(&mut self, _winit_loop: &ActiveEventLoop, _cause: winit::event::StartCause) {
        self.idle.exit();

        #[cfg(windows)]
        if let winit::event::StartCause::ResumeTimeReached { .. } = _cause {
            self.update_memory_watcher(_winit_loop);
        }
    }

    fn user_event(&mut self, winit_loop: &ActiveEventLoop, ev: AppEvent) {
        let mut winit_loop_guard = self.winit_loop.set(winit_loop);
        match ev {
            AppEvent::Request => {
                while let Ok(req) = self.request_recv.try_recv() {
                    match req {
                        RequestEvent::Request(req) => {
                            let rsp = self.respond(req);
                            if rsp.must_be_send() && self.response_sender.send(rsp).is_err() {
                                // lost connection to app-process
                                self.exited = true;
                                self.winit_loop.exit();
                            }
                        }
                        RequestEvent::FrameReady(wid, msg) => self.on_frame_ready(wid, msg),
                    }
                }
            }
            AppEvent::Notify(ev) => self.notify(ev),
            AppEvent::WinitFocused(window_id, focused) => self.window_event(winit_loop, window_id, WindowEvent::Focused(focused)),
            AppEvent::RefreshMonitors => self.refresh_monitors(),
            AppEvent::ParentProcessExited => {
                self.exited = true;
                self.winit_loop.exit();
            }
            AppEvent::ImageLoaded(data) => {
                self.image_cache.loaded(data);
            }
            AppEvent::MonitorPowerChanged => {
                // if a window opens in power-off it is blank until redraw.
                for w in &mut self.windows {
                    w.redraw();
                }
            }
            AppEvent::SetDeviceEventsFilter(filter) => {
                self.set_device_events_filter(filter, Some(winit_loop));
            }
        }
        winit_loop_guard.unset(&mut self.winit_loop);
    }

    fn device_event(&mut self, winit_loop: &ActiveEventLoop, device_id: winit::event::DeviceId, event: DeviceEvent) {
        let filter = self.device_events_filter.input;

        if !filter.is_empty() {
            let _s = tracing::trace_span!("on_device_event", ?event);

            let mut winit_loop_guard = self.winit_loop.set(winit_loop);

            match &event {
                DeviceEvent::Added => {
                    let _ = self.input_device_id(device_id, InputDeviceCapability::empty());
                    // already notifies here
                }
                DeviceEvent::Removed => {
                    if let Some(i) = self.devices.iter().position(|(_, id, _)| *id == device_id) {
                        self.devices.remove(i);
                        self.notify_input_devices_changed();
                    }
                }
                DeviceEvent::MouseMotion { delta } => {
                    let cap = InputDeviceCapability::POINTER_MOTION;
                    if filter.contains(cap) {
                        let d_id = self.input_device_id(device_id, cap);
                        self.notify(Event::InputDeviceEvent {
                            device: d_id,
                            event: InputDeviceEvent::PointerMotion {
                                delta: euclid::vec2(delta.0, delta.1),
                            },
                        });
                    }
                }
                DeviceEvent::MouseWheel { delta } => {
                    let cap = InputDeviceCapability::SCROLL_MOTION;
                    if filter.contains(cap) {
                        let d_id = self.input_device_id(device_id, cap);
                        self.notify(Event::InputDeviceEvent {
                            device: d_id,
                            event: InputDeviceEvent::ScrollMotion {
                                delta: util::winit_mouse_wheel_delta_to_zng(*delta),
                            },
                        });
                    }
                }
                DeviceEvent::Motion { axis, value } => {
                    let cap = InputDeviceCapability::AXIS_MOTION;
                    if filter.contains(cap) {
                        let d_id = self.input_device_id(device_id, cap);
                        self.notify(Event::InputDeviceEvent {
                            device: d_id,
                            event: InputDeviceEvent::AxisMotion {
                                axis: AxisId(*axis),
                                value: *value,
                            },
                        });
                    }
                }
                DeviceEvent::Button { button, state } => {
                    let cap = InputDeviceCapability::BUTTON;
                    if filter.contains(cap) {
                        let d_id = self.input_device_id(device_id, cap);
                        self.notify(Event::InputDeviceEvent {
                            device: d_id,
                            event: InputDeviceEvent::Button {
                                button: ButtonId(*button),
                                state: util::element_state_to_button_state(*state),
                            },
                        });
                    }
                }
                DeviceEvent::Key(k) => {
                    let cap = InputDeviceCapability::KEY;
                    if filter.contains(cap) {
                        let d_id = self.input_device_id(device_id, cap);
                        self.notify(Event::InputDeviceEvent {
                            device: d_id,
                            event: InputDeviceEvent::Key {
                                key_code: util::winit_physical_key_to_key_code(k.physical_key),
                                state: util::element_state_to_key_state(k.state),
                            },
                        });
                    }
                }
            }

            winit_loop_guard.unset(&mut self.winit_loop);
        }

        if let Some((id, pos)) = &mut self.drag_drop_hovered
            && let DeviceEvent::MouseMotion { .. } = &event
            && let Some(win) = self.windows.iter().find(|w| w.id() == *id)
            && let Some(new_pos) = win.drag_drop_cursor_pos()
            && *pos != new_pos
        {
            *pos = new_pos;
            let event = Event::DragMoved {
                window: *id,
                coalesced_pos: vec![],
                position: *pos,
            };
            self.notify(event);
        }
    }

    fn about_to_wait(&mut self, winit_loop: &ActiveEventLoop) {
        let mut winit_loop_guard = self.winit_loop.set(winit_loop);

        self.finish_cursor_entered_move();
        self.update_modifiers();
        self.flush_coalesced();
        #[cfg(windows)]
        {
            self.skip_ralt = false;
        }
        self.idle.enter();

        winit_loop_guard.unset(&mut self.winit_loop);
    }

    fn suspended(&mut self, _: &ActiveEventLoop) {
        #[cfg(target_os = "android")]
        if let Some(w) = &self.windows.first() {
            self.notify(Event::FocusChanged {
                prev: Some(w.id()),
                new: None,
            });
        }

        self.app_state = AppState::Suspended;
        self.windows.clear();
        self.surfaces.clear();
        self.image_cache.clear();
        self.exts.suspended();

        self.notify(Event::Suspended);
    }

    fn exiting(&mut self, event_loop: &ActiveEventLoop) {
        let _ = event_loop;
        if let Some(t) = self.config_listener_exit.take() {
            t();
        }
    }

    fn memory_warning(&mut self, winit_loop: &ActiveEventLoop) {
        let mut winit_loop_guard = self.winit_loop.set(winit_loop);

        self.image_cache.on_low_memory();
        for w in &mut self.windows {
            w.on_low_memory();
        }
        for s in &mut self.surfaces {
            s.on_low_memory();
        }
        self.exts.on_low_memory();
        self.notify(Event::LowMemory);

        winit_loop_guard.unset(&mut self.winit_loop);
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AppState {
    PreInitSuspended,
    Resumed,
    Suspended,
}

struct IdleTrace(Option<tracing::span::EnteredSpan>);
impl IdleTrace {
    pub fn enter(&mut self) {
        self.0 = Some(tracing::trace_span!("<winit-idle>").entered());
    }
    pub fn exit(&mut self) {
        self.0 = None;
    }
}
impl App {
    fn set_device_events_filter(&mut self, filter: DeviceEventsFilter, t: Option<&ActiveEventLoop>) {
        self.device_events_filter = filter;

        if let Some(t) = t {
            if !self.device_events_filter.input.is_empty() {
                t.listen_device_events(winit::event_loop::DeviceEvents::Always);
            } else {
                t.listen_device_events(winit::event_loop::DeviceEvents::Never);
            }
        }
    }

    pub fn run_headless(ipc: ipc::ViewChannels, ext: ViewExtensions) {
        tracing::info!("running headless view-process");

        gl::warmup();

        let (app_sender, app_receiver) = flume::unbounded();
        let (request_sender, request_receiver) = flume::unbounded();
        let mut app = App::new(
            AppEventSender::Headless(app_sender, request_sender),
            ipc.response_sender,
            ipc.event_sender,
            request_receiver,
            ext,
        );
        app.headless = true;

        let winit_span = tracing::trace_span!("winit::EventLoop::new").entered();
        #[cfg(not(target_os = "android"))]
        let event_loop = EventLoop::builder().build().unwrap();
        #[cfg(target_os = "android")]
        let event_loop = EventLoop::builder()
            .with_android_app(platform::android::android_app())
            .build()
            .unwrap();
        drop(winit_span);

        let mut app = HeadlessApp {
            app,
            request_receiver: Some(ipc.request_receiver),
            app_receiver,
        };
        if let Err(e) = event_loop.run_app(&mut app) {
            if app.app.exited {
                // Ubuntu CI runs can get an error here:
                //
                //  "GLXBadWindow", error_code: 170, request_code: 150, minor_code: 32
                //
                // The app run exit ok, so we just log and ignore.
                tracing::error!("winit event loop error after app exit, {e}");
            } else {
                panic!("winit event loop error, {e}");
            }
        }

        struct HeadlessApp {
            app: App,
            request_receiver: Option<ipc::RequestReceiver>,
            app_receiver: flume::Receiver<AppEvent>,
        }
        impl winit::application::ApplicationHandler<()> for HeadlessApp {
            fn resumed(&mut self, winit_loop: &ActiveEventLoop) {
                let mut winit_loop_guard = self.app.winit_loop.set(winit_loop);

                self.app.resumed(winit_loop);
                self.app.start_receiving(self.request_receiver.take().unwrap());

                'app_loop: while !self.app.exited {
                    match self.app_receiver.recv() {
                        Ok(app_ev) => match app_ev {
                            AppEvent::Request => {
                                while let Ok(request) = self.app.request_recv.try_recv() {
                                    match request {
                                        RequestEvent::Request(request) => {
                                            let response = self.app.respond(request);
                                            if response.must_be_send() && self.app.response_sender.send(response).is_err() {
                                                self.app.exited = true;
                                                break 'app_loop;
                                            }
                                        }
                                        RequestEvent::FrameReady(id, msg) => {
                                            let r = if let Some(s) = self.app.surfaces.iter_mut().find(|s| s.id() == id) {
                                                Some(s.on_frame_ready(msg, &mut self.app.image_cache))
                                            } else {
                                                None
                                            };
                                            if let Some((frame_id, image)) = r {
                                                self.app.notify(Event::FrameRendered(EventFrameRendered::new(id, frame_id, image)));
                                            }
                                        }
                                    }
                                }
                            }
                            AppEvent::Notify(ev) => {
                                if self.app.event_sender.send(ev).is_err() {
                                    self.app.exited = true;
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
                                self.app.exited = true;
                                break 'app_loop;
                            }
                            AppEvent::ImageLoaded(data) => {
                                self.app.image_cache.loaded(data);
                            }
                            AppEvent::MonitorPowerChanged => {} // headless
                            AppEvent::SetDeviceEventsFilter(filter) => {
                                self.app.set_device_events_filter(filter, None);
                            }
                        },
                        Err(_) => {
                            self.app.exited = true;
                            break 'app_loop;
                        }
                    }
                }

                self.app.winit_loop.exit();

                winit_loop_guard.unset(&mut self.app.winit_loop);
            }

            fn window_event(&mut self, _: &ActiveEventLoop, _: winit::window::WindowId, _: WindowEvent) {}

            fn suspended(&mut self, event_loop: &ActiveEventLoop) {
                self.app.suspended(event_loop);
            }
        }
    }

    pub fn run_headed(ipc: ipc::ViewChannels, ext: ViewExtensions) {
        tracing::info!("running headed view-process");

        gl::warmup();

        let winit_span = tracing::trace_span!("winit::EventLoop::new").entered();
        #[cfg(not(target_os = "android"))]
        let event_loop = EventLoop::with_user_event().build().unwrap();
        #[cfg(target_os = "android")]
        let event_loop = EventLoop::with_user_event()
            .with_android_app(platform::android::android_app())
            .build()
            .unwrap();
        drop(winit_span);
        let app_sender = event_loop.create_proxy();

        let (request_sender, request_receiver) = flume::unbounded();
        let mut app = App::new(
            AppEventSender::Headed(app_sender, request_sender),
            ipc.response_sender,
            ipc.event_sender,
            request_receiver,
            ext,
        );
        app.start_receiving(ipc.request_receiver);

        app.config_listener_exit = config::spawn_listener(app.app_sender.clone());

        if let Err(e) = event_loop.run_app(&mut app) {
            if app.exited {
                tracing::error!("winit event loop error after app exit, {e}");
            } else {
                panic!("winit event loop error, {e}");
            }
        }
    }

    fn new(
        app_sender: AppEventSender,
        response_sender: ipc::ResponseSender,
        event_sender: ipc::EventSender,
        request_recv: flume::Receiver<RequestEvent>,
        mut exts: ViewExtensions,
    ) -> Self {
        exts.renderer("zng-view.webrender_debug", extensions::RendererDebugExt::new);
        #[cfg(windows)]
        {
            exts.window("zng-view.prefer_angle", extensions::PreferAngleExt::new);
        }
        let mut idle = IdleTrace(None);
        idle.enter();
        App {
            headless: false,
            exts,
            idle,
            gl_manager: GlContextManager::default(),
            image_cache: ImageCache::new(app_sender.clone()),
            app_sender,
            request_recv,
            response_sender,
            event_sender,
            winit_loop: util::WinitEventLoop::default(),
            generation: ViewProcessGen::INVALID,
            device_events_filter: DeviceEventsFilter::empty(),
            windows: vec![],
            surfaces: vec![],
            monitors: vec![],
            monitor_id_gen: MonitorId::INVALID,
            devices: vec![],
            device_id_gen: InputDeviceId::INVALID,
            dialog_id_gen: DialogId::INVALID,
            resize_frame_wait_id_gen: FrameWaitId::INVALID,
            coalescing_event: None,
            cursor_entered_expect_move: Vec::with_capacity(1),
            app_state: AppState::PreInitSuspended,
            exited: false,
            #[cfg(windows)]
            skip_ralt: false,
            pressed_modifiers: FxHashMap::default(),
            pending_modifiers_update: None,
            pending_modifiers_focus_clear: false,
            config_listener_exit: None,
            drag_drop_hovered: None,
            drag_drop_next_move: None,
            #[cfg(not(any(windows, target_os = "android")))]
            arboard: None,
            low_memory_watcher: low_memory::LowMemoryWatcher::new(),
        }
    }

    fn start_receiving(&mut self, mut request_recv: ipc::RequestReceiver) {
        let app_sender = self.app_sender.clone();
        thread::Builder::new()
            .name("request-recv".into())
            .stack_size(256 * 1024)
            .spawn(move || {
                while let Ok(r) = request_recv.recv() {
                    if let Err(ipc::ViewChannelError::Disconnected) = app_sender.request(r) {
                        break;
                    }
                }
                let _ = app_sender.send(AppEvent::ParentProcessExited);
            })
            .expect("failed to spawn thread");
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

        if let Some(m) = self.pending_modifiers_update.take()
            && let Some(id) = self.windows.iter().find(|w| w.is_focused()).map(|w| w.id())
        {
            let mut notify = vec![];
            self.pressed_modifiers.retain(|(key, location), (d_id, s_code)| {
                let mut retain = true;
                if matches!(key, Key::Super) && !m.super_key() {
                    retain = false;
                    notify.push(Event::KeyboardInput {
                        window: id,
                        device: *d_id,
                        key_code: *s_code,
                        state: KeyState::Released,
                        key: key.clone(),
                        key_location: *location,
                        key_modified: key.clone(),
                        text: Txt::from_str(""),
                    });
                }
                if matches!(key, Key::Shift) && !m.shift_key() {
                    retain = false;
                    notify.push(Event::KeyboardInput {
                        window: id,
                        device: *d_id,
                        key_code: *s_code,
                        state: KeyState::Released,
                        key: key.clone(),
                        key_location: *location,
                        key_modified: key.clone(),
                        text: Txt::from_str(""),
                    });
                }
                if matches!(key, Key::Alt | Key::AltGraph) && !m.alt_key() {
                    retain = false;
                    notify.push(Event::KeyboardInput {
                        window: id,
                        device: *d_id,
                        key_code: *s_code,
                        state: KeyState::Released,
                        key: key.clone(),
                        key_location: *location,
                        key_modified: key.clone(),
                        text: Txt::from_str(""),
                    });
                }
                if matches!(key, Key::Ctrl) && !m.control_key() {
                    retain = false;
                    notify.push(Event::KeyboardInput {
                        window: id,
                        device: *d_id,
                        key_code: *s_code,
                        state: KeyState::Released,
                        key: key.clone(),
                        key_location: *location,
                        key_modified: key.clone(),
                        text: Txt::from_str(""),
                    });
                }
                retain
            });

            for ev in notify {
                self.notify(ev);
            }
        }
    }

    fn refresh_monitors(&mut self) {
        let mut monitors = Vec::with_capacity(self.monitors.len());

        let mut changed = false;

        for (fresh_handle, (id, handle)) in self.winit_loop.available_monitors().zip(&self.monitors) {
            let id = if &fresh_handle == handle {
                *id
            } else {
                changed = true;
                self.monitor_id_gen.incr()
            };
            monitors.push((id, fresh_handle))
        }

        if changed {
            self.monitors = monitors;

            let monitors = self.available_monitors();
            self.notify(Event::MonitorsChanged(monitors));
        }
    }

    fn on_frame_ready(&mut self, window_id: WindowId, msg: FrameReadyMsg) {
        let _s = tracing::trace_span!("on_frame_ready").entered();

        if let Some(w) = self.windows.iter_mut().find(|w| w.id() == window_id) {
            let r = w.on_frame_ready(msg, &mut self.image_cache);

            let _ = self
                .event_sender
                .send(Event::FrameRendered(EventFrameRendered::new(window_id, r.frame_id, r.image)));

            if r.first_frame {
                let size = w.size();
                self.notify(Event::WindowChanged(WindowChanged::resized(window_id, size, EventCause::App, None)));
            }
        } else if let Some(s) = self.surfaces.iter_mut().find(|w| w.id() == window_id) {
            let (frame_id, image) = s.on_frame_ready(msg, &mut self.image_cache);

            self.notify(Event::FrameRendered(EventFrameRendered::new(window_id, frame_id, image)))
        }
    }

    pub(crate) fn notify(&mut self, event: Event) {
        let now = Instant::now();
        if let Some((mut coal, timestamp)) = self.coalescing_event.take() {
            let r = if now.saturating_duration_since(timestamp) >= Duration::from_millis(16) {
                Err(event)
            } else {
                coal.coalesce(event)
            };
            match r {
                Ok(()) => self.coalescing_event = Some((coal, timestamp)),
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
                        self.coalescing_event = Some((coal, now));
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
            self.coalescing_event = Some((event, now));
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
                moves.push(Event::MouseMoved {
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
        if let Some((coal, _)) = self.coalescing_event.take()
            && self.event_sender.send(coal).is_err()
        {
            let _ = self.app_sender.send(AppEvent::ParentProcessExited);
        }
    }

    #[track_caller]
    fn assert_resumed(&self) {
        assert_eq!(self.app_state, AppState::Resumed);
    }

    fn with_window<R>(&mut self, id: WindowId, action: impl FnOnce(&mut Window) -> R, not_found: impl FnOnce() -> R) -> R {
        self.assert_resumed();
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

    fn notify_input_devices_changed(&mut self) {
        let devices = self.devices.iter().map(|(id, _, info)| (*id, info.clone())).collect();
        self.notify(Event::InputDevicesChanged(devices));
    }

    /// update `capability` by usage as device metadata query is not implemented for all systems yet.
    fn input_device_id(&mut self, device_id: winit::event::DeviceId, capability: InputDeviceCapability) -> InputDeviceId {
        if let Some((id, _, info)) = self.devices.iter_mut().find(|(_, id, _)| *id == device_id) {
            let id = *id;
            if !self.device_events_filter.input.is_empty() && !capability.is_empty() && !info.capabilities.contains(capability) {
                info.capabilities |= capability;
                self.notify_input_devices_changed();
            }
            id
        } else {
            let id = self.device_id_gen.incr();

            #[cfg(not(windows))]
            let info = InputDeviceInfo::new("Winit Device", InputDeviceCapability::empty());
            #[cfg(windows)]
            let info = {
                use winit::platform::windows::DeviceIdExtWindows as _;
                if !self.device_events_filter.input.is_empty()
                    && let Some(device_path) = device_id.persistent_identifier()
                {
                    input_device_info::get(&device_path)
                } else {
                    InputDeviceInfo::new("Winit Device", InputDeviceCapability::empty())
                }
            };

            self.devices.push((id, device_id, info));

            if !self.device_events_filter.input.is_empty() {
                self.notify_input_devices_changed();
            }

            id
        }
    }

    fn available_monitors(&mut self) -> Vec<(MonitorId, MonitorInfo)> {
        let _span = tracing::trace_span!("available_monitors").entered();

        let primary = self.winit_loop.primary_monitor();
        self.winit_loop
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

    fn update_memory_watcher(&mut self, _winit_loop: &ActiveEventLoop) {
        if let Some(m) = &mut self.low_memory_watcher {
            if m.notify() {
                use winit::application::ApplicationHandler as _;
                self.memory_warning(_winit_loop);
            }
            _winit_loop.set_control_flow(winit::event_loop::ControlFlow::wait_duration(Duration::from_secs(5)));
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
            tracing::error!("window `{:?}` not found, will return fallback result", $id);
            $fallback
        }
    };
}
impl Drop for App {
    fn drop(&mut self) {
        if let Some(f) = self.config_listener_exit.take() {
            f();
        }
    }
}
impl App {
    fn open_headless_impl(&mut self, config: HeadlessRequest) -> HeadlessOpenData {
        self.assert_resumed();
        let surf = Surface::open(
            self.generation,
            config,
            &self.winit_loop,
            &mut self.gl_manager,
            self.exts.new_window(),
            self.exts.new_renderer(),
            self.app_sender.clone(),
        );
        let render_mode = surf.render_mode();

        self.surfaces.push(surf);

        HeadlessOpenData::new(render_mode)
    }

    #[cfg(not(any(windows, target_os = "android")))]
    fn arboard(&mut self) -> Result<&mut arboard::Clipboard, clipboard::ClipboardError> {
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
    fn init(&mut self, vp_gen: ViewProcessGen, is_respawn: bool, headless: bool) {
        if self.exited {
            panic!("cannot restart exited");
        }

        self.generation = vp_gen;
        self.headless = headless;

        self.notify(Event::Inited(Inited::new(vp_gen, is_respawn, self.exts.api_extensions())));

        let available_monitors = self.available_monitors();
        self.notify(Event::MonitorsChanged(available_monitors));

        let cfg = config::multi_click_config();
        if is_respawn || cfg != zng_view_api::config::MultiClickConfig::default() {
            self.notify(Event::MultiClickConfigChanged(cfg));
        }

        let cfg = config::key_repeat_config();
        if is_respawn || cfg != zng_view_api::config::KeyRepeatConfig::default() {
            self.notify(Event::KeyRepeatConfigChanged(cfg));
        }

        let cfg = config::touch_config();
        if is_respawn || cfg != zng_view_api::config::TouchConfig::default() {
            self.notify(Event::TouchConfigChanged(cfg));
        }

        let cfg = config::font_aa();
        if is_respawn || cfg != zng_view_api::config::FontAntiAliasing::default() {
            self.notify(Event::FontAaChanged(cfg));
        }

        let cfg = config::animations_config();
        if is_respawn || cfg != zng_view_api::config::AnimationsConfig::default() {
            self.notify(Event::AnimationsConfigChanged(cfg));
        }

        let cfg = config::locale_config();
        if is_respawn || cfg != zng_view_api::config::LocaleConfig::default() {
            self.notify(Event::LocaleChanged(cfg));
        }

        let cfg = config::colors_config();
        if is_respawn || cfg != zng_view_api::config::ColorsConfig::default() {
            self.notify(Event::ColorsConfigChanged(cfg));
        }

        let cfg = config::chrome_config();
        if is_respawn || cfg != zng_view_api::config::ChromeConfig::default() {
            self.notify(Event::ChromeConfigChanged(cfg));
        }
    }

    fn exit(&mut self) {
        self.assert_resumed();
        self.exited = true;
        if let Some(t) = self.config_listener_exit.take() {
            t();
        }
        // not really, but just to exit winit loop
        let _ = self.app_sender.send(AppEvent::ParentProcessExited);
    }

    fn set_device_events_filter(&mut self, filter: DeviceEventsFilter) {
        let _ = self.app_sender.send(AppEvent::SetDeviceEventsFilter(filter));
    }

    fn open_window(&mut self, mut config: WindowRequest) {
        let _s = tracing::debug_span!("open", ?config).entered();

        config.state.clamp_size();
        config.enforce_kiosk();

        if self.headless {
            let id = config.id;
            let data = self.open_headless_impl(HeadlessRequest::new(
                config.id,
                Factor(1.0),
                config.state.restore_rect.size,
                config.render_mode,
                config.extensions,
            ));
            let msg = WindowOpenData::new(
                WindowStateAll::new(
                    WindowState::Fullscreen,
                    PxPoint::zero(),
                    DipRect::from_size(config.state.restore_rect.size),
                    WindowState::Fullscreen,
                    DipSize::zero(),
                    DipSize::new(Dip::MAX, Dip::MAX),
                    false,
                ),
                None,
                (PxPoint::zero(), DipPoint::zero()),
                config.state.restore_rect.size,
                Factor(1.0),
                data.render_mode,
                DipSideOffsets::zero(),
            );

            self.notify(Event::WindowOpened(id, msg));
        } else {
            self.assert_resumed();

            #[cfg(target_os = "android")]
            if !self.windows.is_empty() {
                tracing::error!("android can only have one window");
                return;
            }

            let id = config.id;
            let win = Window::open(
                self.generation,
                config.icon.and_then(|i| self.image_cache.get(i)).and_then(|i| i.icon()),
                config
                    .cursor_image
                    .and_then(|(i, h)| self.image_cache.get(i).and_then(|i| i.cursor(h, &self.winit_loop))),
                config,
                &self.winit_loop,
                &mut self.gl_manager,
                self.exts.new_window(),
                self.exts.new_renderer(),
                self.app_sender.clone(),
            );

            let msg = WindowOpenData::new(
                win.state(),
                win.monitor().map(|h| self.monitor_id(&h)),
                win.inner_position(),
                win.size(),
                win.scale_factor(),
                win.render_mode(),
                win.safe_padding(),
            );

            self.windows.push(win);

            self.notify(Event::WindowOpened(id, msg));

            // winit does not notify focus for Android window
            #[cfg(target_os = "android")]
            {
                self.windows.last_mut().unwrap().focused_changed(&mut true);
                self.notify(Event::FocusChanged { prev: None, new: Some(id) });
            }
        }
    }

    fn open_headless(&mut self, config: HeadlessRequest) {
        let _s = tracing::debug_span!("open_headless", ?config).entered();

        let id = config.id;
        let msg = self.open_headless_impl(config);

        self.notify(Event::HeadlessOpened(id, msg));
    }

    fn close(&mut self, id: WindowId) {
        let _s = tracing::debug_span!("close_window", ?id);

        self.assert_resumed();
        if let Some(i) = self.windows.iter().position(|w| w.id() == id) {
            let _ = self.windows.swap_remove(i);
        }
        if let Some(i) = self.surfaces.iter().position(|w| w.id() == id) {
            let _ = self.surfaces.swap_remove(i);
        }
    }

    fn set_title(&mut self, id: WindowId, title: Txt) {
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
        if let Some(w) = self.windows.iter_mut().find(|w| w.id() == id)
            && w.set_state(state.clone())
        {
            let mut change = WindowChanged::state_changed(id, state, EventCause::App);

            change.size = w.resized();
            change.position = w.moved();
            if let Some(handle) = w.monitor_change() {
                let monitor = self.monitor_handle_to_id(&handle);
                change.monitor = Some(monitor);
            }

            let _ = self.app_sender.send(AppEvent::Notify(Event::WindowChanged(change)));
        }
    }

    fn set_headless_size(&mut self, renderer: WindowId, size: DipSize, scale_factor: Factor) {
        self.assert_resumed();
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

    fn focus(&mut self, id: WindowId) -> FocusResult {
        #[cfg(windows)]
        {
            let (r, s) = self.with_window(id, |w| w.focus(), || (FocusResult::Requested, false));
            self.skip_ralt = s;
            r
        }

        #[cfg(not(windows))]
        {
            self.with_window(id, |w| w.focus(), || FocusResult::Requested)
        }
    }

    fn drag_move(&mut self, id: WindowId) {
        self.with_window(id, |w| w.drag_move(), || ())
    }

    fn drag_resize(&mut self, id: WindowId, direction: zng_view_api::window::ResizeDirection) {
        self.with_window(id, |w| w.drag_resize(direction), || ())
    }

    fn set_enabled_buttons(&mut self, id: WindowId, buttons: zng_view_api::window::WindowButton) {
        self.with_window(id, |w| w.set_enabled_buttons(buttons), || ())
    }

    fn open_title_bar_context_menu(&mut self, id: WindowId, position: DipPoint) {
        self.with_window(id, |w| w.open_title_bar_context_menu(position), || ())
    }

    fn set_cursor(&mut self, id: WindowId, icon: Option<CursorIcon>) {
        self.with_window(id, |w| w.set_cursor(icon), || ())
    }

    fn set_cursor_image(&mut self, id: WindowId, icon: Option<CursorImage>) {
        let icon = icon.and_then(|img| self.image_cache.get(img.img).and_then(|i| i.cursor(img.hotspot, &self.winit_loop)));
        self.with_window(id, |w| w.set_cursor_image(icon), || ());
    }

    fn set_ime_area(&mut self, id: WindowId, area: Option<DipRect>) {
        self.with_window(id, |w| w.set_ime_area(area), || ())
    }

    fn image_decoders(&mut self) -> Vec<Txt> {
        image_cache::DECODERS.iter().map(|&s| Txt::from_static(s)).collect()
    }

    fn image_encoders(&mut self) -> Vec<Txt> {
        image_cache::ENCODERS.iter().map(|&s| Txt::from_static(s)).collect()
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

    fn encode_image(&mut self, id: ImageId, format: Txt) {
        self.image_cache.encode(id, format)
    }

    fn use_image(&mut self, id: WindowId, image_id: ImageId) -> ImageTextureId {
        if let Some(img) = self.image_cache.get(image_id) {
            with_window_or_surface!(self, id, |w| w.use_image(img), || ImageTextureId::INVALID)
        } else {
            ImageTextureId::INVALID
        }
    }

    fn update_image_use(&mut self, id: WindowId, texture_id: ImageTextureId, image_id: ImageId) {
        if let Some(img) = self.image_cache.get(image_id) {
            with_window_or_surface!(self, id, |w| w.update_image(texture_id, img), || ())
        }
    }

    fn delete_image_use(&mut self, id: WindowId, texture_id: ImageTextureId) {
        with_window_or_surface!(self, id, |w| w.delete_image(texture_id), || ())
    }

    fn add_audio(&mut self, _request: audio::AudioRequest<IpcBytes>) -> audio::AudioId {
        unimplemented!()
    }

    fn add_audio_pro(&mut self, _request: audio::AudioRequest<IpcBytesReceiver>) -> audio::AudioId {
        unimplemented!()
    }

    fn audio_decoders(&mut self) -> Vec<Txt> {
        unimplemented!()
    }

    fn forget_audio(&mut self, _id: audio::AudioId) {
        unimplemented!()
    }

    fn playback(&mut self, _request: audio::PlaybackRequest) -> audio::PlaybackId {
        unimplemented!()
    }

    fn playback_update(&mut self, _id: audio::PlaybackId, _request: audio::PlaybackUpdateRequest) {
        unimplemented!()
    }

    fn add_font_face(&mut self, id: WindowId, bytes: font::IpcFontBytes, index: u32) -> FontFaceId {
        with_window_or_surface!(self, id, |w| w.add_font_face(bytes, index), || FontFaceId::INVALID)
    }

    fn delete_font_face(&mut self, id: WindowId, font_face_id: FontFaceId) {
        with_window_or_surface!(self, id, |w| w.delete_font_face(font_face_id), || ())
    }

    fn add_font(
        &mut self,
        id: WindowId,
        font_face_id: FontFaceId,
        glyph_size: Px,
        options: FontOptions,
        variations: Vec<(FontVariationName, f32)>,
    ) -> FontId {
        with_window_or_surface!(self, id, |w| w.add_font(font_face_id, glyph_size, options, variations), || {
            FontId::INVALID
        })
    }

    fn delete_font(&mut self, id: WindowId, font_id: FontId) {
        with_window_or_surface!(self, id, |w| w.delete_font(font_id), || ())
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

    fn access_update(&mut self, id: WindowId, update: access::AccessTreeUpdate) {
        if let Some(s) = self.windows.iter_mut().find(|s| s.id() == id) {
            s.access_update(update, &self.app_sender);
        }
    }

    fn message_dialog(&mut self, id: WindowId, dialog: MsgDialog) -> DialogId {
        let r_id = self.dialog_id_gen.incr();
        if let Some(s) = self.windows.iter_mut().find(|s| s.id() == id) {
            s.message_dialog(dialog, r_id, self.app_sender.clone());
        } else {
            let r = MsgDialogResponse::Error(Txt::from_static("window not found"));
            let _ = self.app_sender.send(AppEvent::Notify(Event::MsgDialogResponse(r_id, r)));
        }
        r_id
    }

    fn file_dialog(&mut self, id: WindowId, dialog: FileDialog) -> DialogId {
        let r_id = self.dialog_id_gen.incr();
        if let Some(s) = self.windows.iter_mut().find(|s| s.id() == id) {
            s.file_dialog(dialog, r_id, self.app_sender.clone());
        } else {
            let r = MsgDialogResponse::Error(Txt::from_static("window not found"));
            let _ = self.app_sender.send(AppEvent::Notify(Event::MsgDialogResponse(r_id, r)));
        };
        r_id
    }

    #[cfg(windows)]
    fn read_clipboard(&mut self, data_type: clipboard::ClipboardType) -> Result<clipboard::ClipboardData, clipboard::ClipboardError> {
        match data_type {
            clipboard::ClipboardType::Text => {
                let _clip = clipboard_win::Clipboard::new_attempts(10).map_err(util::clipboard_win_to_clip)?;

                clipboard_win::get(clipboard_win::formats::Unicode)
                    .map_err(util::clipboard_win_to_clip)
                    .map(|s: String| clipboard::ClipboardData::Text(Txt::from_str(&s)))
            }
            clipboard::ClipboardType::Image => {
                let _clip = clipboard_win::Clipboard::new_attempts(10).map_err(util::clipboard_win_to_clip)?;

                let bitmap = clipboard_win::get(clipboard_win::formats::Bitmap).map_err(util::clipboard_win_to_clip)?;

                let id = self.image_cache.add(ImageRequest::new(
                    image::ImageDataFormat::FileExtension(Txt::from_str("bmp")),
                    IpcBytes::from_vec(bitmap),
                    u64::MAX,
                    None,
                    None,
                ));
                Ok(clipboard::ClipboardData::Image(id))
            }
            clipboard::ClipboardType::FileList => {
                let _clip = clipboard_win::Clipboard::new_attempts(10).map_err(util::clipboard_win_to_clip)?;

                clipboard_win::get(clipboard_win::formats::FileList)
                    .map_err(util::clipboard_win_to_clip)
                    .map(clipboard::ClipboardData::FileList)
            }
            clipboard::ClipboardType::Extension(_) => Err(clipboard::ClipboardError::NotSupported),
            _ => Err(clipboard::ClipboardError::NotSupported),
        }
    }

    #[cfg(windows)]
    fn write_clipboard(&mut self, data: clipboard::ClipboardData) -> Result<(), clipboard::ClipboardError> {
        use zng_txt::formatx;

        match data {
            clipboard::ClipboardData::Text(t) => {
                let _clip = clipboard_win::Clipboard::new_attempts(10).map_err(util::clipboard_win_to_clip)?;

                clipboard_win::set(clipboard_win::formats::Unicode, t).map_err(util::clipboard_win_to_clip)
            }
            clipboard::ClipboardData::Image(id) => {
                let _clip = clipboard_win::Clipboard::new_attempts(10).map_err(util::clipboard_win_to_clip)?;

                if let Some(img) = self.image_cache.get(id) {
                    let mut bmp = vec![];
                    img.encode(::image::ImageFormat::Bmp, &mut bmp)
                        .map_err(|e| clipboard::ClipboardError::Other(formatx!("{e:?}")))?;
                    clipboard_win::set(clipboard_win::formats::Bitmap, bmp).map_err(util::clipboard_win_to_clip)
                } else {
                    Err(clipboard::ClipboardError::Other(Txt::from_str("image not found")))
                }
            }
            clipboard::ClipboardData::FileList(l) => {
                use clipboard_win::Setter;
                let _clip = clipboard_win::Clipboard::new_attempts(10).map_err(util::clipboard_win_to_clip)?;

                // clipboard_win does not implement write from PathBuf
                let strs = l.into_iter().map(|p| p.display().to_string()).collect::<Vec<String>>();
                clipboard_win::formats::FileList
                    .write_clipboard(&strs)
                    .map_err(util::clipboard_win_to_clip)
            }
            clipboard::ClipboardData::Extension { .. } => Err(clipboard::ClipboardError::NotSupported),
            _ => Err(clipboard::ClipboardError::NotSupported),
        }
    }

    #[cfg(not(any(windows, target_os = "android")))]
    fn read_clipboard(&mut self, data_type: clipboard::ClipboardType) -> Result<clipboard::ClipboardData, clipboard::ClipboardError> {
        match data_type {
            clipboard::ClipboardType::Text => self
                .arboard()?
                .get_text()
                .map_err(util::arboard_to_clip)
                .map(|s| clipboard::ClipboardData::Text(zng_txt::Txt::from(s))),
            clipboard::ClipboardType::Image => {
                let bitmap = self.arboard()?.get_image().map_err(util::arboard_to_clip)?;
                let mut data = bitmap.bytes.into_owned();
                for rgba in data.chunks_exact_mut(4) {
                    rgba.swap(0, 2); // to bgra
                }
                let id = self.image_cache.add(image::ImageRequest::new(
                    image::ImageDataFormat::Bgra8 {
                        size: zng_unit::PxSize::new(Px(bitmap.width as _), Px(bitmap.height as _)),
                        ppi: None,
                    },
                    IpcBytes::from_vec(data),
                    u64::MAX,
                    None,
                    None,
                ));
                Ok(clipboard::ClipboardData::Image(id))
            }
            clipboard::ClipboardType::FileList => self
                .arboard()?
                .get()
                .file_list()
                .map_err(util::arboard_to_clip)
                .map(clipboard::ClipboardData::FileList),
            clipboard::ClipboardType::Extension(_) => Err(clipboard::ClipboardError::NotSupported),
            _ => Err(clipboard::ClipboardError::NotSupported),
        }
    }

    #[cfg(not(any(windows, target_os = "android")))]
    fn write_clipboard(&mut self, data: clipboard::ClipboardData) -> Result<(), clipboard::ClipboardError> {
        match data {
            clipboard::ClipboardData::Text(t) => self.arboard()?.set_text(t).map_err(util::arboard_to_clip),
            clipboard::ClipboardData::Image(id) => {
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
                    Err(clipboard::ClipboardError::Other(zng_txt::Txt::from_static("image not found")))
                }
            }
            clipboard::ClipboardData::FileList(_) => Err(clipboard::ClipboardError::NotSupported),
            clipboard::ClipboardData::Extension { .. } => Err(clipboard::ClipboardError::NotSupported),
            _ => Err(clipboard::ClipboardError::NotSupported),
        }
    }

    #[cfg(target_os = "android")]
    fn read_clipboard(&mut self, data_type: clipboard::ClipboardType) -> Result<clipboard::ClipboardData, clipboard::ClipboardError> {
        let _ = data_type;
        Err(clipboard::ClipboardError::Other(Txt::from_static(
            "clipboard not implemented for Android",
        )))
    }

    #[cfg(target_os = "android")]
    fn write_clipboard(&mut self, data: clipboard::ClipboardData) -> Result<(), clipboard::ClipboardError> {
        let _ = data;
        Err(clipboard::ClipboardError::Other(Txt::from_static(
            "clipboard not implemented for Android",
        )))
    }

    fn start_drag_drop(
        &mut self,
        id: WindowId,
        data: Vec<DragDropData>,
        allowed_effects: DragDropEffect,
    ) -> Result<DragDropId, DragDropError> {
        let _ = (id, data, allowed_effects); // TODO, wait winit
        Err(DragDropError::NotSupported)
    }

    fn cancel_drag_drop(&mut self, id: WindowId, drag_id: DragDropId) {
        let _ = (id, drag_id);
    }

    fn drag_dropped(&mut self, id: WindowId, drop_id: DragDropId, applied: DragDropEffect) {
        let _ = (id, drop_id, applied); // TODO, wait winit
    }

    fn set_system_shutdown_warn(&mut self, id: WindowId, reason: Txt) {
        self.with_window(id, move |w| w.set_system_shutdown_warn(reason), || ())
    }

    fn third_party_licenses(&mut self) -> Vec<zng_tp_licenses::LicenseUsed> {
        #[cfg(feature = "bundle_licenses")]
        {
            zng_tp_licenses::include_bundle!()
        }
        #[cfg(not(feature = "bundle_licenses"))]
        {
            vec![]
        }
    }

    fn app_extension(&mut self, extension_id: ApiExtensionId, extension_request: ApiExtensionPayload) -> ApiExtensionPayload {
        self.exts.call_command(extension_id, extension_request)
    }

    fn window_extension(
        &mut self,
        id: WindowId,
        extension_id: ApiExtensionId,
        extension_request: ApiExtensionPayload,
    ) -> ApiExtensionPayload {
        self.with_window(
            id,
            |w| w.window_extension(extension_id, extension_request),
            || ApiExtensionPayload::invalid_request(extension_id, "window not found"),
        )
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

    fn ping(&mut self, count: u16) -> u16 {
        self.notify(Event::Pong(count));
        count
    }
}

/// Message inserted in the event loop from the view-process.
#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
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

    /// Enable disable winit device events.
    SetDeviceEventsFilter(DeviceEventsFilter),

    /// Send when monitor was turned on/off by the OS, need to redraw all screens to avoid blank issue.
    #[allow(unused)]
    MonitorPowerChanged,
}

/// Message inserted in the request loop from the view-process.
///
/// These *events* are detached from [`AppEvent`] so that we can continue receiving requests while
/// the main loop is blocked in a resize operation.
#[allow(clippy::large_enum_variant)] // Request is the largest, but also most common
#[derive(Debug)]
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

/// Abstraction over channel senders that can inject [`AppEvent`] in the app loop.
#[derive(Clone)]
pub(crate) enum AppEventSender {
    Headed(EventLoopProxy<AppEvent>, flume::Sender<RequestEvent>),
    Headless(flume::Sender<AppEvent>, flume::Sender<RequestEvent>),
}
impl AppEventSender {
    /// Send an event.
    fn send(&self, ev: AppEvent) -> Result<(), ipc::ViewChannelError> {
        match self {
            AppEventSender::Headed(p, _) => p.send_event(ev).map_err(|_| ipc::ViewChannelError::Disconnected),
            AppEventSender::Headless(p, _) => p.send(ev).map_err(|_| ipc::ViewChannelError::Disconnected),
        }
    }

    /// Send a request.
    fn request(&self, req: Request) -> Result<(), ipc::ViewChannelError> {
        match self {
            AppEventSender::Headed(_, p) => p.send(RequestEvent::Request(req)).map_err(|_| ipc::ViewChannelError::Disconnected),
            AppEventSender::Headless(_, p) => p.send(RequestEvent::Request(req)).map_err(|_| ipc::ViewChannelError::Disconnected),
        }?;
        self.send(AppEvent::Request)
    }

    /// Send a frame-ready.
    fn frame_ready(&self, window_id: WindowId, msg: FrameReadyMsg) -> Result<(), ipc::ViewChannelError> {
        match self {
            AppEventSender::Headed(_, p) => p
                .send(RequestEvent::FrameReady(window_id, msg))
                .map_err(|_| ipc::ViewChannelError::Disconnected),
            AppEventSender::Headless(_, p) => p
                .send(RequestEvent::FrameReady(window_id, msg))
                .map_err(|_| ipc::ViewChannelError::Disconnected),
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

    fn new_frame_ready(&self, _: DocumentId, _: FramePublishId, params: &FrameReadyParams) {
        // render is composite_needed (https://github.com/servo/webrender/commit/82860cfd6ebb012a009d639629eeb29078e2974f)
        let msg = FrameReadyMsg {
            composite_needed: params.render,
        };
        let _ = self.sender.frame_ready(self.id, msg);
    }
}

#[cfg(target_arch = "wasm32")]
compile_error!("zng-view does not support Wasm");
