use std::{cell::Cell, error::Error, ffi::CString, fmt, mem, num::NonZeroU32, rc::Rc, thread};

use gleam::gl;
use glutin::{
    config::{Api, ConfigSurfaceTypes, ConfigTemplateBuilder},
    context::{ContextAttributesBuilder, PossiblyCurrentContext},
    display::{Display, DisplayApiPreference},
    prelude::*,
    surface::{Surface, SurfaceAttributesBuilder, WindowSurface},
};
use rustc_hash::FxHashSet;
use winit::{dpi::PhysicalSize, event_loop::ActiveEventLoop};
use zng_txt::ToTxt as _;
use zng_view_api::window::{RenderMode, WindowId};

use raw_window_handle::*;

use crate::{AppEvent, AppEventSender, util};

/// Create and track the current OpenGL context.
pub(crate) struct GlContextManager {
    current: Rc<Cell<Option<WindowId>>>,
    unsupported_headed: FxHashSet<TryConfig>,
    unsupported_headless: FxHashSet<TryConfig>,
}

impl Default for GlContextManager {
    fn default() -> Self {
        Self {
            current: Rc::new(Cell::new(None)),
            unsupported_headed: FxHashSet::default(),
            unsupported_headless: FxHashSet::default(),
        }
    }
}

#[allow(clippy::large_enum_variant)] // enum is temporary
enum GlWindowCreation {
    /// Windows requires this.
    Before(winit::window::Window),
    /// Other platforms don't. X11 requires this because it needs to set the XVisualID.
    After(winit::window::WindowAttributes),
}
fn winit_create_window(winit_loop: &ActiveEventLoop, window: &winit::window::WindowAttributes) -> winit::window::Window {
    let mut retries = 0;
    loop {
        match winit_loop.create_window(window.clone()) {
            Ok(w) => break w,
            Err(e) => {
                // Some platforms work after a retry
                // X11: After a GLXBadWindow
                retries += 1;
                if retries == 10 {
                    panic!("cannot create winit window, {e}")
                } else if retries > 1 {
                    tracing::error!("cannot create winit window (retry={retries}), {e}");
                    thread::sleep(std::time::Duration::from_millis(retries * 100));
                }
            }
        }
    }
}

impl GlContextManager {
    /// New window context.
    pub(crate) fn create_headed(
        &mut self,
        id: WindowId,
        window: winit::window::WindowAttributes,
        winit_loop: &ActiveEventLoop,
        render_mode: RenderMode,
        sender: &AppEventSender,
        prefer_egl: bool,
    ) -> (winit::window::Window, GlContext) {
        let mut errors = vec![];

        for config in TryConfig::iter(render_mode) {
            if self.unsupported_headed.contains(&config) {
                errors.push((config, "previous attempt failed, not supported".into()));
                continue;
            }

            let window = if cfg!(windows) || matches!(config.mode, RenderMode::Software) {
                GlWindowCreation::Before(winit_create_window(winit_loop, &window))
            } else {
                GlWindowCreation::After(window.clone())
            };

            let r = util::catch_suppress(std::panic::AssertUnwindSafe(|| match config.mode {
                RenderMode::Dedicated => self.create_headed_glutin(winit_loop, id, window, config.hardware_acceleration, prefer_egl),
                RenderMode::Integrated => self.create_headed_glutin(winit_loop, id, window, Some(false), prefer_egl),
                RenderMode::Software => self.create_headed_swgl(winit_loop, id, window),
            }));

            let error = match r {
                Ok(Ok(r)) => return r,
                Ok(Err(e)) => e,
                Err(panic) => {
                    let component = match config.mode {
                        RenderMode::Dedicated => "glutin (headed, dedicated)",
                        RenderMode::Integrated => "glutin (headed, integrated)",
                        RenderMode::Software => "swgl (headed)",
                    };
                    let _ = sender.send(AppEvent::Notify(zng_view_api::Event::RecoveredFromComponentPanic {
                        component: component.into(),
                        recover: "will try other modes".into(),
                        panic: panic.to_txt(),
                    }));
                    panic.msg.into()
                }
            };

            tracing::error!("[{}] {}", config.name(), error);
            errors.push((config, error));

            self.unsupported_headed.insert(config);
        }

        let mut msg = "failed to create headed open-gl context:\n".to_owned();
        for (config, error) in errors {
            use std::fmt::Write;
            writeln!(&mut msg, "  {:?}: {}", config.name(), error).unwrap();
        }

        panic!("{msg}")
    }

    /// New headless context.
    pub(crate) fn create_headless(
        &mut self,
        id: WindowId,
        winit_loop: &ActiveEventLoop,
        render_mode: RenderMode,
        sender: &AppEventSender,
        prefer_egl: bool,
    ) -> GlContext {
        let mut errors = vec![];

        for config in TryConfig::iter(render_mode) {
            if self.unsupported_headed.contains(&config) {
                errors.push((config, "previous attempt failed, not supported".into()));
                continue;
            }

            let r = util::catch_suppress(std::panic::AssertUnwindSafe(|| match config.mode {
                RenderMode::Dedicated => self.create_headless_glutin(id, winit_loop, config.hardware_acceleration, prefer_egl),
                RenderMode::Integrated => self.create_headless_glutin(id, winit_loop, Some(false), prefer_egl),
                RenderMode::Software => self.create_headless_swgl(id),
            }));

            let error = match r {
                Ok(Ok(ctx)) => return ctx,
                Ok(Err(e)) => e,
                Err(panic) => {
                    let component = match config.mode {
                        RenderMode::Dedicated => "glutin (headless, dedicated)",
                        RenderMode::Integrated => "glutin (headless, integrated)",
                        RenderMode::Software => "swgl (headless)",
                    };
                    let _ = sender.send(AppEvent::Notify(zng_view_api::Event::RecoveredFromComponentPanic {
                        component: component.into(),
                        recover: "will try other modes".into(),
                        panic: panic.to_txt(),
                    }));
                    panic.msg.into()
                }
            };

            tracing::error!("[{}] {}", config.name(), error);
            errors.push((config, error));

            self.unsupported_headless.insert(config);
        }

        let mut msg = "failed to create headless open-gl context:\n".to_owned();
        for (config, error) in errors {
            use std::fmt::Write;
            writeln!(&mut msg, "  {:?}: {}", config.name(), error).unwrap();
        }

        panic!("{msg}")
    }

    fn create_headed_glutin(
        &mut self,
        event_loop: &ActiveEventLoop,
        id: WindowId,
        window: GlWindowCreation,
        hardware: Option<bool>,
        prefer_egl: bool,
    ) -> Result<(winit::window::Window, GlContext), Box<dyn Error>> {
        #[cfg(windows)]
        let display_pref = {
            let handle = Some(match &window {
                GlWindowCreation::Before(w) => w.window_handle().unwrap().as_raw(),
                GlWindowCreation::After(_) => unreachable!(),
            });
            if prefer_egl {
                DisplayApiPreference::EglThenWgl(handle)
            } else {
                DisplayApiPreference::WglThenEgl(handle)
            }
        };

        #[cfg(any(
            target_os = "linux",
            target_os = "dragonfly",
            target_os = "freebsd",
            target_os = "openbsd",
            target_os = "netbsd",
        ))]
        let display_pref = {
            let handle = Box::new(winit::platform::x11::register_xlib_error_hook);
            if prefer_egl {
                DisplayApiPreference::EglThenGlx(handle)
            } else {
                DisplayApiPreference::GlxThenEgl(handle)
            }
        };

        #[cfg(target_os = "android")]
        let display_pref = DisplayApiPreference::Egl;

        #[cfg(target_os = "macos")]
        let display_pref = DisplayApiPreference::Cgl;

        let _ = prefer_egl;

        let display_handle = match &window {
            GlWindowCreation::Before(w) => w.display_handle().unwrap().as_raw(),
            GlWindowCreation::After(_) => event_loop.display_handle().unwrap().as_raw(),
        };

        // SAFETY: we are trusting the `raw_display_handle` from winit here.
        let display = unsafe { Display::new(display_handle, display_pref) }?;

        let mut template = ConfigTemplateBuilder::new()
            .with_alpha_size(8)
            .with_transparency(cfg!(not(target_os = "android")))
            .with_surface_type(ConfigSurfaceTypes::WINDOW)
            .prefer_hardware_accelerated(hardware);
        if let GlWindowCreation::Before(w) = &window {
            template = template.compatible_with_native_window(w.window_handle().unwrap().as_raw());
        }
        let template = template.build();

        // SAFETY: we are holding the `window` reference.
        let config = unsafe { display.find_configs(template)?.next().ok_or("no display config") }?;

        let window = match window {
            GlWindowCreation::Before(w) => w,
            GlWindowCreation::After(w) => {
                #[cfg(any(
                    target_os = "linux",
                    target_os = "dragonfly",
                    target_os = "freebsd",
                    target_os = "openbsd",
                    target_os = "netbsd"
                ))]
                let w = {
                    use glutin::platform::x11::X11GlConfigExt as _;
                    use winit::platform::x11::WindowAttributesExtX11 as _;

                    if let Some(id) = config.x11_visual() {
                        w.with_x11_visual(id.visual_id() as _)
                    } else {
                        w
                    }
                };
                winit_create_window(event_loop, &w)
            }
        };

        let window_handle = window.window_handle().unwrap().as_raw();

        let size = window.inner_size();
        let attrs = SurfaceAttributesBuilder::<WindowSurface>::new().build(
            window_handle,
            NonZeroU32::new(size.width).unwrap(),
            NonZeroU32::new(size.height).unwrap(),
        );

        // SAFETY: the window handle is valid.
        let surface = unsafe { display.create_window_surface(&config, &attrs)? };

        let context_attributes = ContextAttributesBuilder::new().build(Some(window_handle));
        // SAFETY: the window handle is valid.
        let context = unsafe { display.create_context(&config, &context_attributes)? };

        self.current.set(Some(id));
        let context = context.make_current(&surface)?;

        let gl_api = config.api();
        let gl = if gl_api.contains(Api::OPENGL) {
            // SAFETY: function pointers are directly from safe glutin here.
            unsafe {
                gl::GlFns::load_with(|symbol| {
                    let symbol = CString::new(symbol).unwrap();
                    display.get_proc_address(symbol.as_c_str())
                })
            }
        } else if gl_api.contains(Api::GLES3) {
            // SAFETY: function pointers are directly from safe glutin here.
            unsafe {
                gl::GlesFns::load_with(|symbol| {
                    let symbol = CString::new(symbol).unwrap();
                    display.get_proc_address(symbol.as_c_str())
                })
            }
        } else {
            return Err("no OpenGL or GLES3 available".into());
        };

        check_wr_gl_version(&*gl)?;

        #[cfg(debug_assertions)]
        let gl = gl::ErrorCheckingGl::wrap(gl.clone());

        let mut context = GlContext {
            id,
            current: self.current.clone(),
            backend: GlBackend::Glutin {
                context,
                surface,
                headless: None,
            },
            gl,

            render_mode: if hardware == Some(false) {
                RenderMode::Integrated
            } else {
                RenderMode::Dedicated
            },
        };

        context.resize(size);

        Ok((window, context))
    }

    fn create_headed_swgl(
        &mut self,
        event_loop: &ActiveEventLoop,
        id: WindowId,
        window: GlWindowCreation,
    ) -> Result<(winit::window::Window, GlContext), Box<dyn Error>> {
        #[cfg(not(feature = "software"))]
        {
            let _ = (id, window, event_loop);
            return Err("zng-view not build with \"software\" backend".into());
        }

        #[cfg(target_os = "android")]
        {
            let _ = (id, window, event_loop);
            return Err("software blit not implemented for Android".into());
        }

        #[cfg(all(feature = "software", not(target_os = "android")))]
        {
            let window = match window {
                GlWindowCreation::Before(w) => w,
                GlWindowCreation::After(w) => event_loop.create_window(w)?,
            };

            // SAFETY: softbuffer context is managed like gl context, it is dropped before the window is dropped.
            let static_window_ref = unsafe { mem::transmute::<&winit::window::Window, &'static winit::window::Window>(&window) };
            let blit_context = softbuffer::Context::new(static_window_ref)?;
            let blit_surface = softbuffer::Surface::new(&blit_context, static_window_ref)?;

            let context = swgl::Context::create();
            let gl = Rc::new(context);

            // create_headed_glutin returns as current.
            self.current.set(Some(id));
            context.make_current();

            let context = GlContext {
                id,
                current: self.current.clone(),
                backend: GlBackend::Swgl {
                    context,
                    blit: Some((blit_context, blit_surface)),
                },
                gl,
                render_mode: RenderMode::Software,
            };
            Ok((window, context))
        }
    }

    fn create_headless_glutin(
        &mut self,
        id: WindowId,
        winit_loop: &ActiveEventLoop,
        hardware: Option<bool>,
        prefer_egl: bool,
    ) -> Result<GlContext, Box<dyn Error>> {
        let hidden_window = winit::window::WindowAttributes::default()
            .with_transparent(true)
            .with_inner_size(PhysicalSize::new(1u32, 1u32))
            .with_visible(false)
            .with_decorations(false);
        let hidden_window = winit_loop.create_window(hidden_window)?;

        let display_handle = winit_loop.display_handle().unwrap().as_raw();
        let window_handle = hidden_window.window_handle().unwrap().as_raw();

        #[cfg(windows)]
        let display_pref = if prefer_egl {
            DisplayApiPreference::EglThenWgl(Some(window_handle))
        } else {
            DisplayApiPreference::WglThenEgl(Some(window_handle))
        };

        #[cfg(any(
            target_os = "linux",
            target_os = "dragonfly",
            target_os = "freebsd",
            target_os = "openbsd",
            target_os = "netbsd"
        ))]
        let display_pref = {
            let handle = Box::new(winit::platform::x11::register_xlib_error_hook);
            if prefer_egl {
                DisplayApiPreference::EglThenGlx(handle)
            } else {
                DisplayApiPreference::GlxThenEgl(handle)
            }
        };

        #[cfg(target_os = "android")]
        let display_pref = DisplayApiPreference::Egl;

        #[cfg(target_os = "macos")]
        let display_pref = DisplayApiPreference::Cgl;

        let _ = prefer_egl;

        // SAFETY: we are trusting the `raw_display_handle` from winit here.
        let display = unsafe { Display::new(display_handle, display_pref) }?;

        let template = ConfigTemplateBuilder::new()
            .with_alpha_size(8)
            .with_transparency(true)
            .compatible_with_native_window(window_handle)
            .with_surface_type(ConfigSurfaceTypes::WINDOW)
            .prefer_hardware_accelerated(hardware)
            .build();

        // SAFETY: we are holding the `window` reference.
        let config = unsafe { display.find_configs(template)?.next().ok_or("no display config") }?;

        let size = hidden_window.inner_size();
        let attrs = SurfaceAttributesBuilder::<WindowSurface>::new().build(
            window_handle,
            NonZeroU32::new(size.width).unwrap(),
            NonZeroU32::new(size.height).unwrap(),
        );

        // SAFETY: the window handle is valid.
        let surface = unsafe { display.create_window_surface(&config, &attrs)? };

        let context_attributes = ContextAttributesBuilder::new().build(Some(window_handle));
        // SAFETY: the window handle is valid.
        let context = unsafe { display.create_context(&config, &context_attributes)? };

        self.current.set(Some(id));
        let context = context.make_current(&surface)?;

        let gl_api = config.api();
        let gl = if gl_api.contains(Api::OPENGL) {
            // SAFETY: function pointers are directly from safe glutin here.
            unsafe {
                gl::GlFns::load_with(|symbol| {
                    let symbol = CString::new(symbol).unwrap();
                    display.get_proc_address(symbol.as_c_str())
                })
            }
        } else if gl_api.contains(Api::GLES3) {
            // SAFETY: function pointers are directly from safe glutin here.
            unsafe {
                gl::GlesFns::load_with(|symbol| {
                    let symbol = CString::new(symbol).unwrap();
                    display.get_proc_address(symbol.as_c_str())
                })
            }
        } else {
            return Err("no OpenGL or GLES3 available".into());
        };

        check_wr_gl_version(&*gl)?;

        #[cfg(debug_assertions)]
        let gl = gl::ErrorCheckingGl::wrap(gl.clone());

        let mut context = GlContext {
            id,
            current: self.current.clone(),
            backend: GlBackend::Glutin {
                context,
                surface,
                headless: Some(GlutinHeadless::new(&gl, hidden_window)),
            },
            gl,

            render_mode: if hardware == Some(false) {
                RenderMode::Integrated
            } else {
                RenderMode::Dedicated
            },
        };

        context.resize(size);

        Ok(context)
    }

    fn create_headless_swgl(&mut self, id: WindowId) -> Result<GlContext, Box<dyn Error>> {
        #[cfg(not(feature = "software"))]
        {
            let _ = id;
            return Err("zng-view not build with \"software\" backend".into());
        }

        #[cfg(feature = "software")]
        {
            let context = swgl::Context::create();
            let gl = Rc::new(context);

            // create_headless_glutin returns as current.
            self.current.set(Some(id));
            context.make_current();

            Ok(GlContext {
                id,
                current: self.current.clone(),
                backend: GlBackend::Swgl { context, blit: None },
                gl,
                render_mode: RenderMode::Software,
            })
        }
    }
}

#[allow(clippy::large_enum_variant)] // glutin is the largest, but also most common
enum GlBackend {
    Glutin {
        headless: Option<GlutinHeadless>,
        context: PossiblyCurrentContext,
        surface: Surface<WindowSurface>,
    },

    #[cfg(feature = "software")]
    Swgl {
        context: swgl::Context,
        // is None for headless.
        #[cfg(not(target_os = "android"))]
        blit: Option<(
            softbuffer::Context<&'static winit::window::Window>,
            softbuffer::Surface<&'static winit::window::Window, &'static winit::window::Window>,
        )>,
        #[cfg(target_os = "android")]
        blit: Option<((), ())>,
    },

    Dropped,
}

/// OpenGL context managed by [`GlContextManager`].
pub(crate) struct GlContext {
    id: WindowId,
    current: Rc<Cell<Option<WindowId>>>,

    backend: GlBackend,

    gl: Rc<dyn gl::Gl>,
    render_mode: RenderMode,
}
impl fmt::Debug for GlContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GlContext")
            .field("id", &self.id)
            .field("is_current", &self.is_current())
            .field("render_mode", &self.render_mode)
            .finish_non_exhaustive()
    }
}
impl GlContext {
    /// If the context is backed by SWGL.
    pub(crate) fn is_software(&self) -> bool {
        #[cfg(feature = "software")]
        {
            matches!(&self.backend, GlBackend::Swgl { .. })
        }
        #[cfg(not(feature = "software"))]
        {
            false
        }
    }

    pub fn is_current(&self) -> bool {
        Some(self.id) == self.current.get()
    }

    pub(crate) fn gl(&self) -> &Rc<dyn gl::Gl> {
        &self.gl
    }

    pub(crate) fn render_mode(&self) -> RenderMode {
        self.render_mode
    }

    pub(crate) fn resize(&mut self, size: PhysicalSize<u32>) {
        assert!(self.is_current());

        match &mut self.backend {
            GlBackend::Glutin {
                context,
                surface,
                headless,
            } => {
                if let Some(h) = headless {
                    h.resize(&self.gl, size.width as _, size.height as _);
                } else {
                    let width = NonZeroU32::new(size.width.max(1)).unwrap();
                    let height = NonZeroU32::new(size.height.max(1)).unwrap();
                    surface.resize(context, width, height);
                }
            }
            #[cfg(feature = "software")]
            GlBackend::Swgl { context, blit } => {
                // NULL means SWGL manages the buffer, it also retains the buffer if the size did not change.
                let w = size.width.max(1);
                let h = size.height.max(1);
                context.init_default_framebuffer(0, 0, w as i32, h as i32, 0, std::ptr::null_mut());

                #[cfg(not(target_os = "android"))]
                if let Some((_, surface)) = blit {
                    surface.resize(NonZeroU32::new(w).unwrap(), NonZeroU32::new(h).unwrap()).unwrap();
                }

                #[cfg(target_os = "android")]
                let _ = blit;
            }
            GlBackend::Dropped => unreachable!(),
        }
    }

    pub(crate) fn make_current(&mut self) {
        let id = Some(self.id);
        if self.current.get() != id {
            self.current.set(id);

            match &self.backend {
                GlBackend::Glutin { context, surface, .. } => context.make_current(surface).unwrap(),
                #[cfg(feature = "software")]
                GlBackend::Swgl { context, .. } => context.make_current(),
                GlBackend::Dropped => unreachable!(),
            }
        }
    }

    pub(crate) fn swap_buffers(&mut self) {
        assert!(self.is_current());

        match &mut self.backend {
            GlBackend::Glutin {
                context,
                surface,
                headless,
            } => {
                if headless.is_none() {
                    surface.swap_buffers(context).unwrap()
                }
            }
            #[cfg(feature = "software")]
            GlBackend::Swgl { context, blit } => {
                #[cfg(target_os = "android")]
                let _ = (context, blit);

                #[cfg(not(target_os = "android"))]
                if let Some((_, blit_surface)) = blit {
                    gl::Gl::finish(context);
                    let (data_ptr, w, h, stride) = context.get_color_buffer(0, true);

                    if w == 0 || h == 0 {
                        return;
                    }

                    // SAFETY: we trust SWGL
                    assert!(stride == w * 4);
                    let frame = unsafe { std::slice::from_raw_parts(data_ptr as *const u8, w as usize * h as usize * 4) };
                    // bgra, max_y=0
                    let frame = frame.chunks_exact(stride as _).rev().flat_map(|row| row.chunks_exact(4));
                    let mut buffer = blit_surface.buffer_mut().unwrap();
                    for (argb, bgra) in buffer.iter_mut().zip(frame) {
                        let blue = bgra[0] as u32;
                        let green = bgra[1] as u32;
                        let red = bgra[2] as u32;
                        let alpha = bgra[3] as u32;
                        *argb = blue | (green << 8) | (red << 16) | (alpha << 24);
                    }

                    buffer.present().unwrap();
                }
            }
            GlBackend::Dropped => unreachable!(),
        }
    }
}
impl Drop for GlContext {
    fn drop(&mut self) {
        self.make_current();

        match mem::replace(&mut self.backend, GlBackend::Dropped) {
            GlBackend::Glutin { headless, .. } => {
                if let Some(h) = headless {
                    let _ = h.hidden_window;

                    h.destroy(&self.gl);
                }
            }
            #[cfg(feature = "software")]
            GlBackend::Swgl { context, .. } => context.destroy(),
            GlBackend::Dropped => unreachable!(),
        }
    }
}

/// Warmup the OpenGL driver in a throwaway thread, some NVIDIA drivers have a slow startup (500ms~),
/// hopefully this loads it in parallel while the app is starting up so we don't block creating the first window.
#[cfg(windows)]
pub(crate) fn warmup() {
    // idea copied from here:
    // https://hero.handmade.network/forums/code-discussion/t/2503-day_235_opengl%2527s_pixel_format_takes_a_long_time#13029

    use windows_sys::Win32::Graphics::{
        Gdi::*,
        OpenGL::{self},
    };

    let _ = std::thread::Builder::new()
        .name("warmup".to_owned())
        .stack_size(3 * 64 * 1024)
        .spawn(|| unsafe {
            let _span = tracing::trace_span!("open-gl-init").entered();
            let hdc = GetDC(0);
            let _ = OpenGL::DescribePixelFormat(hdc, 0, 0, std::ptr::null_mut());
            ReleaseDC(0, hdc);
        });
}

#[cfg(not(windows))]
pub(crate) fn warmup() {}

// check if equal or newer then 3.1
fn check_wr_gl_version(gl: &dyn gl::Gl) -> Result<(), String> {
    let mut version = [0; 2];
    let is_2_or_1;
    // SAFETY: get_integer_v API available in all impls
    unsafe {
        gl.get_integer_v(gl::MAJOR_VERSION, &mut version[..1]);
        is_2_or_1 = gl.get_error() == gl::INVALID_ENUM; // MAJOR_VERSION is only 3.0 and above
        gl.get_integer_v(gl::MINOR_VERSION, &mut version[1..]);
    };

    if !is_2_or_1 && version[0] >= 3 {
        let min_minor = match gl.get_type() {
            gl::GlType::Gl => 1,
            gl::GlType::Gles => 0,
        };
        if version[1] >= min_minor {
            return Ok(());
        }
    }
    Err(format!(
        "webrender requires OpenGL >=3.1 or OpenGL ES >=3.0, found {}",
        gl.get_string(gl::VERSION)
    ))
}

/// Glutin, SWGL config to attempt.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
struct TryConfig {
    mode: RenderMode,
    hardware_acceleration: Option<bool>,
}
impl TryConfig {
    fn iter(mode: RenderMode) -> impl Iterator<Item = TryConfig> {
        let mut configs = Vec::with_capacity(4);
        let mut try_hardware_none = false;
        for mode in mode.with_fallbacks() {
            match mode {
                RenderMode::Dedicated => {
                    configs.push(TryConfig {
                        mode,
                        hardware_acceleration: Some(true),
                    });
                    try_hardware_none = true;
                }
                RenderMode::Integrated => configs.push(TryConfig {
                    mode,
                    hardware_acceleration: Some(false),
                }),
                RenderMode::Software => {
                    if mem::take(&mut try_hardware_none) {
                        // some dedicated hardware end-up classified as generic integrated for some reason,
                        // so we try `None`, after `Some(false)` and before `Software`.
                        configs.push(TryConfig {
                            mode: RenderMode::Dedicated,
                            hardware_acceleration: None,
                        });
                    }
                    configs.push(TryConfig {
                        mode,
                        hardware_acceleration: Some(false),
                    });
                }
            }
        }
        configs.into_iter()
    }

    pub fn name(&self) -> &str {
        match self.hardware_acceleration {
            Some(true) => "Dedicated",
            Some(false) => match self.mode {
                RenderMode::Integrated => "Integrated",
                RenderMode::Software => "Software",
                RenderMode::Dedicated => unreachable!(),
            },
            None => "Dedicated (generic)",
        }
    }
}

struct GlutinHeadless {
    hidden_window: winit::window::Window,

    // actual surface.
    rbos: [u32; 2],
    fbo: u32,
}
impl GlutinHeadless {
    fn new(gl: &Rc<dyn gl::Gl>, hidden_window: winit::window::Window) -> Self {
        // create a surface for Webrender:

        let rbos = gl.gen_renderbuffers(2);

        let rbos = [rbos[0], rbos[1]];
        let fbo = gl.gen_framebuffers(1)[0];

        gl.bind_renderbuffer(gl::RENDERBUFFER, rbos[0]);
        gl.renderbuffer_storage(gl::RENDERBUFFER, gl::RGBA8, 1, 1);

        gl.bind_renderbuffer(gl::RENDERBUFFER, rbos[1]);
        gl.renderbuffer_storage(gl::RENDERBUFFER, gl::DEPTH24_STENCIL8, 1, 1);

        gl.viewport(0, 0, 1, 1);

        gl.bind_framebuffer(gl::FRAMEBUFFER, fbo);
        gl.framebuffer_renderbuffer(gl::FRAMEBUFFER, gl::COLOR_ATTACHMENT0, gl::RENDERBUFFER, rbos[0]);
        gl.framebuffer_renderbuffer(gl::FRAMEBUFFER, gl::DEPTH_STENCIL_ATTACHMENT, gl::RENDERBUFFER, rbos[1]);

        GlutinHeadless { hidden_window, rbos, fbo }
    }

    fn resize(&self, gl: &Rc<dyn gl::Gl>, width: i32, height: i32) {
        gl.bind_renderbuffer(gl::RENDERBUFFER, self.rbos[0]);
        gl.renderbuffer_storage(gl::RENDERBUFFER, gl::RGBA8, width, height);

        gl.bind_renderbuffer(gl::RENDERBUFFER, self.rbos[1]);
        gl.renderbuffer_storage(gl::RENDERBUFFER, gl::DEPTH24_STENCIL8, width, height);

        gl.viewport(0, 0, width, height);
    }

    fn destroy(self, gl: &Rc<dyn gl::Gl>) {
        gl.delete_framebuffers(&[self.fbo]);
        gl.delete_renderbuffers(&self.rbos);
    }
}
