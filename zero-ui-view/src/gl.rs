use std::{cell::Cell, error::Error, ffi::CString, fmt, mem, num::NonZeroU32, rc::Rc};

use gleam::gl;
use glutin::{
    config::{Api, ConfigSurfaceTypes, ConfigTemplateBuilder},
    context::{ContextAttributesBuilder, PossiblyCurrentContext},
    display::{Display, DisplayApiPreference},
    prelude::*,
    surface::{Surface, SurfaceAttributesBuilder, WindowSurface},
};
use linear_map::set::LinearSet;
use winit::{dpi::PhysicalSize, event_loop::EventLoopWindowTarget};
use zero_ui_view_api::{RenderMode, WindowId};

#[cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd"
))]
use winit::platform::unix;

use raw_window_handle::*;

use crate::{util, AppEvent};

/// Create and track the current OpenGL context.
pub(crate) struct GlContextManager {
    current: Rc<Cell<Option<WindowId>>>,
    unsupported_headed: LinearSet<TryConfig>,
    unsupported_headless: LinearSet<TryConfig>,
}

impl Default for GlContextManager {
    fn default() -> Self {
        Self {
            current: Rc::new(Cell::new(None)),
            unsupported_headed: LinearSet::new(),
            unsupported_headless: LinearSet::new(),
        }
    }
}

impl GlContextManager {
    /// New window context.
    pub(crate) fn create_headed(
        &mut self,
        id: WindowId,
        window: winit::window::WindowBuilder,
        window_target: &EventLoopWindowTarget<AppEvent>,
        render_mode: RenderMode,
    ) -> (winit::window::Window, GlContext) {
        let mut errors = vec![];

        for config in TryConfig::iter(render_mode) {
            if self.unsupported_headed.contains(&config) {
                errors.push((config, "previous attempt failed, not supported".into()));
                continue;
            }

            let window = window.clone().build(window_target).unwrap();

            let r = util::catch_supress(std::panic::AssertUnwindSafe(|| match config.mode {
                RenderMode::Dedicated => self.create_headed_glutin(id, &window, window_target, config.hardware_acceleration),
                RenderMode::Integrated => self.create_headed_glutin(id, &window, window_target, Some(false)),
                RenderMode::Software => self.create_headed_swgl(id, &window),
            }));

            let error = match r {
                Ok(Ok(ctx)) => return (window, ctx),
                Ok(Err(e)) => e,
                Err(panic) => util::panic_msg(&*panic).to_owned().into(),
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
        window_target: &EventLoopWindowTarget<AppEvent>,
        render_mode: RenderMode,
    ) -> GlContext {
        let mut errors = vec![];

        for config in TryConfig::iter(render_mode) {
            if self.unsupported_headed.contains(&config) {
                errors.push((config, "previous attempt failed, not supported".into()));
                continue;
            }

            let r = util::catch_supress(std::panic::AssertUnwindSafe(|| match config.mode {
                RenderMode::Dedicated => self.create_headless_glutin(id, window_target, config.hardware_acceleration),
                RenderMode::Integrated => self.create_headless_glutin(id, window_target, Some(false)),
                RenderMode::Software => self.create_headless_swgl(id),
            }));

            let error = match r {
                Ok(Ok(ctx)) => return ctx,
                Ok(Err(e)) => e,
                Err(panic) => util::panic_msg(&*panic).to_owned().into(),
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
        id: WindowId,
        window: &winit::window::Window,
        window_target: &EventLoopWindowTarget<AppEvent>,
        hardware: Option<bool>,
    ) -> Result<GlContext, Box<dyn Error>> {
        let display_handle = window_target.raw_display_handle();
        let window_handle = window.raw_window_handle();

        #[cfg(windows)]
        let display_pref = DisplayApiPreference::WglThenEgl(Some(window_handle));

        #[cfg(any(
            target_os = "linux",
            target_os = "dragonfly",
            target_os = "freebsd",
            target_os = "openbsd",
            target_os = "netbsd"
        ))]
        let display_pref = DisplayApiPreference::GlxThenEgl(Box::new(unix::register_xlib_error_hook));

        // SAFETY: we are trusting the `raw_display_handle` from winit here.
        let display = unsafe { Display::from_raw(display_handle, display_pref) }?;

        let template = ConfigTemplateBuilder::new()
            .with_alpha_size(8)
            .with_transparency(true)
            .compatible_with_native_window(window_handle)
            .with_surface_type(ConfigSurfaceTypes::WINDOW)
            .prefer_hardware_accelerated(hardware)
            .build();

        // SAFETY: we are holding the `window` reference.
        let config = unsafe { display.find_configs(template)?.next().ok_or("no display config") }?;

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

        #[cfg(debug_assertions)]
        let gl = gl::ErrorCheckingGl::wrap(gl.clone());

        check_wr_gl_version(&*gl)?;

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

        Ok(context)
    }

    #[allow(unreachable_code)]
    fn create_headed_swgl(&mut self, id: WindowId, window: &winit::window::Window) -> Result<GlContext, Box<dyn Error>> {
        #[cfg(not(software))]
        {
            return Err("zero-ui-view not build with \"software\" backend".into());
        }

        #[cfg(software)]
        {
            if !blit::Impl::supported() {
                return Err("zero-ui-view does not fully implement headed \"software\" backend on target OS (missing blit)".into());
            }

            let blit = blit::Impl::new(window);
            let context = swgl::Context::create();
            let gl = Rc::new(context);

            // create_headed_glutin returns as current.
            self.current.set(Some(id));
            context.make_current();

            Ok(GlContext {
                id,
                current: self.current.clone(),
                backend: GlBackend::Swgl { context, blit: Some(blit) },
                gl,
                render_mode: RenderMode::Software,
            })
        }
    }

    fn create_headless_glutin(
        &mut self,
        id: WindowId,
        window_target: &EventLoopWindowTarget<AppEvent>,
        hardware: Option<bool>,
    ) -> Result<GlContext, Box<dyn Error>> {
        let hidden_window = winit::window::WindowBuilder::new()
            .with_transparent(true)
            .with_inner_size(PhysicalSize::new(1u32, 1u32))
            .with_visible(false)
            .with_decorations(false)
            .build(window_target)?;

        let display_handle = window_target.raw_display_handle();
        let window_handle = hidden_window.raw_window_handle();

        #[cfg(windows)]
        let display_pref = DisplayApiPreference::WglThenEgl(Some(window_handle));

        #[cfg(any(
            target_os = "linux",
            target_os = "dragonfly",
            target_os = "freebsd",
            target_os = "openbsd",
            target_os = "netbsd"
        ))]
        let display_pref = DisplayApiPreference::GlxThenEgl(Box::new(unix::register_xlib_error_hook));

        // SAFETY: we are trusting the `raw_display_handle` from winit here.
        let display = unsafe { Display::from_raw(display_handle, display_pref) }?;

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

        #[cfg(debug_assertions)]
        let gl = gl::ErrorCheckingGl::wrap(gl.clone());

        check_wr_gl_version(&*gl)?;

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

    #[allow(unreachable_code)]
    fn create_headless_swgl(&mut self, id: WindowId) -> Result<GlContext, Box<dyn Error>> {
        #[cfg(not(software))]
        {
            return Err("zero-ui-view not build with \"software\" backend".into());
        }

        #[cfg(software)]
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

enum GlBackend {
    Glutin {
        headless: Option<GlutinHeadless>,
        context: PossiblyCurrentContext,
        surface: Surface<WindowSurface>,
    },

    #[cfg(software)]
    Swgl {
        context: swgl::Context,
        // is None for headless.
        blit: Option<blit::Impl>,
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
    #[allow(unreachable_code)]
    pub(crate) fn is_software(&self) -> bool {
        #[cfg(software)]
        {
            return matches!(&self.backend, GlBackend::Swgl { .. });
        }
        false
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

        match &self.backend {
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
            GlBackend::Swgl { context, .. } => {
                // NULL means SWGL manages the buffer, it also retains the buffer if the size did not change.
                context.init_default_framebuffer(0, 0, size.width.max(1) as i32, size.height.max(1) as i32, 0, std::ptr::null_mut());
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
            GlBackend::Swgl { context, blit } => {
                if let Some(headed) = blit {
                    gl::Gl::finish(context);
                    let (data_ptr, w, h, stride) = context.get_color_buffer(0, true);

                    if w == 0 || h == 0 {
                        return;
                    }

                    // SAFETY: we trust SWGL
                    assert!(stride == w * 4);
                    let frame = unsafe { std::slice::from_raw_parts(data_ptr as *const u8, w as usize * h as usize * 4) };

                    headed.blit(w, h, frame);
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

    use windows::Win32::{
        Foundation::HWND,
        Graphics::{
            Gdi::*,
            OpenGL::{self, PFD_PIXEL_TYPE},
        },
    };

    let _ = std::thread::Builder::new()
        .name("warmup".to_owned())
        .stack_size(3 * 64 * 1024)
        .spawn(|| unsafe {
            let _span = tracing::trace_span!("open-gl-init").entered();
            let hdc = GetDC(HWND(0));
            let _ = OpenGL::DescribePixelFormat(hdc, PFD_PIXEL_TYPE(0), 0, std::ptr::null_mut());
            ReleaseDC(HWND(0), hdc);
        });
}

#[cfg(not(windows))]
pub(crate) fn warmup() {}

// check if equal or newer then 3.1
fn check_wr_gl_version(gl: &dyn gl::Gl) -> Result<(), String> {
    let version = gl.get_string(gl::VERSION);

    // pattern is "\d+(\.\d+)?."
    // we take the major and optionally minor versions.
    let ver: Vec<_> = version.split('.').take(2).filter_map(|n| n.parse::<u8>().ok()).collect();

    let supported = if ver[0] == 3 {
        ver.get(1).copied().unwrap_or(0) >= 1
    } else {
        ver[0] > 3
    };

    if supported {
        Ok(())
    } else {
        Err(format!("webrender requires OpenGL 3.1 or newer, found {version}"))
    }
}

/// Glutin, SWGL config to attempt.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
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

#[cfg(software)]
mod blit {
    /// Bottom-top BGRA8.
    pub type Bgra8 = [u8];

    #[cfg(not(any(
        windows,
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd",
    )))]
    pub type Impl = NotImplementedBlit;

    #[cfg(windows)]
    pub type Impl = windows_blit::GdiBlit;

    #[cfg(any(
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd",
    ))]
    pub type Impl = linux_blit::XLibOrWaylandBlit;

    #[allow(unused)]
    pub struct NotImplementedBlit {}
    #[allow(unused)]
    impl NotImplementedBlit {
        pub fn new(_window: &winit::window::Window) -> Self {
            NotImplementedBlit {}
        }

        pub fn supported() -> bool {
            false
        }

        pub fn blit(&mut self, _width: i32, _height: i32, _frame: &Bgra8) {
            panic!("Software blit not implemented on this OS");
        }
    }

    #[cfg(windows)]
    mod windows_blit {

        use windows::Win32::Foundation::HWND;
        use windows::Win32::Graphics::Gdi::*;
        use winit::platform::windows::WindowExtWindows;

        pub struct GdiBlit {
            hwnd: HWND,
        }

        impl GdiBlit {
            pub fn new(window: &winit::window::Window) -> Self {
                GdiBlit {
                    hwnd: HWND(window.hwnd() as _),
                }
            }

            pub fn supported() -> bool {
                true
            }

            pub fn blit(&mut self, width: i32, height: i32, frame: &super::Bgra8) {
                // SAFETY: its a simple operation, and we try to cleanup before panic.
                unsafe { self.blit_unsafe(width, height, frame) }
            }

            unsafe fn blit_unsafe(&mut self, width: i32, height: i32, frame: &super::Bgra8) {
                // not BeginPaint because winit calls DefWindowProcW?

                let hdc = GetDC(self.hwnd);

                let mem_dc = CreateCompatibleDC(hdc);
                let mem_bm = CreateCompatibleBitmap(hdc, width, height);

                let bmi = BITMAPINFO {
                    bmiHeader: BITMAPINFOHEADER {
                        biSize: std::mem::size_of::<BITMAPINFO>() as u32,
                        biWidth: width,
                        biHeight: height,
                        biPlanes: 1,
                        biBitCount: 32,
                        biCompression: 0,
                        biSizeImage: 0,
                        biXPelsPerMeter: 0,
                        biYPelsPerMeter: 0,
                        biClrUsed: 0,
                        biClrImportant: 0,
                    },
                    bmiColors: [RGBQUAD {
                        rgbBlue: 0,
                        rgbGreen: 0,
                        rgbRed: 0,
                        rgbReserved: 0,
                    }],
                };
                let old_bm = SelectObject(mem_dc, mem_bm);

                StretchDIBits(
                    mem_dc,
                    0,
                    0,
                    width,
                    height,
                    0,
                    0,
                    width,
                    height,
                    frame.as_ptr() as *const _,
                    &bmi as *const _,
                    DIB_USAGE(0),
                    SRCCOPY,
                );
                BitBlt(hdc, 0, 0, width, height, mem_dc, 0, 0, SRCCOPY);

                SelectObject(mem_dc, old_bm);
                ReleaseDC(self.hwnd, hdc);
            }
        }
    }

    #[cfg(any(
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd",
    ))]
    mod linux_blit {
        use wayland_client::protocol::wl_surface::WlSurface;
        use winit::platform::unix::{x11::ffi::*, WindowExtUnix};

        #[allow(clippy::large_enum_variant)]
        pub enum XLibOrWaylandBlit {
            XLib { xlib: Xlib, display: *mut _XDisplay, window: u64 },
            Wayland { surface: *const WlSurface },
        }

        impl XLibOrWaylandBlit {
            pub fn new(window: &winit::window::Window) -> Self {
                if let Some(d) = window.xlib_display() {
                    Self::XLib {
                        xlib: Xlib::open().unwrap(),
                        display: d as _,
                        window: window.xlib_window().unwrap(),
                    }
                } else if let Some(d) = window.wayland_surface() {
                    Self::Wayland { surface: d as _ }
                } else {
                    panic!("window does not use XLib nor Wayland");
                }
            }

            pub fn supported() -> bool {
                true
            }

            pub fn blit(&mut self, width: i32, height: i32, frame: &super::Bgra8) {
                match self {
                    XLibOrWaylandBlit::XLib { xlib, display, window } => unsafe {
                        Self::xlib_blit(xlib, *display, *window, width as _, height as _, frame)
                    },
                    XLibOrWaylandBlit::Wayland { surface } => unsafe { Self::wayland_blit(*surface, width, height, frame) },
                }
            }

            unsafe fn xlib_blit(xlib: &Xlib, display: *mut _XDisplay, window: u64, width: u32, height: u32, frame: &super::Bgra8) {
                let screen = (xlib.XDefaultScreen)(display);

                let mut info: XVisualInfo = std::mem::zeroed();
                if (xlib.XMatchVisualInfo)(display, screen, 32, TrueColor, &mut info) == 0 {
                    panic!("no compatible xlib visual")
                }

                let mut top_down_frame = Vec::with_capacity(frame.len());
                let line_len = width as usize * 4;
                for line in frame.chunks_exact(line_len).rev() {
                    top_down_frame.extend_from_slice(line);
                }
                let frame = top_down_frame.as_ptr();

                let mut opts: XGCValues = std::mem::zeroed();
                opts.graphics_exposures = 0;
                let ctx = (xlib.XCreateGC)(display, window, GCGraphicsExposures as _, &mut opts);

                let img = (xlib.XCreateImage)(
                    display,
                    info.visual,
                    32,
                    ZPixmap,
                    0,
                    frame as _,
                    width as _,
                    height as _,
                    8,
                    line_len as i32,
                );

                (xlib.XPutImage)(display, window, ctx, img, 0, 0, 0, 0, width, height);

                (xlib.XFreeGC)(display, ctx);
                // (xlib.XDestroyImage)(img);
            }

            unsafe fn wayland_blit(surface: *const WlSurface, width: i32, height: i32, frame: &super::Bgra8) {
                let _ = (surface, width, height, frame);
                todo!("wayland blit not implemented")
            }
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
