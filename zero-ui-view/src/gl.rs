use std::{
    cell::Cell,
    fmt::{self, Write},
    mem,
    rc::Rc,
};

use crate::util::{self, panic_msg, PxToWinit};
use gleam::gl::{self, Gl};
use glutin::{event_loop::EventLoopWindowTarget, window::WindowBuilder, PossiblyCurrent};
use linear_map::set::LinearSet;
use parking_lot::Mutex;
use zero_ui_view_api::{
    units::{Px, PxSize},
    RenderMode, WindowId,
};

pub(crate) struct GlContext {
    id: WindowId,
    current_id: CurrentId,

    mode: RenderMode,
    inner: GlContextInner,
    gl: Rc<dyn Gl>,
}

#[allow(clippy::large_enum_variant)]
enum GlContextInner {
    Headed(glutin::ContextWrapper<PossiblyCurrent, ()>),
    Headless(HeadlessData),
    #[cfg(software)]
    Software(swgl::Context, Option<blit::Impl>),

    // glutin context takes ownership to make current..
    MakingCurrent,

    Dropped,
}

struct HeadlessData {
    ctx: glutin::Context<PossiblyCurrent>,
    gl: Rc<dyn Gl>,

    // webrender requirements
    rbos: [u32; 2],
    fbo: u32,
}
impl HeadlessData {
    fn new(ctx: glutin::Context<PossiblyCurrent>, gl: Rc<dyn Gl>) -> Self {
        // manually create a surface for Webrender:

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

        HeadlessData { ctx, gl, rbos, fbo }
    }

    fn resize(&self, width: i32, height: i32) {
        self.gl.bind_renderbuffer(gl::RENDERBUFFER, self.rbos[0]);
        self.gl.renderbuffer_storage(gl::RENDERBUFFER, gl::RGBA8, width, height);

        self.gl.bind_renderbuffer(gl::RENDERBUFFER, self.rbos[1]);
        self.gl.renderbuffer_storage(gl::RENDERBUFFER, gl::DEPTH24_STENCIL8, width, height);

        self.gl.viewport(0, 0, width, height);
    }

    fn destroy(mut self, is_current: bool) {
        if !is_current {
            self.ctx = unsafe { self.ctx.treat_as_not_current().make_current().unwrap() };
        }

        self.gl.delete_framebuffers(&[self.fbo]);
        self.gl.delete_renderbuffers(&self.rbos);

        let _ = unsafe { self.ctx.make_not_current() };
    }
}

impl GlContext {
    pub fn gl(&self) -> &Rc<dyn Gl> {
        &self.gl
    }

    pub fn is_current(&self) -> bool {
        Some(self.id) == self.current_id.get()
    }

    #[cfg(software)]
    pub fn is_software(&self) -> bool {
        matches!(&self.inner, GlContextInner::Software(_, _))
    }

    #[cfg(not(software))]
    pub fn is_software(&self) -> bool {
        false
    }

    pub fn render_mode(&self) -> RenderMode {
        self.mode
    }

    pub fn make_current(&mut self) {
        if self.is_current() {
            return;
        }

        self.current_id.set(Some(self.id));

        #[cfg(software)]
        if let GlContextInner::Software(ctx, _) = &self.inner {
            ctx.make_current();
            return;
        }

        // SAFETY:
        // glutin docs says that calling `make_not_current` is not necessary and
        // that "If you call make_current on some context, you should call treat_as_not_current as soon
        // as possible on the previously current context."
        //
        // As far as the glutin code goes `treat_as_not_current` just changes the state tag, so we can call
        // `treat_as_not_current` just to get access to the `make_current` when we know it is not current
        // anymore, and just ignore the whole "current state tag" thing.
        self.inner = match mem::replace(&mut self.inner, GlContextInner::MakingCurrent) {
            GlContextInner::Headed(ctx) => {
                let ctx = unsafe { ctx.treat_as_not_current().make_current() }.unwrap();
                GlContextInner::Headed(ctx)
            }
            GlContextInner::Headless(mut ctx) => {
                ctx.ctx = unsafe { ctx.ctx.treat_as_not_current().make_current() }.unwrap();
                GlContextInner::Headless(ctx)
            }
            s => panic!("unexpected context state, {s:?}"),
        }
    }

    pub fn resize(&self, width: i32, height: i32) {
        assert!(self.is_current());

        match &self.inner {
            GlContextInner::Headed(ctx) => ctx.resize(glutin::dpi::PhysicalSize::new(width as _, height as _)),
            GlContextInner::Headless(ctx) => ctx.resize(width, height),
            #[cfg(software)]
            GlContextInner::Software(ctx, _) => {
                // NULL means SWGL manages the buffer, it also retains the buffer if the size did not change.
                ctx.init_default_framebuffer(0, 0, width, height, 0, std::ptr::null_mut());
            }
            s => panic!("unexpected context state, {s:?}"),
        }
    }

    /// Swap headed buffers or blit headed software.
    pub fn swap_buffers(&mut self) {
        assert!(self.is_current());

        match &mut self.inner {
            GlContextInner::Headed(ctx) => ctx.swap_buffers().unwrap(),
            GlContextInner::Headless(_) => {}
            #[cfg(software)]
            GlContextInner::Software(swgl, headed) => {
                if let Some(headed) = headed {
                    swgl.finish();
                    let (data_ptr, w, h, stride) = swgl.get_color_buffer(0, true);

                    if w == 0 || h == 0 {
                        return;
                    }

                    // SAFETY: we trust SWGL
                    assert!(stride == w * 4);
                    let frame = unsafe { std::slice::from_raw_parts(data_ptr as *const u8, w as usize * h as usize * 4) };

                    headed.blit(w, h, frame);
                }
            }
            s => panic!("unexpected context state, {s:?}"),
        }
    }

    /// Glutin requires that the context is [dropped before the window][1], calling this
    /// function safely disposes of the context, the winit window should be dropped immediately after.
    ///
    /// [1]: https://docs.rs/glutin/0.27.0/glutin/type.WindowedContext.html#method.split
    pub fn drop_before_winit(&mut self) {
        match mem::replace(&mut self.inner, GlContextInner::Dropped) {
            GlContextInner::Headed(ctx) => {
                if self.is_current() {
                    let _ = unsafe { ctx.make_not_current() };
                } else {
                    let _ = unsafe { ctx.treat_as_not_current() };
                }
            }
            GlContextInner::Headless(ctx) => ctx.destroy(self.is_current()),
            #[cfg(software)]
            GlContextInner::Software(ctx, _) => ctx.destroy(),
            GlContextInner::Dropped => {}
            GlContextInner::MakingCurrent => {
                tracing::error!("unexpected `MakingCurrent` on drop");
            }
        }

        if self.is_current() {
            self.current_id.set(None);
        }
    }
}

impl Drop for GlContext {
    fn drop(&mut self) {
        match mem::replace(&mut self.inner, GlContextInner::Dropped) {
            GlContextInner::Headed(_) => panic!("call `drop_before_winit` before dropping a headed context"),
            GlContextInner::Headless(ctx) => ctx.destroy(self.is_current()),
            #[cfg(software)]
            GlContextInner::Software(ctx, _) => ctx.destroy(),
            GlContextInner::Dropped => {}
            GlContextInner::MakingCurrent => {
                tracing::error!("unexpected `MakingCurrent` on drop");
            }
        }
    }
}

type CurrentId = Rc<Cell<Option<WindowId>>>;

/// Manages the "current" `glutin` or `swgl` OpenGL context.
///
/// # Safety
///
/// If this manager is in use all OpenGL contexts created in the process must be managed by a single instance of it.
#[derive(Default)]
pub(crate) struct GlContextManager {
    current_id: CurrentId,
    unsupported: Mutex<LinearSet<TryConfig>>,
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
                        // so we try `None`, after `Some(false)`.
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

impl GlContextManager {
    pub fn create_headed(
        &self,
        id: WindowId,
        window: WindowBuilder,
        window_target: &EventLoopWindowTarget<crate::AppEvent>,
        mode_pref: RenderMode,
    ) -> (GlContext, glutin::window::Window) {
        let mut unsupported = self.unsupported.lock();

        let mut error_log = String::new();

        for config in TryConfig::iter(mode_pref) {
            if unsupported.contains(&config) {
                let _ = write!(&mut error_log, "\n[{}]\nskip, previous attempt failed", config.name());
                continue;
            }

            match config.mode {
                #[cfg(software)]
                RenderMode::Software => {
                    if !blit::Impl::supported() {
                        error_log.push_str(
                            "\n[Software]\nzero-ui-view does not fully implement headed \"software\" backend on target OS (missing blit)",
                        );

                        unsupported.insert(config);
                        continue;
                    }

                    let _span = tracing::trace_span!("create-software-ctx").entered();

                    let window = window.build(window_target).unwrap();
                    let headed = blit::Impl::new(&window);
                    let ctx = self.create_software(id, Some(headed));

                    return (ctx, window);
                }
                #[cfg(not(software))]
                RenderMode::Software => {
                    error_log.push_str("\n[Software]\nzero-ui-view not build with \"software\" backend");
                    unsupported.insert(config);
                }

                mode => {
                    // glutin try.

                    let mut logged = false;
                    let mut log_error = |e: &dyn fmt::Debug| {
                        if !logged {
                            let _ = write!(error_log, "\n[{}]", config.name());
                            logged = true;
                            unsupported.insert(config);
                        }
                        let _ = write!(error_log, "\n{e:?}");
                    };

                    let _span = tracing::trace_span!("create-glutin-ctx", ?config).entered();

                    let panic_result = util::catch_supress(assert_unwind_safe(|| {
                        glutin::ContextBuilder::new()
                            .with_gl(glutin::GlRequest::Latest)
                            .with_hardware_acceleration(config.hardware_acceleration)
                            .build_windowed(window.clone(), window_target)
                    }));

                    match panic_result {
                        Ok(Ok(c)) => {
                            self.current_id.set(None);

                            // SAFETY: we assume all glutin context are managed by us in a single thread.
                            let ctx = match unsafe { c.make_current() } {
                                Ok(c) => c,
                                Err(e) => {
                                    log_error(&e);
                                    continue;
                                }
                            };

                            let gl = match ctx.get_api() {
                                glutin::Api::OpenGl => unsafe { gl::GlFns::load_with(|symbol| ctx.get_proc_address(symbol) as *const _) },
                                glutin::Api::OpenGlEs => unsafe {
                                    gl::GlesFns::load_with(|symbol| ctx.get_proc_address(symbol) as *const _)
                                },
                                glutin::Api::WebGl => {
                                    log_error(&"WebGL is not supported");
                                    continue;
                                }
                            };

                            #[cfg(debug_assertions)]
                            let gl = gl::ErrorCheckingGl::wrap(gl.clone());

                            if !wr_supports_gl(&*gl) {
                                log_error(&"Webrender requires at least OpenGL 3.1");
                                continue;
                            }

                            self.current_id.set(Some(id));

                            // SAFETY: panic if `ctx` is not dropped using `GlContext::drop_before_winit`.
                            let (ctx, window) = unsafe { ctx.split() };

                            let ctx = GlContextInner::Headed(ctx);

                            let ctx = GlContext {
                                id,
                                current_id: self.current_id.clone(),
                                mode,
                                inner: ctx,
                                gl,
                            };

                            return (ctx, window);
                        }
                        Ok(Err(e)) => log_error(&e),
                        Err(payload) => log_error(&panic_msg(&payload)),
                    }
                }
            }
        }

        panic!("failed to created headed context:{error_log}");
    }

    pub fn create_headless(
        &self,
        id: WindowId,
        window_target: &EventLoopWindowTarget<crate::AppEvent>,
        mode_pref: RenderMode,
    ) -> GlContext {
        let mut unsupported = self.unsupported.lock();

        let mut error_log = String::new();

        let size = PxSize::new(Px(2), Px(2));

        for config in TryConfig::iter(mode_pref) {
            if unsupported.contains(&config) {
                let _ = write!(&mut error_log, "\n[{}]\nskip, previous attempt failed", config.name());
                continue;
            }

            match config.mode {
                #[cfg(software)]
                RenderMode::Software => {
                    return self.create_software(id, None);
                }
                #[cfg(not(software))]
                RenderMode::Software => {
                    error_log.push_str("\n[Software]\nzero-ui-view not build with \"software\" backend");
                    unsupported.insert(config);
                }

                mode => {
                    // glutin try.

                    let mut logged = false;
                    let mut log_error = |e: &dyn fmt::Debug| {
                        if !logged {
                            let _ = write!(error_log, "\n[{}]", config.name());
                            logged = true;
                            unsupported.insert(config);
                        }
                        let _ = write!(error_log, "\n{e:?}");
                    };

                    let context_builder = glutin::ContextBuilder::new()
                        .with_gl(glutin::GlRequest::Latest)
                        .with_hardware_acceleration(config.hardware_acceleration);

                    // On Linux, try "surfaceless" first.
                    #[cfg(any(
                        target_os = "linux",
                        target_os = "dragonfly",
                        target_os = "freebsd",
                        target_os = "netbsd",
                        target_os = "openbsd",
                    ))]
                    let panic_result = {
                        use glutin::platform::unix::HeadlessContextExt;

                        let c = context_builder.clone();
                        let r = util::catch_supress(assert_unwind_safe(|| c.build_surfaceless(window_target)));

                        let mut surfaceless_ok = false;
                        match &r {
                            Ok(Ok(_)) => surfaceless_ok = true,
                            Ok(Err(e)) => log_error(&format!("surfaceless error: {e:?}")),
                            Err(payload) => log_error(&format!("surfaceless panic: {}", panic_msg(&*payload))),
                        }

                        if surfaceless_ok {
                            r
                        } else {
                            util::catch_supress(assert_unwind_safe(|| {
                                context_builder.build_headless(window_target, size.to_winit())
                            }))
                        }
                    };

                    #[cfg(not(any(
                        target_os = "linux",
                        target_os = "dragonfly",
                        target_os = "freebsd",
                        target_os = "netbsd",
                        target_os = "openbsd",
                    )))]
                    let panic_result = util::catch_supress(assert_unwind_safe(|| {
                        context_builder.build_headless(window_target, size.to_winit())
                    }));

                    match panic_result {
                        Ok(Ok(c)) => {
                            self.current_id.set(None);

                            // SAFETY: we assume all glutin context are managed by us in a single thread.
                            let ctx = match unsafe { c.make_current() } {
                                Ok(c) => c,
                                Err(e) => {
                                    log_error(&e);
                                    continue;
                                }
                            };

                            let gl = match ctx.get_api() {
                                glutin::Api::OpenGl => unsafe { gl::GlFns::load_with(|symbol| ctx.get_proc_address(symbol) as *const _) },
                                glutin::Api::OpenGlEs => unsafe {
                                    gl::GlesFns::load_with(|symbol| ctx.get_proc_address(symbol) as *const _)
                                },
                                glutin::Api::WebGl => {
                                    log_error(&"WebGL is not supported");
                                    continue;
                                }
                            };
                            #[cfg(debug_assertions)]
                            let gl = gl::ErrorCheckingGl::wrap(gl.clone());

                            if !wr_supports_gl(&*gl) {
                                log_error(&"Webrender requires at least OpenGL 3.1");
                                continue;
                            }

                            self.current_id.set(Some(id));

                            let ctx = GlContextInner::Headless(HeadlessData::new(ctx, gl.clone()));

                            return GlContext {
                                id,
                                current_id: self.current_id.clone(),
                                mode,
                                inner: ctx,
                                gl,
                            };
                        }
                        Ok(Err(e)) => log_error(&e),
                        Err(payload) => log_error(&panic_msg(&payload)),
                    }
                }
            }
        }

        panic!("failed to created headeless context:{error_log}");
    }

    #[cfg(software)]
    fn create_software(&self, id: WindowId, headed: Option<blit::Impl>) -> GlContext {
        let ctx = swgl::Context::create();
        let gl = Rc::new(ctx);
        let ctx = GlContextInner::Software(ctx, headed);
        let mut ctx = GlContext {
            id,
            current_id: self.current_id.clone(),
            mode: RenderMode::Software,
            inner: ctx,
            gl,
        };
        ctx.make_current();

        ctx
    }
}

// check if equal or newer then 3.1
fn wr_supports_gl(gl: &dyn Gl) -> bool {
    let version = gl.get_string(gl::VERSION);

    // pattern is "\d+(\.\d+)?."
    // we take the major and optionally minor versions.
    let ver: Vec<_> = version.split('.').take(2).filter_map(|n| n.parse::<u8>().ok()).collect();

    if ver[0] == 3 {
        ver.get(1).copied().unwrap_or(0) >= 1
    } else {
        ver[0] > 3
    }
}

impl fmt::Debug for GlContextInner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Headed(_) => write!(f, "Headed"),
            Self::Headless(_) => write!(f, "Headless"),
            #[cfg(software)]
            Self::Software(_, _) => write!(f, "Software"),
            Self::MakingCurrent => write!(f, "MakingCurrent"),
            Self::Dropped => write!(f, "Dropped"),
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
        pub fn new(_window: &glutin::window::Window) -> Self {
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

        use glutin::platform::windows::WindowExtWindows;
        use windows::Win32::Foundation::HWND;
        use windows::Win32::Graphics::Gdi::*;

        pub struct GdiBlit {
            hwnd: HWND,
        }

        impl GdiBlit {
            pub fn new(window: &glutin::window::Window) -> Self {
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
        use glutin::platform::unix::{x11::ffi::*, WindowExtUnix};
        use wayland_client::protocol::wl_surface::WlSurface;

        #[allow(clippy::large_enum_variant)]
        pub enum XLibOrWaylandBlit {
            XLib { xlib: Xlib, display: *mut _XDisplay, window: u64 },
            Wayland { surface: *const WlSurface },
        }

        impl XLibOrWaylandBlit {
            pub fn new(window: &glutin::window::Window) -> Self {
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

/// Assert that a glutin `&EventLoopWindowTarget` can still be used after a panic during build.
///
/// We expect panics to happen before the event loop is modified, they have some `unimplemented!` panics,
/// at worst we will leak a bit but still fallback to software context, better then becoming unusable?
fn assert_unwind_safe<T>(glutin_build: T) -> std::panic::AssertUnwindSafe<T> {
    std::panic::AssertUnwindSafe(glutin_build)
}
