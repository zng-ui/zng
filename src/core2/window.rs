use super::{AppContext, IntoVar, UiNode, Var, WebRenderEvent, WindowId};
use glutin::event_loop::{EventLoopProxy, EventLoopWindowTarget};
use glutin::window::WindowBuilder;
use glutin::{NotCurrent, WindowedContext};
use rayon::ThreadPool;
use std::borrow::Cow;
use std::sync::Arc;
use webrender::api::{DocumentId, RenderNotifier};

#[derive(Clone)]
struct Notifier {
    window_id: WindowId,
    event_loop: EventLoopProxy<WebRenderEvent>,
}
impl RenderNotifier for Notifier {
    fn clone(&self) -> Box<dyn RenderNotifier> {
        Box::new(Clone::clone(self))
    }

    fn wake_up(&self) {}

    fn new_frame_ready(&self, _: DocumentId, _scrolled: bool, _composite_needed: bool, _: Option<u64>) {
        let _ = self
            .event_loop
            .send_event(WebRenderEvent::NewFrameReady(self.window_id));
    }
}

struct GlWindow {
    context: Option<WindowedContext<NotCurrent>>,
    renderer: webrender::Renderer,

    root: UiRoot,
}

impl GlWindow {
    pub fn update(&mut self, ctx: &mut AppContext) {
        if self.root.title.is_new(&ctx) {
            self.context
                .as_ref()
                .unwrap()
                .window()
                .set_title(self.root.title.get(&ctx));
        }

        self.root.child.update(ctx);
    }
}

trait TestTrait {
    fn gen<'s>(&'s self) -> &'s u32;
}

fn test(title: Box<dyn Var<Cow<'static, str>>>, test: Box<dyn TestTrait>, ctx: &mut AppContext) {
    println!("{}", title.get(ctx))
}

impl GlWindow {
    pub fn new(
        new_window: impl FnOnce(&AppContext) -> UiRoot,
        event_loop: &EventLoopWindowTarget<WebRenderEvent>,
        event_loop_proxy: EventLoopProxy<WebRenderEvent>,
        ui_threads: Arc<ThreadPool>,
    ) -> Self {
        todo!()
    }
}

pub struct UiRoot {
    title: Box<dyn Var<Cow<'static, str>>>,
    child: Box<dyn UiNode>,
}

fn window(child: impl UiNode, title: impl IntoVar<Cow<'static, str>>) -> UiRoot {
    UiRoot {
        title: Box::new(title.into_var()),
        child: Box::new(child),
    }
}
