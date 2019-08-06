use crate::button::Button;
use gleam::gl;
use glutin::dpi::LogicalSize;
use glutin::NotCurrent;
use webrender::api::*;
use webrender::DebugFlags;

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
        #[cfg(not(target_os = "android"))]
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

pub struct RenderContext {
    api: RenderApi,
    document_id: DocumentId,
    epoch: Epoch,
    pipeline_id: PipelineId,
    renderer: webrender::Renderer,
}
impl RenderContext {
    pub fn hit_test(&self, world_point: WorldPoint, tag: (u64, u16)) -> bool {
        self.api
            .hit_test(
                self.document_id,
                Some(self.pipeline_id),
                world_point,
                HitTestFlags::FIND_ALL,
            )
            .items
            .into_iter()
            .any(|r| r.tag == tag)
    }
}

pub struct Window {
    button: Button,
    context: Option<glutin::WindowedContext<NotCurrent>>,
    events_loop: glutin::EventsLoop, //TODO: share events loop?
    name: &'static str,
    render_context: RenderContext,
}

impl Window {
    pub fn new(name: &'static str, clear_color: ColorF) -> Self {
        let events_loop = glutin::EventsLoop::new();
        let window_builder = glutin::WindowBuilder::new()
            .with_title(name)
            .with_multitouch()
            .with_dimensions(LogicalSize::new(800., 600.));

        let context = glutin::ContextBuilder::new()
            .with_gl(glutin::GlRequest::GlThenGles {
                opengl_version: (3, 2),
                opengles_version: (3, 0),
            })
            .build_windowed(window_builder, &events_loop)
            .unwrap();

        let context = unsafe { context.make_current().unwrap() };

        let gl = match context.get_api() {
            glutin::Api::OpenGl => unsafe {
                gl::GlFns::load_with(|symbol| context.get_proc_address(symbol) as *const _)
            },
            glutin::Api::OpenGlEs => unsafe {
                gl::GlesFns::load_with(|symbol| context.get_proc_address(symbol) as *const _)
            },
            glutin::Api::WebGl => unimplemented!(),
        };

        let device_pixel_ratio = context.window().get_hidpi_factor() as f32;

        let opts = webrender::RendererOptions {
            device_pixel_ratio,
            clear_color: Some(clear_color),
            ..webrender::RendererOptions::default()
        };

        let device_size = {
            let size = context
                .window()
                .get_inner_size()
                .unwrap()
                .to_physical(device_pixel_ratio as f64);
            DeviceIntSize::new(size.width as i32, size.height as i32)
        };
        let notifier = Box::new(Notifier::new(events_loop.create_proxy()));
        let (renderer, sender) =
            webrender::Renderer::new(gl.clone(), notifier, opts, None).unwrap();
        let api = sender.create_api();
        let document_id = api.add_document(device_size, 0);

        let epoch = Epoch(0);
        let pipeline_id = PipelineId(0, 0);
        let txn = Transaction::new();

        api.send_transaction(document_id, txn);

        Window {
            button: Button::default(),
            context: Some(unsafe { context.make_not_current().unwrap() }),
            events_loop,
            name,
            render_context: RenderContext {
                api,
                document_id,
                epoch,
                pipeline_id,
                renderer,
            },
        }
    }

    pub fn tick(&mut self) -> bool {
        let mut do_exit = false;
        let my_name = &self.name;
        let button = &mut self.button;
        let render_context = &mut self.render_context;

        self.events_loop
            .poll_events(|global_event| match global_event {
                glutin::Event::WindowEvent { event, .. } => match event {
                    glutin::WindowEvent::CloseRequested
                    | glutin::WindowEvent::KeyboardInput {
                        input:
                            glutin::KeyboardInput {
                                virtual_keycode: Some(glutin::VirtualKeyCode::Escape),
                                ..
                            },
                        ..
                    } => do_exit = true,
                    glutin::WindowEvent::KeyboardInput {
                        input:
                            glutin::KeyboardInput {
                                state: glutin::ElementState::Pressed,
                                virtual_keycode: Some(glutin::VirtualKeyCode::P),
                                ..
                            },
                        ..
                    } => {
                        println!("set flags {}", my_name);
                        render_context
                            .api
                            .send_debug_cmd(DebugCommand::SetFlags(DebugFlags::PROFILER_DBG))
                    }
                    _ => {
                        button.on_event(&event, render_context);
                    }
                },
                _ => {}
            });
        if do_exit {
            return true;
        }

        let context = unsafe { self.context.take().unwrap().make_current().unwrap() };
        let device_pixel_ratio = context.window().get_hidpi_factor() as f32;
        let device_size = {
            let size = context
                .window()
                .get_inner_size()
                .unwrap()
                .to_physical(device_pixel_ratio as f64);
            DeviceIntSize::new(size.width as i32, size.height as i32)
        };
        let layout_size = device_size.to_f32() / euclid::TypedScale::new(device_pixel_ratio);
        let mut txn = Transaction::new();
        let mut builder = DisplayListBuilder::new(render_context.pipeline_id, layout_size);
        self.button.render(render_context.pipeline_id, &mut builder);

        txn.set_display_list(
            render_context.epoch,
            None,
            layout_size,
            builder.finalize(),
            true,
        );
        txn.set_root_pipeline(render_context.pipeline_id);
        txn.generate_frame();
        render_context
            .api
            .send_transaction(render_context.document_id, txn);

        render_context.renderer.update();
        render_context.renderer.render(device_size).unwrap();
        context.swap_buffers().ok();

        self.context = Some(unsafe { context.make_not_current().unwrap() });

        false
    }

    pub fn deinit(mut self) {
        let context = unsafe { self.context.take().unwrap().make_current().unwrap() };
        self.render_context.renderer.deinit();
        unsafe { context.make_not_current().unwrap() };
    }
}
