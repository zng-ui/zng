use gleam::gl;
use glutin::dpi::LogicalSize;
use glutin::ControlFlow;
use webrender::api::*;

pub fn show_window() {
    let mut events_loop = glutin::EventsLoop::new();

    let w_context = glutin::ContextBuilder::new()
        .with_gl(glutin::GlRequest::GlThenGles {
            opengl_version: (3, 2),
            opengles_version: (3, 0),
        })
        .build_windowed(
            glutin::WindowBuilder::new()
                .with_title("Test")
                .with_dimensions(LogicalSize::new(800.0, 600.0)),
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

    println!("OpenGL version {}", gl.get_string(gl::VERSION));

    let device_pixel_ratio = w_context.window().get_hidpi_factor() as f32;
    println!("Device pixel ratio: {}", device_pixel_ratio);

    let background = ColorF::new(0., 0., 0., 1.);

    let opts = webrender::RendererOptions {
        clear_color: Some(background),
        ..Default::default()
    };
    let notifier = Box::new(Notifier::new(events_loop.create_proxy()));
    let (mut renderer, sender) =
        webrender::Renderer::new(gl, notifier, opts, None).expect("Error creating web renderer");

    let device_size = {
        let size = w_context
            .window()
            .get_inner_size()
            .unwrap()
            .to_physical(device_pixel_ratio as f64);
        DeviceIntSize::new(size.width as i32, size.height as i32)
    };

    let api = sender.create_api();

    let document_id = api.add_document(device_size, 0);
    let epoch = Epoch(0);
    let pipeline_id = PipelineId(0, 0);
    let layout_size = LayoutSize::new(
        device_size.width as f32 / device_pixel_ratio,
        device_size.height as f32 / device_pixel_ratio,
    );
    let mut builder = DisplayListBuilder::new(pipeline_id, layout_size);
    let mut tsn = Transaction::new();

    tsn.set_display_list(
        epoch,
        Some(background),
        layout_size,
        builder.finalize(),
        true,
    );
    tsn.set_root_pipeline(pipeline_id);
    tsn.generate_frame();
    api.send_transaction(document_id, tsn);

    events_loop.run_forever(|global_event| {
        let mut tsn = Transaction::new();

        let win_event = match global_event {
            glutin::Event::WindowEvent { event, .. } => event,
            _ => return ControlFlow::Continue,
        };

        match win_event {
            glutin::WindowEvent::CloseRequested => return ControlFlow::Break,
            _ => {}
        };

        api.send_transaction(document_id, tsn);
        renderer.update();
        renderer.render(device_size).unwrap();
        let _ = renderer.flush_pipeline_info();
        w_context.swap_buffers().ok();

        ControlFlow::Continue
    });

    renderer.deinit();
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
