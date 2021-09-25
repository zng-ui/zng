//! View-Process implementation that is headless only.
//!
//! Headed windows are also headless in this backend, system configuration is not retrieved
//! and most events never fire.
//!
//! This backend is recommended for command line apps that just render images, it can
//! also be used in other backends to provide headless windows and headless mode.
//!
//! # `webrender`
//!
//! The version of `webrender` used in this crate is re-exported as the `webrender` module.
//! This is useful for implementing other backends, so you use the same `webrender` version.

use std::{cell::Cell, fmt, rc::Rc, sync::Arc, time::Duration};

#[doc(inline)]
pub use webrender;

mod surface;
use surface::*;

use webrender::api::*;
use zero_ui_view_api::{units::*, *};

/// Starts the headless view-process server if called in the environment of a view-process.
///
/// Returns `false` if no view-process environment is setup, returns `true` if started and ran
/// a headless app until exit. Returns [`Disconnected`] if an environment was setup but lost connection
/// with the app client during the lifetime of the view-process.
///
/// # Panics
///
/// Panics if not called in the main thread, this is a requirement of OpenGL.
pub fn init() -> Result<bool, Disconnected> {
    if !is_main_thread::is_main_thread().unwrap_or(true) {
        panic!("only call `init` in the main thread, this is a requirement of OpenGL");
    }
    if let Some(config) = ViewConfig::from_thread().or_else(ViewConfig::from_env) {
        let mut c = connect_view_process(config.server_name)?;
        let mut app = HeadlessBackend::default();
        while !app.exited {
            let request = c.request_receiver.recv()?;
            let response = app.respond(request);
            c.response_sender.send(response)?;
        }
        Ok(true)
    } else {
        Ok(false)
    }
}

/// The backend implementation.
///
/// This type is public so it can be used as the "headless-mode" in other backends, to just
/// start the headless view-process use [`init`].
pub struct HeadlessBackend {
    started: bool,

    event_loop: glutin::event_loop::EventLoop<()>,
    gl_manager: GlContextManager,
    event_sender: flume::Sender<AppEvent>,
    event_receiver: flume::Receiver<AppEvent>,

    gen: ViewProcessGen,
    device_events: bool,

    surfaces: Vec<Surface>,

    surface_id_gen: WinId,

    exited: bool,
}
impl fmt::Debug for HeadlessBackend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HeadlessBackend")
            .field("started", &self.started)
            .field("gen", &self.gen)
            .field("device_events", &self.device_events)
            .field("surfaces", &self.surfaces)
            .finish_non_exhaustive()
    }
}
impl Default for HeadlessBackend {
    fn default() -> Self {
        let (event_sender, event_receiver) = flume::unbounded();
        HeadlessBackend {
            started: false,
            event_loop: glutin::event_loop::EventLoop::new(),
            gl_manager: GlContextManager::default(),
            event_sender,
            event_receiver,
            gen: 0,
            device_events: false,
            surfaces: vec![],
            surface_id_gen: 0,
            exited: false,
        }
    }
}
impl HeadlessBackend {
    /// Returns a clone of the event sender.
    pub fn event_sender(&self) -> flume::Sender<AppEvent> {
        self.event_sender.clone()
    }

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
impl Api for HeadlessBackend {
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
            &self.event_loop,
            &mut self.gl_manager,
            self.event_sender.clone(),
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
        panic!("HEADLESS CRASH")
    }
}

///
pub enum AppEvent {
    /// A request from the app-process.
    Request(Request),
    /// A frame is ready for redraw.
    FrameReady(WinId),
}

/// Manages the "current" `glutin` OpenGL context.
///
/// If this manager is in use all OpenGL contexts created in the process must be managed by it. For
/// this reason a "headed" context is also supported here, even thought this crate does not support
/// headed apps.
#[derive(Default)]
pub struct GlContextManager {
    current: Rc<Cell<Option<WinId>>>,
}
impl GlContextManager {
    /// Start managing a "headed" glutin context.
    ///
    /// See [`GlContext`] for details.
    pub fn manage_headed(&self, id: WinId, ctx: glutin::RawContext<glutin::NotCurrent>) -> GlContext {
        GlContext {
            id,
            ctx: Some(unsafe { ctx.treat_as_current() }),
            current: Rc::clone(&self.current),
        }
    }

    /// Start managing a headless glutin context.
    pub fn manage_headless(&self, id: WinId, ctx: glutin::Context<glutin::NotCurrent>) -> GlHeadlessContext {
        GlHeadlessContext {
            id,
            ctx: Some(unsafe { ctx.treat_as_current() }),
            current: Rc::clone(&self.current),
        }
    }
}

/// Managed headless Open-GL context.
pub struct GlHeadlessContext {
    id: WinId,
    ctx: Option<glutin::Context<glutin::PossiblyCurrent>>,
    current: Rc<Cell<Option<WinId>>>,
}
impl GlHeadlessContext {
    /// Gets the context as current.
    ///
    /// It can already be current or it is made current.
    pub fn make_current(&mut self) -> &mut glutin::Context<glutin::PossiblyCurrent> {
        let id = Some(self.id);
        if self.current.get() != id {
            self.current.set(id);
            let c = self.ctx.take().unwrap();
            // glutin docs says that calling `make_not_current` is not necessary and
            // that "If you call make_current on some context, you should call treat_as_not_current as soon
            // as possible on the previously current context."
            //
            // As far as the glutin code goes `treat_as_not_current` just changes the state tag, so we can call
            // `treat_as_not_current` just to get access to the `make_current` when we know it is not current
            // anymore, and just ignore the whole "current state tag" thing.
            let c = unsafe { c.treat_as_not_current().make_current() }.expect("failed to make current");
            self.ctx = Some(c);
        }
        self.ctx.as_mut().unwrap()
    }
}
impl Drop for GlHeadlessContext {
    fn drop(&mut self) {
        if self.current.get() == Some(self.id) {
            let _ = unsafe { self.ctx.take().unwrap().make_not_current() };
            self.current.set(None);
        } else {
            let _ = unsafe { self.ctx.take().unwrap().treat_as_not_current() };
        }
    }
}

/// Managed headed Open-GL context.
///
/// Note that headed apps are not supported by this crate, we just provide a headed context
/// because [`GlContextManager`] needs to manage all contexts in a process.
pub struct GlContext {
    id: WinId,
    ctx: Option<glutin::ContextWrapper<glutin::PossiblyCurrent, ()>>,
    current: Rc<Cell<Option<WinId>>>,
}
impl GlContext {
    /// Gets the context as current.
    ///
    /// It can already be current or it is made current.
    pub fn make_current(&mut self) -> &mut glutin::ContextWrapper<glutin::PossiblyCurrent, ()> {
        let id = Some(self.id);
        if self.current.get() != id {
            self.current.set(id);
            let c = self.ctx.take().unwrap();
            // glutin docs says that calling `make_not_current` is not necessary and
            // that "If you call make_current on some context, you should call treat_as_not_current as soon
            // as possible on the previously current context."
            //
            // As far as the glutin code goes `treat_as_not_current` just changes the state tag, so we can call
            // `treat_as_not_current` just to get access to the `make_current` when we know it is not current
            // anymore, and just ignore the whole "current state tag" thing.
            let c = unsafe { c.treat_as_not_current().make_current() }.expect("failed to make current");
            self.ctx = Some(c);
        }
        self.ctx.as_mut().unwrap()
    }

    /// Glutin requires that the context is [dropped before the window][1], calling this
    /// function safely disposes of the context, the winit window should be dropped immediately after.
    ///
    /// [1]: https://docs.rs/glutin/0.27.0/glutin/type.WindowedContext.html#method.split
    pub fn drop_before_winit(&mut self) {
        if self.current.get() == Some(self.id) {
            let _ = unsafe { self.ctx.take().unwrap().make_not_current() };
            self.current.set(None);
        } else {
            let _ = unsafe { self.ctx.take().unwrap().treat_as_not_current() };
        }
    }
}
impl Drop for GlContext {
    fn drop(&mut self) {
        if self.ctx.is_some() {
            panic!("call `drop_before_winit` before dropping")
        }
    }
}
