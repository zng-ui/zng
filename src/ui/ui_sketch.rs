pub struct RenderContext;

#[derive(Default, Clone, Copy)]
pub struct LayoutSize {
    width: f32,
    height: f32,
}

impl LayoutSize {
    pub fn max(&self, other: LayoutSize) -> LayoutSize {
        unimplemented!()
    }
}

pub struct KeyboardInput {}

enum Invalidate {
    Render,
    Layout,
}

enum CloseRequest {
    Window,
    App,
}

#[derive(Clone, Copy)]
pub struct WorkKey(usize);

pub struct UpdateContext {
    next_work_key: WorkKey,

    invalidate: Option<Invalidate>,
    close: Option<CloseRequest>,
    cancel_close: bool,

    new_window_title: Option<String>,

    new_work: Vec<(WorkKey, Box<dyn FnOnce()>)>,
    cancel_work: Vec<WorkKey>
}

impl UpdateContext {
    pub fn invalidate_layout(&mut self) {
        self.invalidate = Some(Invalidate::Layout);
    }

    pub fn invalidate_render(&mut self) {
        if let Some(Invalidate::Layout) = self.invalidate {
            return;
        }
        self.invalidate = Some(Invalidate::Render);
    }

    pub fn close_window(&mut self) {
        if let Some(CloseRequest::App) = self.close {
            return;
        }
        self.close = Some(CloseRequest::Window);
    }

    pub fn close_app(&mut self) {
        self.close = Some(CloseRequest::App);
    }

    pub fn cancel_close(&mut self) {
        self.cancel_close = true;
    }

    pub fn set_window_title(&mut self, title: String) {
        self.new_window_title = Some(title);
    }

    pub fn start_work(&mut self, work: impl FnOnce() + 'static) -> WorkKey {
        let key = self.next_work_key;
        self.new_work.push((key, Box::new(work)));
        self.next_work_key = WorkKey(key.0.wrapping_add(1));
        key
    }

    pub fn cancel_work(&mut self, work_key: WorkKey) {
        self.cancel_work.push(work_key)
    }
}

pub trait Ui {
    type Child: Ui;

    fn for_each_child(&mut self, action: impl FnMut(&mut Self::Child));

    fn measure(&mut self, available_size: LayoutSize) -> LayoutSize {
        let mut desired_size = LayoutSize::default();
        let mut have_child = false;

        self.for_each_child(|c| {
            have_child = true;
            let child_desired_size = c.measure(available_size);
            desired_size = desired_size.max(child_desired_size);
        });

        if have_child {
            desired_size
        } else {
            desired_size = available_size;
            if desired_size.width.is_infinite() {
                desired_size.width = 0.;
            }
            if desired_size.height.is_infinite() {
                desired_size.height = 0.;
            }
            desired_size
        }
    }

    fn arrange(&mut self, final_size: LayoutSize) {
        self.for_each_child(|c| c.arrange(final_size));
    }

    fn render(&mut self, rc: &mut RenderContext) {
        self.for_each_child(|c| c.render(rc));
    }

    fn keyboard_input(&mut self, input: &KeyboardInput, update: &mut UpdateContext) {
        self.for_each_child(|c| c.keyboard_input(input, update));
    }

    fn close_request(&mut self, update: &mut UpdateContext) {
        self.for_each_child(|c| c.close_request(update));
    }

    fn as_any(self) -> AnyUi
    where
        Self: Sized + 'static,
    {
        AnyUi::new(self)
    }
}

impl Ui for () {
    type Child = ();
    fn for_each_child(&mut self, _: impl FnMut(&mut Self::Child)) {}

    fn measure(&mut self, _: LayoutSize) -> LayoutSize {
        LayoutSize::default()
    }

    fn arrange(&mut self, _: LayoutSize) {}
    fn render(&mut self, _: &mut RenderContext) {}
    fn keyboard_input(&mut self, _: &KeyboardInput, _: &mut UpdateContext) {}
    fn close_request(&mut self, _: &mut UpdateContext) {}
}

mod any_ui {
    use super::*;
    use std::any::Any;

    pub trait UiFns: Any {
        fn measure(&mut self, _: LayoutSize) -> LayoutSize;
        fn arrange(&mut self, _: LayoutSize);
        fn render(&mut self, _: &mut RenderContext);
        fn keyboard_input(&mut self, input: &KeyboardInput, update: &mut UpdateContext);
        fn close_request(&mut self, update: &mut UpdateContext);
    }

    impl<T: Ui + 'static> UiFns for T {
        fn measure(&mut self, available_size: LayoutSize) -> LayoutSize {
            Ui::measure(self, available_size)
        }

        fn arrange(&mut self, final_size: LayoutSize) {
            Ui::arrange(self, final_size)
        }

        fn render(&mut self, rc: &mut RenderContext) {
            Ui::render(self, rc)
        }

        fn keyboard_input(&mut self, input: &KeyboardInput, update: &mut UpdateContext) {
            Ui::keyboard_input(self, input, update)
        }

        fn close_request(&mut self, update: &mut UpdateContext) {
            Ui::close_request(self, update)
        }
    }
}

pub struct AnyUi {
    ui: Box<dyn any_ui::UiFns>,
}

impl AnyUi {
    fn new<T: any_ui::UiFns>(ui: T) -> Self {
        Self { ui: Box::new(ui) }
    }
}

impl Ui for AnyUi {
    type Child = ();

    fn for_each_child(&mut self, _: impl FnMut(&mut Self::Child)) {
        panic!("Ui::for_each_child must not be called directly")
    }

    fn as_any(self) -> AnyUi {
        self
    }

    fn measure(&mut self, available_size: LayoutSize) -> LayoutSize {
        self.ui.measure(available_size)
    }

    fn arrange(&mut self, final_size: LayoutSize) {
        self.ui.arrange(final_size)
    }

    fn render(&mut self, rc: &mut RenderContext) {
        self.ui.render(rc)
    }

    fn keyboard_input(&mut self, input: &KeyboardInput, update: &mut UpdateContext) {
        self.ui.keyboard_input(input, update)
    }

    fn close_request(&mut self, update: &mut UpdateContext) {
        self.ui.close_request(update)
    }
}

struct Container<T: Ui> {
    child: T,
}

impl<T: Ui> Ui for Container<T> {
    type Child = T;

    fn for_each_child(&mut self, mut action: impl FnMut(&mut Self::Child)) {
        action(&mut self.child);
    }
}
