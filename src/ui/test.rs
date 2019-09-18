use super::*;

#[derive(Default)]
pub struct TestChildData {
    pub init_calls: u32,

    pub measure_calls: Vec<LayoutSize>,
    pub measure_return: LayoutSize,

    pub arrange_calls: Vec<LayoutSize>,

    pub render_calls: u32,

    pub keyboard_input_calls: Vec<KeyboardInput>,
    pub focused_calls: Vec<bool>,
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

impl UiLeaf for TestChild {
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

    fn focused(&mut self, focused: bool, _values: &mut UiValues, _update: &mut NextUpdate) {
        self.0.borrow_mut().focused_calls.push(focused);
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
delegate_ui!(UiLeaf, TestChild);

#[macro_export]
macro_rules! ui_leaf_tests {
    ($new: expr) => {
        use $crate::ui::*;

        #[allow(unused)]
        fn is_ui_leaf() -> impl UiLeaf {
            $new
        }

        fn new() -> impl Ui {
            $new
        }

        #[test]
        fn measures_to_finite_size() {
            use std::f32::INFINITY;
            let mut ui = new();
            let t = ui.measure(LayoutSize::new(INFINITY, INFINITY));

            assert!(
                t.width.is_finite() && t.height.is_finite(),
                "`measure` return size must be finite. \n\
                 It receives infinity to indicate that the parent \
                 container will size to wathever content size, so, for infinite dimensions, a UiLeaf must \
                 return its full content size, or 0 if it only fills the available space."
            );
        }

        #[test]
        fn layout_and_render_fast() {
            // TODO figure timing here
        }
    };
}

pub fn test_next_frame() -> NextFrame {
    use webrender::api::*;
    let pipeline_id = PipelineId::dummy();
    let size = LayoutSize::new(200., 100.);
    let builder = DisplayListBuilder::new(pipeline_id, size);
    let spatial_id = SpatialId::root_reference_frame(pipeline_id);

    NextFrame::new(builder, spatial_id, size)
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

#[macro_export]
macro_rules! ui_container_tests {
    ($new: expr) => {
        use $crate::ui::test::*;
        use $crate::ui::*;

        fn new_ui_container(child: TestChild) -> impl UiContainer {
            let new = $new;
            new(child)
        }

        fn new_ui(child: TestChild) -> impl Ui {
            let new = $new;
            new(child)
        }

        #[test]
        fn child() {
            let (c, t) = TestChild::new();
            let ui = new_ui_container(c);
            ui.child().render(&mut test_next_frame());

            let reached_child = t.borrow().render_calls == 1;

            assert!(
                reached_child,
                "`UiContainer::child` must return a reference to the child."
            );
        }

        #[test]
        fn child_mut() {
            let (c, t) = TestChild::new();
            let mut ui = new_ui_container(c);
            ui.child_mut().arrange(LayoutSize::new(100., 200.));

            let reached_child = t.borrow().arrange_calls.len() == 1;

            assert!(
                reached_child,
                "`UiContainer::child_mut` must return a reference to the child."
            );
        }

        #[test]
        fn into_child() {
            let (c, t) = TestChild::new();
            let ui = new_ui_container(c);
            ui.into_child().arrange(LayoutSize::new(100., 200.));

            let reached_child = t.borrow().arrange_calls.len() == 1;

            assert!(reached_child, "`UiContainer::into_child` must return the child.");
        }

        macro_rules! propagation_test {
            ($test_name:ident, |$ui:ident| $ui_call:expr, |$t: ident| $call_count: expr, $message: expr) => {
                #[test]
                fn $test_name() {
                    let (c, t) = TestChild::new();
                    let mut $ui = new_ui(c);

                    $ui_call;

                    let $t = t.borrow();

                    assert_eq!($call_count, 1, $message);
                }
            };
        }

        #[test]
        fn render_propagation() {
            let (c, t) = TestChild::new();
            let ui = new_ui(c);

            ui.render(&mut test_next_frame());

            let t = t.borrow();

            assert_eq!(
                t.render_calls, 1,
                "UiContainer::render` must call `self.child().render(..)`."
            );
        }

        propagation_test!(
            init_propagation,
            |ui| ui.init(&mut UiValues::new(), &mut test_next_update()),
            |t| t.init_calls,
            "`UiContainer::init` must call `self.child_mut().init(..)`."
        );

        propagation_test!(
            measure_propagation,
            |ui| ui.measure(LayoutSize::new(200., 100.)),
            |t| t.measure_calls.len(),
            "`UiContainer::measure` must call `self.child_mut().measure(..)`."
        );

        propagation_test!(
            arrange_propagation,
            |ui| ui.arrange(LayoutSize::new(200., 100.)),
            |t| t.arrange_calls.len(),
            "`UiContainer::arrange` must call `self.child_mut().arrange(..)`."
        );

        propagation_test!(
            keyboard_input_propagation,
            |ui| ui.keyboard_input(
                &test_keyboard_input(),
                &mut UiValues::new(),
                &mut test_next_update()
            ),
            |t| t.keyboard_input_calls.len(),
            "`UiContainer::keyboard_input` must call `self.child_mut().keyboard_input(..)`."
        );

        propagation_test!(
            focused_input_propagation,
            |ui| ui.focused(false, &mut UiValues::new(), &mut test_next_update()),
            |t| t.keyboard_input_calls.len(),
            "`UiContainer::focused` must call `self.child_mut().focused(..)`."
        );

        propagation_test!(
            mouse_input_propagation,
            |ui| ui.mouse_input(
                &test_mouse_input(),
                &Hits::default(),
                &mut UiValues::new(),
                &mut test_next_update()
            ),
            |t| t.mouse_input_calls.len(),
            "`UiContainer::mouse_input` must call `self.child_mut().mouse_input(..)`."
        );

        propagation_test!(
            mouse_move_propagation,
            |ui| ui.mouse_move(
                &test_mouse_move(),
                &Hits::default(),
                &mut UiValues::new(),
                &mut test_next_update()
            ),
            |t| t.mouse_move_calls.len(),
            "`UiContainer::mouse_move` must call `self.child_mut().mouse_move(..)`."
        );

        propagation_test!(
            mouse_entered_propagation,
            |ui| ui.mouse_entered(&mut UiValues::new(), &mut test_next_update()),
            |t| t.mouse_entered_calls,
            "`UiContainer::mouse_entered` must call `self.child_mut().mouse_entered(..)`."
        );

        propagation_test!(
            mouse_left_propagation,
            |ui| ui.mouse_left(&mut UiValues::new(), &mut test_next_update()),
            |t| t.mouse_left_calls,
            "`UiContainer::mouse_left` must call `self.child_mut().mouse_left(..)`."
        );

        propagation_test!(
            close_request_propagation,
            |ui| ui.close_request(&mut UiValues::new(), &mut test_next_update()),
            |t| t.close_request_calls,
            "`UiContainer::close_request` must call `self.child_mut().close_request(..)`."
        );

        propagation_test!(
            value_changed_propagation,
            |ui| ui.value_changed(&mut UiValues::new(), &mut test_next_update()),
            |t| t.value_changed_calls,
            "`UiContainer::value_changed` must call `self.child_mut().value_changed(..)`."
        );

        propagation_test!(
            parent_value_changed_propagation,
            |ui| ui.parent_value_changed(&mut UiValues::new(), &mut test_next_update()),
            |t| t.parent_value_changed_calls,
            "`UiContainer::parent_value_changed` must call `self.child_mut().parent_value_changed(..)`."
        );
    };
}

#[macro_export]
macro_rules! ui_multi_container_tests {
    ($new: expr) => {
        use $crate::ui::test::*;
        use $crate::ui::*;

        #[allow(unused)]
        fn is_ui_multi_container<'a>(children: Vec<TestChild>) -> impl UiMultiContainer<'a> {
            let new = $new;
            new(children)
        }

        fn new_ui(children: Vec<TestChild>) -> impl Ui {
            let new = $new;
            new(children)
        }

        fn new_children() -> (Vec<TestChild>, Vec<Rc<RefCell<TestChildData>>>) {
            let mut cs = vec![];
            let mut ts = vec![];

            for _ in 0..2 {
                let (c, t) = TestChild::new();
                cs.push(c);
                ts.push(t);
            }

            (cs, ts)
        }

        macro_rules! propagation_test {
            ($test_name:ident, |$ui:ident| $ui_call:expr, |$t: ident| $call_count: expr, $message: expr) => {
                #[test]
                fn $test_name() {
                    let (cs, ts) = new_children();
                    let mut $ui = new_ui(cs);

                    $ui_call;

                    let propagated_all = ts.iter().all(|t| {
                        let $t = t.borrow();
                        $call_count == 1
                    });

                    assert!(propagated_all, $message);
                }
            };
        }

        #[test]
        fn render_propagation() {
            let (cs, ts) = new_children();
            let ui = new_ui(cs);

            ui.render(&mut test_next_frame());

            let propagated_all = ts.iter().all(|t| {
                let t = t.borrow();
                t.render_calls == 1
            });

            assert!(
                propagated_all,
                "UiMultiContainer::render` must call `render` in all children."
            );
        }

        propagation_test!(
            init_propagation,
            |ui| ui.init(&mut UiValues::new(), &mut test_next_update()),
            |t| t.init_calls,
            "`UiMultiContainer::init` must call init` in all children."
        );

        propagation_test!(
            measure_propagation,
            |ui| ui.measure(LayoutSize::new(200., 100.)),
            |t| t.measure_calls.len(),
            "`UiMultiContainer::measure` must call `measure` in all children."
        );

        propagation_test!(
            arrange_propagation,
            |ui| ui.arrange(LayoutSize::new(200., 100.)),
            |t| t.arrange_calls.len(),
            "`UiMultiContainer::arrange` must call `arrange` in all children."
        );

        propagation_test!(
            keyboard_input_propagation,
            |ui| ui.keyboard_input(
                &test_keyboard_input(),
                &mut UiValues::new(),
                &mut test_next_update()
            ),
            |t| t.keyboard_input_calls.len(),
            "`UiMultiContainer::keyboard_input` must call `keyboard_input` in all children."
        );

        propagation_test!(
            focused_input_propagation,
            |ui| ui.focused(false, &mut UiValues::new(), &mut test_next_update()),
            |t| t.keyboard_input_calls.len(),
            "`UiMultiContainer::focused` must call `focused` in all children."
        );

        propagation_test!(
            mouse_input_propagation,
            |ui| ui.mouse_input(
                &test_mouse_input(),
                &Hits::default(),
                &mut UiValues::new(),
                &mut test_next_update()
            ),
            |t| t.mouse_input_calls.len(),
            "`UiMultiContainer::mouse_input` must call `mouse_input` in all children."
        );

        propagation_test!(
            mouse_move_propagation,
            |ui| ui.mouse_move(
                &test_mouse_move(),
                &Hits::default(),
                &mut UiValues::new(),
                &mut test_next_update()
            ),
            |t| t.mouse_move_calls.len(),
            "`UiMultiContainer::mouse_move` must call `mouse_move` in all children."
        );

        propagation_test!(
            mouse_entered_propagation,
            |ui| ui.mouse_entered(&mut UiValues::new(), &mut test_next_update()),
            |t| t.mouse_entered_calls,
            "`UiContainer::mouse_entered` must call `mouse_entered` in all children."
        );

        propagation_test!(
            mouse_left_propagation,
            |ui| ui.mouse_left(&mut UiValues::new(), &mut test_next_update()),
            |t| t.mouse_left_calls,
            "`UiMultiContainer::mouse_left` must call `mouse_left` in all children."
        );

        propagation_test!(
            close_request_propagation,
            |ui| ui.close_request(&mut UiValues::new(), &mut test_next_update()),
            |t| t.close_request_calls,
            "`UiMultiContainer::close_request` must call `close_request` in all children."
        );

        propagation_test!(
            value_changed_propagation,
            |ui| ui.value_changed(&mut UiValues::new(), &mut test_next_update()),
            |t| t.value_changed_calls,
            "`UiMultiContainer::value_changed` must call `value_changed` in all children."
        );

        propagation_test!(
            parent_value_changed_propagation,
            |ui| ui.parent_value_changed(&mut UiValues::new(), &mut test_next_update()),
            |t| t.parent_value_changed_calls,
            "`UiMultiContainer::parent_value_changed` must call `parent_value_changed` in all children."
        );
    };
}
