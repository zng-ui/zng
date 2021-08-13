struct HeadlessData {
    _el: EventLoop<()>,
    rbos: [u32; 2],
    fbo: [u32; 1],
}
impl HeadlessData {
    fn partial(el: EventLoop<()>) -> Self {
        HeadlessData {
            _el: el,
            rbos: [0; 2],
            fbo: [0; 1],
        }
    }
}

/// Create a headless renderer.
///
/// The `size` must be already scaled by the `scale_factor`. The `scale_factor` is usually `1.0` for headless rendering.
///
/// The `render_callback` is called every time a new frame is ready to be [presented](Self::present).
///
/// The `window_id` is the optional id of the headless window associated with this renderer.
pub fn new<C: RenderCallback>(
    size: RenderSize,
    scale_factor: f32,
    config: RendererConfig,
    render_callback: C,
    window_id: Option<crate::window::WindowId>,
) -> Result<Self, RendererError> {
    if !is_main_thread::is_main_thread().unwrap_or(true) {
        panic!("can only init renderer in the main thread")
    }
    let el = glutin::event_loop::EventLoop::new();
    let context = ContextBuilder::new().with_gl(GlRequest::GlThenGles {
        opengl_version: (3, 2),
        opengles_version: (3, 0),
    });
    let size_one = glutin::dpi::PhysicalSize::new(1, 1);
    let renderer_kind;
    #[cfg(target_os = "linux")]
    let context = {
        use glutin::platform::unix::HeadlessContextExt;
        match context.clone().build_surfaceless(&el) {
            Ok(ctx) => {
                renderer_kind = RendererKind::Native;
                ctx
            }
            Err(suf_e) => match context.clone().build_headless(&el, size_one) {
                Ok(ctx) => {
                    renderer_kind = RendererKind::Native;
                    ctx
                }
                Err(hea_e) => match context.build_osmesa(size_one) {
                    Ok(ctx) => {
                        renderer_kind = RendererKind::OSMesa;
                        ctx
                    }
                    Err(osm_e) => return Err(RendererError::CreationHeadlessLinux([suf_e, hea_e, osm_e])),
                },
            },
        }
    };
    #[cfg(not(target_os = "linux"))]
    let context = {
        let c = context.build_headless(&el, size_one)?;
        renderer_kind = RendererKind::Native;
        c
    };


    #[cfg(debug_assertions)]
    let gl = gleam::gl::ErrorCheckingGl::wrap(gl.clone());

    // manually create a surface for headless.
    let rbos = gl.gen_renderbuffers(2);
    let fbo = gl.gen_framebuffers(1)[0];

    data.fbo = [fbo];
    data.rbos = [rbos[0], rbos[1]];
    Self::size_headless(&gl, &data.rbos, size);

    gl.bind_framebuffer(gl::FRAMEBUFFER, fbo);
    gl.framebuffer_renderbuffer(gl::FRAMEBUFFER, gl::COLOR_ATTACHMENT0, gl::RENDERBUFFER, rbos[0]);
    gl.framebuffer_renderbuffer(gl::FRAMEBUFFER, gl::DEPTH_STENCIL_ATTACHMENT, gl::RENDERBUFFER, rbos[1]);

}

fn resize_headless(gl: &Rc<dyn gl::Gl>, rbos: &[u32; 2], size: RenderSize) {
    gl.bind_renderbuffer(gl::RENDERBUFFER, rbos[0]);
    gl.renderbuffer_storage(gl::RENDERBUFFER, gl::RGBA8, size.width, size.height);

    gl.bind_renderbuffer(gl::RENDERBUFFER, rbos[1]);
    gl.renderbuffer_storage(gl::RENDERBUFFER, gl::DEPTH24_STENCIL8, size.width, size.height);

    gl.viewport(0, 0, size.width, size.height);
}

fn drop() {
    self.gl.delete_framebuffers(&data.fbo);
    self.gl.delete_renderbuffers(&data.rbos);
}