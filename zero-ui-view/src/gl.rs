use std::{
    cell::Cell,
    fmt::{self, Write},
    mem,
    rc::Rc,
};

use crate::util::PxToWinit;
use gleam::gl::{self, Gl};
use glutin::{event_loop::EventLoopWindowTarget, window::WindowBuilder, PossiblyCurrent};
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
            s => panic!("unexpected context state, {:?}", s),
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
            s => panic!("unexpected context state, {:?}", s),
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
            s => panic!("unexpected context state, {:?}", s),
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
}

/// Glutin, SWGL config to attempt.
#[derive(Debug)]
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
                        // some dedicated harwarare endup classified as generic integrated for some reason,
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
        let mut error_log = String::new();

        for config in TryConfig::iter(mode_pref) {
            match config.mode {
                #[cfg(software)]
                RenderMode::Software => {
                    if !blit::Impl::supported() {
                        error_log.push_str(
                            "\n[Software]\nzero-ui-view does not fully implement headed \"software\" backend on target OS (missing blit)",
                        );

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
                }

                mode => {
                    // glutin try.

                    let mut logged = false;
                    let mut log_error = |e: &dyn fmt::Debug| {
                        if !logged {
                            let _ = write!(error_log, "\n[{}]", config.name());
                            logged = true;
                        }
                        let _ = write!(error_log, "\n{:?}", e);
                    };

                    let _span = tracing::trace_span!("create-glutin-ctx", ?config).entered();

                    let r = glutin::ContextBuilder::new()
                        .with_gl(glutin::GlRequest::Latest)
                        .with_hardware_acceleration(config.hardware_acceleration)
                        .build_windowed(window.clone(), window_target);

                    match r {
                        Ok(c) => {
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
                        Err(e) => log_error(&e),
                    }
                }
            }
        }

        panic!("failed to created headed context:{}", error_log);
    }

    pub fn create_headless(
        &self,
        id: WindowId,
        window_target: &EventLoopWindowTarget<crate::AppEvent>,
        mode_pref: RenderMode,
    ) -> GlContext {
        let mut error_log = String::new();

        let size = PxSize::new(Px(2), Px(2));

        for config in TryConfig::iter(mode_pref) {
            match config.mode {
                #[cfg(software)]
                RenderMode::Software => {
                    return self.create_software(id, None);
                }
                #[cfg(not(software))]
                RenderMode::Software => {
                    error_log.push_str("\n[Software]\nzero-ui-view not build with \"software\" backend");
                }

                mode => {
                    // glutin try.

                    let mut logged = false;
                    let mut log_error = |e: &dyn fmt::Debug| {
                        if !logged {
                            let _ = write!(error_log, "\n[{}]", config.name());
                            logged = true;
                        }
                        let _ = write!(error_log, "\n{:?}", e);
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
                    let r = {
                        use glutin::platform::unix::HeadlessContextExt;

                        let mut r = context_builder.clone().build_surfaceless(window_target);
                        if let Err(e) = r {
                            log_error(&format!("surfaceless error: {:?}", e));

                            r = context_builder.build_headless(window_target, size.to_winit());
                        }
                        r
                    };

                    #[cfg(not(any(
                        target_os = "linux",
                        target_os = "dragonfly",
                        target_os = "freebsd",
                        target_os = "netbsd",
                        target_os = "openbsd",
                    )))]
                    let r = context_builder.build_headless(window_target, size.to_winit());

                    match r {
                        Ok(c) => {
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
                        Err(e) => log_error(&e),
                    }
                }
            }
        }

        panic!("failed to created headeless context:{}", error_log);
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

fn wr_supports_gl(gl: &dyn Gl) -> bool {
    let ver: Vec<_> = gl
        .get_string(gl::VERSION)
        .split('.')
        .take(2)
        .map(|n| n.parse::<u8>().unwrap())
        .collect();

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
    pub type Impl = NotImplementedBlit;

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
        use winapi::{
            shared::windef::HWND,
            um::{wingdi, winuser},
        };

        pub struct GdiBlit {
            hwnd: HWND,
        }

        impl GdiBlit {
            pub fn new(window: &glutin::window::Window) -> Self {
                GdiBlit { hwnd: window.hwnd() as _ }
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

                let hdc = winuser::GetDC(self.hwnd);

                let mem_dc = wingdi::CreateCompatibleDC(hdc);
                let mem_bm = wingdi::CreateCompatibleBitmap(hdc, width, height);

                let mut bmi = wingdi::BITMAPINFO::default();
                {
                    let mut info = &mut bmi.bmiHeader;
                    info.biSize = std::mem::size_of::<wingdi::BITMAPINFO>() as u32;
                    info.biWidth = width;
                    info.biHeight = height;
                    info.biPlanes = 1;
                    info.biBitCount = 32;
                }

                let old_bm = wingdi::SelectObject(mem_dc, mem_bm as winapi::shared::minwindef::LPVOID);

                wingdi::StretchDIBits(
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
                    0,
                    wingdi::SRCCOPY,
                );
                wingdi::BitBlt(hdc, 0, 0, width, height, mem_dc, 0, 0, wingdi::SRCCOPY);

                wingdi::SelectObject(mem_dc, old_bm);
                winuser::ReleaseDC(self.hwnd, hdc);
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
    mod linux_blit {}
}
