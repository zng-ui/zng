use std::rc::Rc;

use gleam::gl;
use glutin::{dpi::PhysicalSize, Api as GApi, ContextBuilder, GlRequest};
use webrender::{api::*, RenderApi, Renderer, RendererOptions, Transaction};

use crate::{
    config,
    units::*,
    util::{self, GlHeadlessContext},
    AppEvent, AppEventSender, Context, FramePixels, FrameRequest, HeadlessConfig, TextAntiAliasing, ViewProcessGen, WinId,
};

pub(crate) struct ViewHeadless {
    id: WinId,
    pipeline_id: PipelineId,
    document_id: DocumentId,
    api: RenderApi,
    size: DipSize,
    scale_factor: f32,
    clear_color: Option<ColorF>,

    context: GlHeadlessContext,
    gl: Rc<dyn gl::Gl>,
    renderer: Option<Renderer>,
    rbos: [u32; 2],
    fbo: u32,

    frame_id: Epoch,
    resized: bool,
}
impl ViewHeadless {
    pub fn new<E: AppEventSender>(ctx: &Context<E>, gen: ViewProcessGen, id: WinId, cfg: HeadlessConfig) -> Self {
        let context = ContextBuilder::new()
            .with_gl(GlRequest::GlThenGles {
                opengl_version: (3, 2),
                opengles_version: (3, 0),
            })
            .with_hardware_acceleration(None);

        let size_one = PhysicalSize::new(1, 1);
        #[cfg(target_os = "linux")]
        let context = {
            use glutin::platform::unix::HeadlessContextExt;
            match context.clone().build_surfaceless(ctx.window_target) {
                Ok(ctx) => ctx,
                Err(suf_e) => match context.clone().build_headless(ctx.window_target, size_one) {
                    Ok(ctx) => ctx,
                    Err(hea_e) => match context.build_osmesa(size_one) {
                        Ok(ctx) => ctx,
                        Err(osm_e) => panic!(
                            "failed all headless modes supported in linux\nsurfaceless: {:?}\n\nheadless: {:?}\n\n osmesa: {:?}",
                            suf_e, hea_e, osm_e
                        ),
                    },
                },
            }
        };
        #[cfg(not(target_os = "linux"))]
        let context = context
            .build_headless(ctx.window_target, size_one)
            .expect("failed to build headless context");

        let mut context = ctx.gl_manager.manage_headless(id, context);
        let gl_ctx = context.make_current();

        let gl = match gl_ctx.get_api() {
            GApi::OpenGl => unsafe { gl::GlFns::load_with(|symbol| gl_ctx.get_proc_address(symbol) as *const _) },
            GApi::OpenGlEs => unsafe { gl::GlesFns::load_with(|symbol| gl_ctx.get_proc_address(symbol) as *const _) },
            GApi::WebGl => panic!("WebGl is not supported"),
        };

        #[cfg(debug_assertions)]
        let gl = gleam::gl::ErrorCheckingGl::wrap(gl.clone());

        // manually create a surface.
        let rbos = gl.gen_renderbuffers(2);
        let rbos = [rbos[0], rbos[1]];
        let fbo = gl.gen_framebuffers(1)[0];

        Self::resize(&gl, rbos, cfg.size, cfg.scale_factor);

        gl.bind_framebuffer(gl::FRAMEBUFFER, fbo);
        gl.framebuffer_renderbuffer(gl::FRAMEBUFFER, gl::COLOR_ATTACHMENT0, gl::RENDERBUFFER, rbos[0]);
        gl.framebuffer_renderbuffer(gl::FRAMEBUFFER, gl::DEPTH_STENCIL_ATTACHMENT, gl::RENDERBUFFER, rbos[1]);

        let mut text_aa = cfg.text_aa;
        if let TextAntiAliasing::Default = cfg.text_aa {
            text_aa = config::text_aa();
        }

        let mut opts = RendererOptions {
            enable_aa: text_aa != TextAntiAliasing::Mono,
            enable_subpixel_aa: text_aa == TextAntiAliasing::Subpixel,
            renderer_id: Some((gen as u64) << 32 | id as u64),
            //panic_on_gl_error: true,
            // TODO expose more options to the user.
            ..Default::default()
        };
        if let Some(clear_color) = cfg.clear_color {
            opts.clear_color = clear_color;
        }

        let device_size = cfg.size.to_px(cfg.scale_factor).to_wr_device();

        let (renderer, sender) = webrender::Renderer::new(
            Rc::clone(&gl),
            Box::new(Notifier {
                id,
                sender: ctx.app_ev_sender.clone_boxed(),
            }),
            opts,
            None,
        )
        .unwrap();

        let api = sender.create_api();
        let document_id = api.add_document(device_size);

        let pipeline_id = webrender::api::PipelineId(gen, id);

        Self {
            id,
            pipeline_id,
            document_id,
            api,
            size: cfg.size,
            scale_factor: cfg.scale_factor,
            clear_color: cfg.clear_color,

            context,
            gl,
            renderer: Some(renderer),
            rbos,
            fbo,

            frame_id: Epoch::invalid(),
            resized: true,
        }
    }

    fn resize(gl: &Rc<dyn gl::Gl>, rbos: [u32; 2], size: DipSize, scale_factor: f32) {
        let size = size.to_px(scale_factor);
        let width = size.width.0;
        let height = size.height.0;

        gl.bind_renderbuffer(gl::RENDERBUFFER, rbos[0]);
        gl.renderbuffer_storage(gl::RENDERBUFFER, gl::RGBA8, width, height);

        gl.bind_renderbuffer(gl::RENDERBUFFER, rbos[1]);
        gl.renderbuffer_storage(gl::RENDERBUFFER, gl::DEPTH24_STENCIL8, width, height);

        gl.viewport(0, 0, width, height);
    }

    pub fn id(&self) -> WinId {
        self.id
    }

    pub fn size(&self) -> DipSize {
        self.size
    }

    pub fn frame_id(&self) -> Epoch {
        self.frame_id
    }

    pub fn scale_factor(&self) -> f32 {
        self.scale_factor
    }

    pub fn pipeline_id(&self) -> PipelineId {
        self.pipeline_id
    }

    pub fn namespace_id(&self) -> IdNamespace {
        self.api.get_namespace_id()
    }

    pub fn generate_image_key(&self) -> ImageKey {
        self.api.generate_image_key()
    }

    pub fn generate_font_key(&self) -> FontKey {
        self.api.generate_font_key()
    }

    pub fn generate_font_instance_key(&self) -> FontInstanceKey {
        self.api.generate_font_instance_key()
    }

    pub fn set_size(&mut self, size: DipSize, scale_factor: f32) {
        if self.size != size || (self.scale_factor - scale_factor).abs() > 0.001 {
            self.size = size;
            self.scale_factor = scale_factor;
            Self::resize(&self.gl, self.rbos, size, scale_factor);
            self.resized = true;
        }
    }

    pub fn set_text_aa(&mut self, aa: TextAntiAliasing) {
        todo!("need to rebuild the renderer? {:?}", aa)
    }

    pub fn render(&mut self, frame: FrameRequest) {
        self.frame_id = frame.id;

        let mut txn = Transaction::new();
        let display_list = BuiltDisplayList::from_data(
            DisplayListPayload {
                data: frame.display_list.0.into_vec(),
            },
            frame.display_list.1,
        );
        let viewport_size = self.size.to_px(self.scale_factor).to_wr();
        txn.set_display_list(frame.id, self.clear_color, viewport_size, (frame.pipeline_id, display_list), true);
        txn.set_root_pipeline(self.pipeline_id);

        self.push_resize(&mut txn);

        txn.generate_frame(self.frame_id.0 as u64);
        self.api.send_transaction(self.document_id, txn);
    }

    pub fn render_update(&mut self, updates: DynamicProperties) {
        let mut txn = Transaction::new();
        txn.set_root_pipeline(self.pipeline_id);
        txn.update_dynamic_properties(updates);

        self.push_resize(&mut txn);

        txn.generate_frame(self.frame_id.0 as u64);
        self.api.send_transaction(self.document_id, txn);
    }

    fn push_resize(&mut self, txn: &mut Transaction) {
        if self.resized {
            self.resized = false;
            let rect = PxRect::from_size(self.size.to_px(self.scale_factor)).to_wr_device();
            txn.set_document_view(rect);
        }
    }

    pub fn send_transaction(&mut self, txn: Transaction) {
        self.api.send_transaction(self.document_id, txn);
    }

    pub fn redraw(&mut self) {
        let _ctx = self.context.make_current();

        let renderer = self.renderer.as_mut().unwrap();

        renderer.update();
        renderer.render((self.size.to_px(self.scale_factor)).to_wr_device(), 0).unwrap();
        let _ = renderer.flush_pipeline_info();
    }

    pub fn hit_test(&self, point: PxPoint) -> (Epoch, HitTestResult) {
        (
            self.frame_id,
            self.api.hit_test(self.document_id, Some(self.pipeline_id), point.to_wr_world()),
        )
    }

    pub fn read_pixels(&mut self) -> FramePixels {
        let px_size = self.size.to_px(self.scale_factor);
        // `self.gl` is only valid if we are the current context.
        let _ctx = self.context.make_current();
        util::read_pixels_rect(&self.gl, px_size, PxRect::from_size(px_size), self.scale_factor)
    }

    pub fn read_pixels_rect(&mut self, rect: PxRect) -> FramePixels {
        // `self.gl` is only valid if we are the current context.
        let _ctx = self.context.make_current();
        util::read_pixels_rect(&self.gl, self.size.to_px(self.scale_factor), rect, self.scale_factor)
    }
}
impl Drop for ViewHeadless {
    fn drop(&mut self) {
        let _ctx = self.context.make_current();

        self.renderer.take().unwrap().deinit();

        self.gl.delete_framebuffers(&[self.fbo]);
        self.gl.delete_renderbuffers(&self.rbos);
    }
}

struct Notifier {
    id: WinId,
    sender: Box<dyn AppEventSender>,
}
impl RenderNotifier for Notifier {
    fn clone(&self) -> Box<dyn RenderNotifier> {
        Box::new(Self {
            id: self.id,
            sender: self.sender.clone_boxed(),
        })
    }

    fn wake_up(&self, _: bool) {}

    fn new_frame_ready(&self, _: DocumentId, _: bool, _: bool, _: Option<u64>) {
        let _ = self.sender.send(AppEvent::HeadlessFrameReady(self.id));
    }
}
