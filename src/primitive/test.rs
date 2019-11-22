use crate::core::*;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Default)]
pub struct TestChildData {
    pub init_calls: u32,

    pub measure_calls: Vec<LayoutSize>,
    pub measure_return: LayoutSize,

    pub arrange_calls: Vec<LayoutSize>,

    pub render_calls: u32,

    pub keyboard_input_calls: Vec<KeyboardInput>,
    pub window_focused_calls: Vec<bool>,
    pub focus_changed_calls: u32,
    pub mouse_input_calls: Vec<MouseInput>,
    pub mouse_move_calls: Vec<UiMouseMove>,
    pub mouse_entered_calls: u32,
    pub mouse_left_calls: u32,
    pub close_request_calls: u32,

    pub value_changed_calls: u32,
    pub parent_value_changed_calls: u32,
}

pub struct TestChild(Rc<RefCell<TestChildData>>);

impl TestChild {
    pub fn new() -> (TestChild, Rc<RefCell<TestChildData>>) {
        let data = Rc::new(RefCell::new(TestChildData::default()));
        (TestChild(Rc::clone(&data)), data)
    }
}

impl Ui for TestChild {
    fn measure(&mut self, available_size: LayoutSize) -> LayoutSize {
        self.0.borrow_mut().measure_calls.push(available_size);
        self.0.borrow().measure_return
    }

    fn init(&mut self, _values: &mut UiValues, _update: &mut NextUpdate) {
        self.0.borrow_mut().init_calls += 1;
    }

    fn arrange(&mut self, final_size: LayoutSize) {
        self.0.borrow_mut().arrange_calls.push(final_size);
    }

    fn render(&self, _f: &mut NextFrame) {
        self.0.borrow_mut().render_calls += 1;
    }

    fn keyboard_input(&mut self, input: &KeyboardInput, _values: &mut UiValues, _update: &mut NextUpdate) {
        self.0.borrow_mut().keyboard_input_calls.push(input.clone());
    }

    fn window_focused(&mut self, focused: bool, _values: &mut UiValues, _update: &mut NextUpdate) {
        self.0.borrow_mut().window_focused_calls.push(focused);
    }

    fn focus_changed(&mut self, _change: &FocusChange, _values: &mut UiValues, _update: &mut NextUpdate) {
        self.0.borrow_mut().focus_changed_calls += 1;
    }

    fn mouse_input(&mut self, input: &MouseInput, _hits: &Hits, _values: &mut UiValues, _update: &mut NextUpdate) {
        self.0.borrow_mut().mouse_input_calls.push(input.clone());
    }

    fn mouse_move(&mut self, input: &UiMouseMove, _hits: &Hits, _values: &mut UiValues, _update: &mut NextUpdate) {
        self.0.borrow_mut().mouse_move_calls.push(input.clone());
    }

    fn mouse_entered(&mut self, _values: &mut UiValues, _update: &mut NextUpdate) {
        self.0.borrow_mut().mouse_entered_calls += 1;
    }

    fn mouse_left(&mut self, _values: &mut UiValues, _update: &mut NextUpdate) {
        self.0.borrow_mut().mouse_left_calls += 1;
    }

    fn close_request(&mut self, _values: &mut UiValues, _update: &mut NextUpdate) {
        self.0.borrow_mut().close_request_calls += 1;
    }

    fn focus_status(&self) -> Option<FocusStatus> {
        None
    }

    fn point_over(&self, _hits: &Hits) -> Option<LayoutPoint> {
        None
    }

    fn value_changed(&mut self, _values: &mut UiValues, _update: &mut NextUpdate) {
        self.0.borrow_mut().value_changed_calls += 1;
    }

    fn parent_value_changed(&mut self, _values: &mut UiValues, _update: &mut NextUpdate) {
        self.0.borrow_mut().parent_value_changed_calls += 1;
    }
}

pub fn test_next_frame() -> NextFrame {
    use webrender::api::*;
    let pipeline_id = PipelineId::dummy();
    let size = LayoutSize::new(200., 100.);
    let builder = DisplayListBuilder::new(pipeline_id, size);
    let spatial_id = SpatialId::root_reference_frame(pipeline_id);

    NextFrame::new(builder, spatial_id, size, FocusMap::new())
}

pub fn test_next_update() -> NextUpdate {
    unimplemented!()
}

pub fn test_modifiers_state() -> ModifiersState {
    ModifiersState {
        shift: false,
        ctrl: false,
        logo: false,
        alt: false,
    }
}

pub fn test_keyboard_input() -> KeyboardInput {
    KeyboardInput {
        scancode: 0,
        state: ElementState::Pressed,
        virtual_keycode: None,
        modifiers: test_modifiers_state(),
        repeat: false,
    }
}

pub fn test_mouse_input() -> MouseInput {
    MouseInput {
        state: ElementState::Pressed,
        button: MouseButton::Left,
        modifiers: test_modifiers_state(),
        position: LayoutPoint::default(),
    }
}

pub fn test_mouse_move() -> UiMouseMove {
    UiMouseMove {
        position: LayoutPoint::default(),
        modifiers: test_modifiers_state(),
    }
}
