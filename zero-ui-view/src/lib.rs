//! View-Process implementation using [`glutin`].
//!
//! This backend supports both headed and headless apps
//!
//! # Examples
//!
//! Call [`init`] before any other code in `main` to setup a view-process that uses
//! the same app executable:
//!
//! ```
//! # pub mod zero_ui { pub mod prelude {
//! # pub struct App { } impl App { fn default() -> Self { todo!() }
//! # fn run_window(self, f: impl FnOnce(bool)) } } }
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

use std::{cell::Cell, fmt, process, rc::Rc, sync::Arc, thread, time::Duration};

use glutin::{
    event::{DeviceEvent, WindowEvent},
    event_loop::{ControlFlow, EventLoop, EventLoopProxy, EventLoopWindowTarget},
};
use util::GlContextManager;
#[doc(inline)]
pub use webrender;

mod config;
mod surface;
mod util;
use surface::*;

use webrender::api::*;
use zero_ui_view_api::{units::*, *};

/// Runs the view-process server if called in the environment of a view-process.
///
/// If this function is called in a process not configured to be a view-process it will return
/// immediately, with the expectation that the app will be started. If called in a view-process
/// if will highjack the process **never returning**.
///
/// # Examples
///
/// ```
/// # pub mod zero_ui { pub mod prelude {
/// # pub struct App { } impl App { fn default() -> Self { todo!() }
/// # fn run_window(self, f: impl FnOnce(bool)) } } }
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

    if let Some(config) = ViewConfig::from_thread().or_else(ViewConfig::from_env) {
        let c = connect_view_process(config.server_name).expect("failed to connect to app-process");

        if config.headless {
            App::run_headless(c);
        } else {
            App::run_headed(c);
        }
    }
}

/// The backend implementation.
pub(crate) struct App<S> {
    started: bool,

    headless: bool,

    gl_manager: GlContextManager,
    window_target: *const EventLoopWindowTarget<AppEvent>,
    app_sender: S,

    gen: ViewProcessGen,
    device_events: bool,

    surfaces: Vec<Surface>,

    surface_id_gen: WinId,

    exited: bool,
}
impl<S> fmt::Debug for App<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HeadlessBackend")
            .field("started", &self.started)
            .field("gen", &self.gen)
            .field("device_events", &self.device_events)
            .field("surfaces", &self.surfaces)
            .finish_non_exhaustive()
    }
}
impl App<()> {
    pub fn run_headless(c: ViewChannels) -> ! {
        let (app_sender, app_receiver) = flume::unbounded();
        let mut app = App::new(app_sender);
        app.headless = true;
        let event_loop = EventLoop::<AppEvent>::with_user_event();
        let window_target: &EventLoopWindowTarget<AppEvent> = &event_loop;
        app.window_target = window_target as *const _;
        app.start_receiving(c.request_receiver);

        let mut response_sender = c.response_sender;
        let mut event_sender = c.event_sender;
        while !app.exited {
            match app_receiver.recv() {
                Ok(app_ev) => match app_ev {
                    AppEvent::Request(request) => {
                        let response = app.respond(request);
                        if response_sender.send(response).is_err() {
                            break;
                        }
                    }
                    AppEvent::FrameReady(id) => {
                        if let Some(s) = app.surfaces.iter_mut().find(|s| s.id() == id) {
                            s.redraw();
                        }
                    }
                    AppEvent::Notify(ev) => {
                        event_sender.send(ev);
                    }
                    AppEvent::RefreshMonitors => {
                        panic!("no monitor info in headless mode")
                    }
                },
                Err(_) => break,
            }
        }

        process::exit(0)
    }

    pub fn run_headed(c: ViewChannels) -> ! {
        let event_loop = EventLoop::with_user_event();
        let app_sender = event_loop.create_proxy();
        let mut app = App::new(app_sender);
        app.start_receiving(c.request_receiver);

        #[cfg(windows)]
        let config_listener = config::config_listener(app.app_sender.clone_(), &event_loop);

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
                        AppEvent::Request(_) => todo!(),
                        AppEvent::Notify(_) => todo!(),
                        AppEvent::FrameReady(_) => todo!(),
                        AppEvent::RefreshMonitors => todo!(),
                    },
                    GEvent::Suspended => {}
                    GEvent::Resumed => {}
                    GEvent::MainEventsCleared => app.on_events_cleared(),
                    GEvent::RedrawRequested(w_id) => app.on_redraw_requested(w_id),
                    GEvent::RedrawEventsCleared => {}
                    GEvent::LoopDestroyed => {}
                }
            }

            app.window_target = std::ptr::null();
        })
    }
}
impl<S: AppEventSender> App<S> {
    fn new(app_sender: S) -> Self {
        App {
            headless: false,
            started: false,
            gl_manager: GlContextManager::default(),
            app_sender,
            window_target: std::ptr::null(),
            gen: 0,
            device_events: false,
            surfaces: vec![],
            surface_id_gen: 0,
            exited: false,
        }
    }

    fn start_receiving(&mut self, mut request_recv: RequestReceiver) {
        let mut app_sender = self.app_sender.clone_();
        thread::spawn(move || {
            while let Ok(r) = request_recv.recv() {
                if app_sender.send(AppEvent::Request(r)).is_err() {
                    break;
                }
            }
        });
    }

    fn on_window_event(&mut self, window_id: glutin::window::WindowId, event: WindowEvent) {}

    fn on_device_event(&mut self, device_id: glutin::event::DeviceId, event: DeviceEvent) {}

    fn on_events_cleared(&mut self) {}

    fn on_redraw_requested(&mut self, window_id: glutin::window::WindowId) {}

    fn assert_started(&self) {
        if !self.started {
            panic!("not started")
        }
    }

    fn generate_win_id(&mut self) -> WinId {
        self.surface_id_gen = self.surface_id_gen.wrapping_add(1);
        if self.surface_id_gen == 0 {
            self.surface_id_gen = 1;
        }
        self.surface_id_gen
    }

    fn with_surface<R>(&mut self, id: WinId, action: impl FnOnce(&mut Surface) -> R, not_found: impl FnOnce() -> R) -> R {
        self.assert_started();
        self.surfaces.iter_mut().find(|w| w.id() == id).map(action).unwrap_or_else(|| {
            log::error!("window `{}` not found, will return fallback result", id);
            not_found()
        })
    }
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
        if !headless {
            log::warn!("only headless is supported, headed windows will also be headless in this backend");
        }
    }

    fn exit(&mut self) {
        self.assert_started();
        self.started = false;
        self.exited = true;
    }

    fn primary_monitor(&mut self) -> Option<(MonId, MonitorInfo)> {
        self.assert_started();
        None
    }

    fn monitor_info(&mut self, _: MonId) -> Option<MonitorInfo> {
        self.assert_started();
        None
    }

    fn available_monitors(&mut self) -> Vec<(MonId, MonitorInfo)> {
        self.assert_started();
        vec![]
    }

    fn open_window(&mut self, config: WindowConfig) -> (WinId, webrender_api::IdNamespace, webrender_api::PipelineId) {
        self.open_headless(HeadlessConfig {
            scale_factor: 1.0,
            size: config.size,
            text_aa: config.text_aa,
        })
    }

    fn open_headless(&mut self, config: HeadlessConfig) -> (WinId, webrender_api::IdNamespace, webrender_api::PipelineId) {
        self.assert_started();
        let id = self.generate_win_id();

        let surf = Surface::open(
            id,
            self.gen,
            config,
            unsafe { &*self.window_target },
            &mut self.gl_manager,
            self.app_sender.clone_(),
        );
        let namespace = surf.namespace_id();
        let pipeline = surf.pipeline_id();

        self.surfaces.push(surf);

        (id, namespace, pipeline)
    }

    fn close_window(&mut self, id: WinId) {
        if let Some(i) = self.surfaces.iter().position(|w| w.id() == id) {
            let _ = self.surfaces.swap_remove(i);
        } else {
            log::error!("tried to close unkown window `{}`", id)
        }
    }

    fn text_aa(&mut self) -> TextAntiAliasing {
        self.assert_started();
        TextAntiAliasing::Default
    }

    fn multi_click_config(&mut self) -> MultiClickConfig {
        self.assert_started();
        MultiClickConfig::default()
    }

    fn animation_enabled(&mut self) -> bool {
        self.assert_started();
        true
    }

    fn key_repeat_delay(&mut self) -> Duration {
        self.assert_started();
        Duration::ZERO
    }

    fn set_title(&mut self, id: WinId, title: String) {
        self.with_surface(id, |_| log::warn!("ignoring `set_title({}, {:?})`", id, title), || ());
    }

    fn set_visible(&mut self, id: WinId, visible: bool) {
        self.with_surface(id, |_| log::warn!("ignoring `set_visible({}, {:?})`", id, visible), || ());
    }

    fn set_always_on_top(&mut self, id: WinId, always_on_top: bool) {
        self.with_surface(
            id,
            |_| log::warn!("ignoring `set_always_on_top({}, {:?})`", id, always_on_top),
            || (),
        );
    }

    fn set_movable(&mut self, id: WinId, movable: bool) {
        self.with_surface(id, |_| log::warn!("ignoring `set_movable({}, {:?})`", id, movable), || ());
    }

    fn set_resizable(&mut self, id: WinId, resizable: bool) {
        self.with_surface(id, |_| log::warn!("ignoring `set_resizable({}, {:?})`", id, resizable), || ());
    }

    fn set_taskbar_visible(&mut self, id: WinId, visible: bool) {
        self.with_surface(id, |_| log::warn!("ignoring `set_taskbar_visible({}, {:?})`", id, visible), || ());
    }

    fn set_parent(&mut self, id: WinId, parent: Option<WinId>, modal: bool) {
        self.with_surface(id, |_| log::warn!("ignoring `set_parent({}, {:?}, {})`", id, parent, modal), || ());
    }

    fn set_transparent(&mut self, id: WinId, transparent: bool) {
        self.with_surface(id, |w| w.set_transparent(transparent), || ());
    }

    fn set_chrome_visible(&mut self, id: WinId, visible: bool) {
        self.with_surface(id, |_| log::warn!("ignoring `set_taskbar_visible({}, {:?})`", id, visible), || ());
    }

    fn set_position(&mut self, id: WinId, pos: DipPoint) {
        self.with_surface(id, |_| log::warn!("ignoring `set_position({}, {:?})`", id, pos), || ());
    }

    fn set_size(&mut self, id: WinId, size: DipSize, frame: FrameRequest) {
        self.with_surface(
            id,
            |w| {
                w.set_size(size, w.scale_factor());
                w.render(frame);
            },
            || (),
        );
    }

    fn set_state(&mut self, id: WinId, state: WindowState) {
        self.with_surface(id, |_| log::warn!("ignoring `set_state({}, {:?})`", id, state), || ());
    }

    fn set_headless_size(&mut self, id: WinId, size: DipSize, scale_factor: f32) {
        self.with_surface(
            id,
            |w| {
                w.set_size(size, scale_factor);
            },
            || (),
        )
    }

    fn set_min_size(&mut self, id: WinId, size: DipSize) {
        self.with_surface(id, |_| log::warn!("ignoring `set_min_size({}, {:?})`", id, size), || ());
    }

    fn set_max_size(&mut self, id: WinId, size: DipSize) {
        self.with_surface(id, |_| log::warn!("ignoring `set_max_size({}, {:?})`", id, size), || ());
    }

    fn set_icon(&mut self, id: WinId, icon: Option<Icon>) {
        self.with_surface(id, |_| log::warn!("ignoring `set_icon({}, {:?})`", id, icon), || ());
    }

    fn pipeline_id(&mut self, id: WinId) -> PipelineId {
        self.with_surface(id, |w| w.pipeline_id(), PipelineId::dummy)
    }

    fn namespace_id(&mut self, id: WinId) -> IdNamespace {
        self.with_surface(id, |w| w.namespace_id(), || IdNamespace(0))
    }

    fn add_image(&mut self, id: WinId, descriptor: ImageDescriptor, data: ByteBuf) -> ImageKey {
        self.with_surface(id, |w| w.add_image(descriptor, Arc::new(data.to_vec())), || ImageKey::DUMMY)
    }

    fn update_image(&mut self, id: WinId, key: ImageKey, descriptor: ImageDescriptor, data: ByteBuf) {
        self.with_surface(id, |w| w.update_image(key, descriptor, Arc::new(data.to_vec())), || ())
    }

    fn delete_image(&mut self, id: WinId, key: ImageKey) {
        self.with_surface(id, |w| w.delete_image(key), || ())
    }

    fn add_font(&mut self, id: WinId, bytes: ByteBuf, index: u32) -> FontKey {
        self.with_surface(id, |w| w.add_font(bytes.to_vec(), index), || FontKey(IdNamespace(0), 0))
    }

    fn delete_font(&mut self, id: WinId, key: FontKey) {
        self.with_surface(id, |w| w.delete_font(key), || ())
    }

    fn add_font_instance(
        &mut self,
        id: WinId,
        font_key: FontKey,
        glyph_size: Px,
        options: Option<FontInstanceOptions>,
        plataform_options: Option<FontInstancePlatformOptions>,
        variations: Vec<FontVariation>,
    ) -> FontInstanceKey {
        self.with_surface(
            id,
            |w| w.add_font_instance(font_key, glyph_size, options, plataform_options, variations),
            || FontInstanceKey(IdNamespace(0), 0),
        )
    }

    fn delete_font_instance(&mut self, id: WinId, instance_key: FontInstanceKey) {
        self.with_surface(id, |w| w.delete_font_instance(instance_key), || ())
    }

    fn size(&mut self, id: WinId) -> DipSize {
        self.with_surface(id, |w| w.size(), DipSize::zero)
    }

    fn set_allow_alt_f4(&mut self, id: WinId, allow: bool) {
        self.with_surface(id, |_| log::warn!("ignoring `set_allow_alt_f4({}, {:?})`", id, allow), || ())
    }

    fn read_pixels(&mut self, id: WinId) -> FramePixels {
        self.with_surface(id, |w| w.read_pixels(), FramePixels::default)
    }

    fn read_pixels_rect(&mut self, id: WinId, rect: PxRect) -> FramePixels {
        self.with_surface(id, |w| w.read_pixels_rect(rect), FramePixels::default)
    }

    fn hit_test(&mut self, id: WinId, point: PxPoint) -> (Epoch, HitTestResult) {
        self.with_surface(id, |w| w.hit_test(point), || (Epoch(0), HitTestResult::default()))
    }

    fn set_text_aa(&mut self, id: WinId, aa: TextAntiAliasing) {
        self.with_surface(id, |w| w.set_text_aa(aa), || ())
    }

    fn render(&mut self, id: WinId, frame: FrameRequest) {
        self.with_surface(id, |w| w.render(frame), || ())
    }

    fn render_update(&mut self, id: WinId, updates: DynamicProperties, clear_color: Option<ColorF>) {
        self.with_surface(id, |w| w.render_update(updates, clear_color), || ())
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
    FrameReady(WinId),
    /// Re-query available monitors and send update event.
    RefreshMonitors,
}

/// Abstraction over channel senders  that can inject [`AppEvent`] in the app loop.
pub(crate) trait AppEventSender: Send + 'static {
    /// Send an event.
    fn send(&self, ev: AppEvent) -> Result<(), Disconnected>;

    /// Clone the sender.
    fn clone_(&self) -> Self
    where
        Self: Sized;

    /// Clone the sender !Sized.
    fn clone_boxed(&self) -> Box<dyn AppEventSender>;
}
/// headless
impl AppEventSender for flume::Sender<AppEvent> {
    fn send(&self, ev: AppEvent) -> Result<(), Disconnected> {
        self.send(ev).map_err(|_| Disconnected)
    }

    fn clone_(&self) -> Self
    where
        Self: Sized,
    {
        std::clone::Clone::clone(self)
    }

    fn clone_boxed(&self) -> Box<dyn AppEventSender> {
        Box::new(std::clone::Clone::clone(self))
    }
}
/// headed
impl AppEventSender for EventLoopProxy<AppEvent> {
    fn send(&self, ev: AppEvent) -> Result<(), Disconnected> {
        self.send_event(ev).map_err(|_| Disconnected)
    }

    fn clone_(&self) -> Self
    where
        Self: Sized,
    {
        std::clone::Clone::clone(self)
    }

    fn clone_boxed(&self) -> Box<dyn AppEventSender> {
        Box::new(std::clone::Clone::clone(self))
    }
}
