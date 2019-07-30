use euclid::rect;
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

    println!("OpenGL version {}", gl.get_string(gl::VERSION));

    let dpi = w_context.window().get_hidpi_factor();
    println!("Device pixel ratio: {}", dpi);

    let background = ColorF::new(0., 0., 0., 1.);

    let opts = webrender::RendererOptions {
        device_pixel_ratio: dpi as f32,
        clear_color: Some(background),
        ..Default::default()
    };
    let notifier = Box::new(Notifier::new(events_loop.create_proxy()));
    let (mut render, sender) =
        webrender::Renderer::new(gl, notifier, opts, None).expect("Error creating web renderer");

    let framebuffer_size = {
        let size = w_context
            .window()
            .get_inner_size()
            .unwrap()
            .to_physical(dpi);
        DeviceIntSize::new(size.width as i32, size.height as i32)
    };

    let api = sender.create_api();

    let document_id = api.add_document(framebuffer_size, 0);
    let mut epoch = Epoch(0);
    let pipeline_id = PipelineId(0, 0);

    let mut tsn = Transaction::new();
    tsn.set_root_pipeline(pipeline_id);
    api.send_transaction(document_id, tsn);

    events_loop.run_forever(|global_event| {
        let win_event = match global_event {
            glutin::Event::WindowEvent { event, .. } => event,
            _ => return ControlFlow::Continue,
        };

        match win_event {
            glutin::WindowEvent::CloseRequested => return ControlFlow::Break,
             // skip high-frequency events
            glutin::WindowEvent::AxisMotion { .. } |
            glutin::WindowEvent::CursorMoved { .. } => return ControlFlow::Continue,
            _ => {}
        };

        let mut tsn = Transaction::new();

        epoch = increase_epoch(epoch);

        let framebuffer_size = {
            let size = w_context
                .window()
                .get_inner_size()
                .unwrap()
                .to_physical(dpi);
            DeviceIntSize::new(size.width as i32, size.height as i32)
        };
        let layout_size = framebuffer_size.to_f32() / euclid::TypedScale::new(dpi as f32);

        let mut builder = DisplayListBuilder::new(pipeline_id, layout_size);
        builder.push_rect(
            &LayoutPrimitiveInfo::new(rect(80.0, 2.0, 554., 50.)),
            &SpaceAndClipInfo::root_scroll(pipeline_id),
            ColorF::new(1., 0., 0.4, 1.),
        );
        api.set_window_parameters(
            document_id,
            framebuffer_size,
            DeviceIntRect::new(DeviceIntPoint::zero(), framebuffer_size),
            dpi as f32,
        );
        tsn.set_display_list(
            epoch,
            Some(background),
            layout_size,
            builder.finalize(),
            true,
        );

        //tsn.set_root_pipeline(pipeline_id);
        tsn.generate_frame();

        api.send_transaction(document_id, tsn);
        render.update();
        render.render(framebuffer_size).unwrap();

        let _ = render.flush_pipeline_info();
        w_context.swap_buffers().ok();

        ControlFlow::Continue
    });

    render.deinit();
}

fn increase_epoch(old: Epoch) -> Epoch {
    use std::u32;
    const MAX_ID: u32 = u32::MAX - 1;
    match old.0 {
        MAX_ID => Epoch(0),
        other => Epoch(other + 1),
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
