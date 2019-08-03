use euclid::rect;
use gleam::gl;
use glutin::dpi::LogicalSize;
use webrender::api::*;

pub struct Window {
    events_loop: glutin::EventsLoop,
    w_context: glutin::ContextWrapper<glutin::PossiblyCurrent, glutin::Window>,
    renderer: webrender::Renderer,
    renderer_sender: RenderApiSender,
    background: ColorF,
    render: Option<RenderData>,
    content: Option<Button>,
}

struct RenderData {
    api: RenderApi,
    doc: DocumentId,
    epoch: Epoch,
    pipe: PipelineId,
}

impl RenderData {
    pub fn increase_epoch(&mut self) {
        use std::u32;
        const MAX_ID: u32 = u32::MAX - 1;
        self.epoch = match self.epoch.0 {
            MAX_ID => Epoch(0),
            other => Epoch(other + 1),
        };
    }
}

struct Button {
    tag: (u64, u16),
    is_hovered: bool,
}

impl Button {
    pub fn on_event(&mut self, event: &glutin::WindowEvent, render: &RenderData) -> bool {
        match event {
            glutin::WindowEvent::CursorMoved { position, .. } => {
                let r = render.api.hit_test(
                    render.doc,
                    Some(render.pipe),
                    WorldPoint::new(position.x as f32, position.y as f32),
                    HitTestFlags::FIND_ALL,
                );

                let new_is_hovered = r.items.into_iter().any(|r| r.tag == self.tag);

                if self.is_hovered != new_is_hovered {
                    self.is_hovered = new_is_hovered;
                    return true;
                }
            }
            _ => {}
        }
        false
    }

    pub fn render(&self, render: &RenderData, builder: &mut DisplayListBuilder) {
        let mut layour_primitive_info = LayoutPrimitiveInfo::new(rect(80.0, 2.0, 554., 50.));
        layour_primitive_info.tag = Some(self.tag);
        builder.push_rect(
            &layour_primitive_info,
            &SpaceAndClipInfo::root_scroll(render.pipe),
            if self.is_hovered {
                ColorF::new(0., 1., 0.4, 1.)
            } else {
                ColorF::new(1., 0., 0.4, 1.)
            },
        );
    }
}

impl Window {
    pub fn show() -> Window {
        let events_loop = glutin::EventsLoop::new();
        let w_context = glutin::ContextBuilder::new()
            .with_gl(glutin::GlRequest::GlThenGles {
                opengl_version: (3, 2),
                opengles_version: (3, 0),
            })
            .build_windowed(
                glutin::WindowBuilder::new()
                    .with_title("Test")
                    .with_dimensions(LogicalSize::new(800.0, 600.0))
                    .with_multitouch(),
                &events_loop,
            )
            .expect("Error building windowed GL context");

        let w_context = unsafe {
            w_context
                .make_current()
                .expect("Error making context current")
        };

        let gl = match w_context.get_api() {
            glutin::Api::OpenGl => unsafe {
                gl::GlFns::load_with(|addr| w_context.get_proc_address(addr) as *const _)
            },
            glutin::Api::OpenGlEs => unsafe {
                gl::GlesFns::load_with(|addr| w_context.get_proc_address(addr) as *const _)
            },
            _ => unreachable!(),
        };

        let dpi = w_context.window().get_hidpi_factor();

        let background = ColorF::new(0., 0., 0., 1.);

        let opts = webrender::RendererOptions {
            device_pixel_ratio: dpi as f32,
            clear_color: Some(background),
            ..Default::default()
        };
        let notifier = Box::new(Notifier::new(events_loop.create_proxy()));
        let (renderer, renderer_sender) = webrender::Renderer::new(gl, notifier, opts, None)
            .expect("Error creating web renderer");

        Window {
            events_loop,
            w_context,
            renderer,
            renderer_sender,
            background,
            render: None,
            content: None,
        }
    }

    fn init_render(&mut self) {
        let api = self.renderer_sender.create_api();
        let doc = api.add_document(self.framebuffer_size(), 0);
        let epoch = Epoch(0);
        let pipe = PipelineId(0, 0);
        let mut tsn = Transaction::new();
        tsn.set_root_pipeline(pipe);
        tsn.generate_frame();
        api.send_transaction(doc, tsn);

        self.render = Some(RenderData {
            api,
            doc,
            epoch,
            pipe,
        });
    }

    fn init_content(&mut self) {
        self.content = Some(Button {
            tag: (0, 0),
            is_hovered: false,
        });
    }

    pub fn run(mut self) {
        self.init_render();
        self.init_content();

        let mut events_loop = self.events_loop;
        let mut content = self.content.unwrap();
        let mut render = self.render.unwrap();
        let w_context = self.w_context;

        let mut run = true;
        let mut first_render = true;

        loop {
            let mut render_content = first_render;
            first_render = false;

            events_loop.poll_events(|event| {
                let w_event = match event {
                    glutin::Event::WindowEvent { event, .. } => event,
                    _ => return,
                };

                match w_event {
                    glutin::WindowEvent::CloseRequested => {
                        run = false;
                        return;
                    }
                    glutin::WindowEvent::Resized { .. } => render_content = true,
                    e => {
                        if content.on_event(&e, &render) {
                            render_content = true
                        }
                    }
                }
            });

            if !run {
                break;
            }

            let dpi = w_context.window().get_hidpi_factor();
            let fsz = w_context
                .window()
                .get_inner_size()
                .unwrap()
                .to_physical(dpi);
            let fsz = DeviceIntSize::new(fsz.width as i32, fsz.height as i32);

            if true {
                let dpi = dpi as f32;

                render.increase_epoch();

                let layout_size = fsz.to_f32() / euclid::TypedScale::new(dpi);

                let mut tsn = Transaction::new();

                let mut builder = DisplayListBuilder::new(render.pipe, layout_size);

                content.render(&render, &mut builder);

                render.api.set_window_parameters(
                    render.doc,
                    fsz,
                    DeviceIntRect::new(DeviceIntPoint::zero(), fsz),
                    dpi,
                );

                tsn.set_display_list(
                    render.epoch,
                    Some(self.background),
                    layout_size,
                    builder.finalize(),
                    true,
                );

                tsn.generate_frame();

                render.api.send_transaction(render.doc, tsn);
                self.renderer.update();
            }

            self.renderer.render(fsz).unwrap();
            self.renderer.flush_pipeline_info();
            w_context.swap_buffers().unwrap();
        }

        self.renderer.deinit();
    }

    fn dpi(&self) -> f64 {
        self.w_context.window().get_hidpi_factor()
    }

    fn framebuffer_size(&self) -> DeviceIntSize {
        let size = self
            .w_context
            .window()
            .get_inner_size()
            .unwrap()
            .to_physical(self.dpi());
        DeviceIntSize::new(size.width as i32, size.height as i32)
    }
}

struct Notifier {
    events_proxy: glutin::EventsLoopProxy,
}

impl Notifier {
    fn new(events_proxy: glutin::EventsLoopProxy) -> Notifier {
        Notifier { events_proxy }
    }
}

impl RenderNotifier for Notifier {
    fn clone(&self) -> Box<dyn RenderNotifier> {
        Box::new(Notifier {
            events_proxy: self.events_proxy.clone(),
        })
    }

    fn wake_up(&self) {
        let _ = self.events_proxy.wakeup();
    }

    fn new_frame_ready(
        &self,
        _: DocumentId,
        _scrolled: bool,
        _composite_needed: bool,
        _render_time: Option<u64>,
    ) {
        self.wake_up();
    }
}
