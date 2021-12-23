use std::{
    cell::Cell,
    fmt::{self, Write},
    mem,
    rc::Rc,
};

use crate::util::PxToWinit;
use gleam::gl::{self, Gl};
use glutin::{event_loop::EventLoopWindowTarget, window::WindowBuilder, PossiblyCurrent};
use zero_ui_view_api::{units::PxSize, RenderMode, WindowId};

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
    Software(swgl::Context),

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
        matches!(&self.inner, GlContextInner::Software(_))
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
        if let GlContextInner::Software(ctx) = &self.inner {
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
            GlContextInner::Software(ctx) => {
                // NULL means SWGL manages the buffer, it also retains the buffer if the size did not change.
                ctx.init_default_framebuffer(0, 0, width, height, 0, std::ptr::null_mut());
            }
            s => panic!("unexpected context state, {:?}", s),
        }
    }

    /// Blit software render or swap headed buffers.
    pub fn swap_buffers(&self) {
        assert!(self.is_current());

        match &self.inner {
            GlContextInner::Headed(ctx) => ctx.swap_buffers().unwrap(),
            GlContextInner::Headless(_) => {}
            #[cfg(software)]
            GlContextInner::Software(_) => todo!(),
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
            GlContextInner::Software(ctx) => ctx.destroy(),
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
            GlContextInner::Software(ctx) => ctx.destroy(),
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
                    // software mode does not use glutin.

                    let window = window.build(window_target).unwrap();
                    let size = window.inner_size();
                    let ctx = self.create_software(id, size.width as _, size.height as _);

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
        size: PxSize,
    ) -> GlContext {
        let mut error_log = String::new();

        for config in TryConfig::iter(mode_pref) {
            match config.mode {
                #[cfg(software)]
                RenderMode::Software => {
                    return self.create_software(id, size.width.0, size.height.0);
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
    fn create_software(&self, id: WindowId, width: i32, height: i32) -> GlContext {
        let ctx = swgl::Context::create();
        let gl = Rc::new(ctx);
        let ctx = GlContextInner::Software(ctx);
        let mut ctx = GlContext {
            id,
            current_id: self.current_id.clone(),
            mode: RenderMode::Software,
            inner: ctx,
            gl,
        };
        ctx.make_current();
        ctx.resize(width, height);

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
            Self::Software(_) => write!(f, "Software"),
            Self::MakingCurrent => write!(f, "MakingCurrent"),
            Self::Dropped => write!(f, "Dropped"),
        }
    }
}

// TODO
#[cfg(software)]
fn upload_swgl_to_native(swgl: &swgl::Context, gl: &dyn Gl) {
    swgl.finish();

    let tex = gl.gen_textures(1)[0];
    gl.bind_texture(gl::TEXTURE_2D, tex);
    let (data_ptr, w, h, stride) = swgl.get_color_buffer(0, true);

    if w == 0 || h == 0 {
        tracing::error!("cannot upload SWGL, no color buffer, did resize not get called?");
        return;
    }

    assert!(stride == w * 4);
    let buffer = unsafe { std::slice::from_raw_parts(data_ptr as *const u8, w as usize * h as usize * 4) };
    gl.tex_image_2d(
        gl::TEXTURE_2D,
        0,
        gl::RGBA8 as _,
        w,
        h,
        0,
        gl::BGRA,
        gl::UNSIGNED_BYTE,
        Some(buffer),
    );
    let fb = gl.gen_framebuffers(1)[0];
    gl.bind_framebuffer(gl::READ_FRAMEBUFFER, fb);
    gl.framebuffer_texture_2d(gl::READ_FRAMEBUFFER, gl::COLOR_ATTACHMENT0, gl::TEXTURE_2D, tex, 0);
    gl.blit_framebuffer(0, 0, w, h, 0, 0, w, h, gl::COLOR_BUFFER_BIT, gl::NEAREST);
    gl.delete_framebuffers(&[fb]);
    gl.delete_textures(&[tex]);
    gl.finish();
}
