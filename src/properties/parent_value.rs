use crate::core::*;

#[derive(new)]
pub struct SetParentValue<T: Ui, V, R: Value<V>> {
    child: T,
    key: ParentValueKey<V>,
    value: R,
}

#[impl_ui_crate(child)]
impl<T: Ui, V: 'static, R: Value<V>> SetParentValue<T, V, R> {
    #[Ui]
    fn init(&mut self, values: &mut UiValues, update: &mut NextUpdate) {
        let child = &mut self.child;
        values.with_parent_value(self.key, &self.value, |v| child.init(v, update));
    }

    #[Ui]
    fn value_changed(&mut self, values: &mut UiValues, update: &mut NextUpdate) {
        let child = &mut self.child;

        if self.value.touched() {
            values.with_parent_value(self.key, &self.value, |v| child.parent_value_changed(v, update));
        }

        values.with_parent_value(self.key, &self.value, |v| child.value_changed(v, update));
    }

    #[Ui]
    fn parent_value_changed(&mut self, values: &mut UiValues, update: &mut NextUpdate) {
        let child = &mut self.child;
        values.with_parent_value(self.key, &self.value, |v| child.parent_value_changed(v, update));
    }

    #[Ui]
    fn keyboard_input(&mut self, input: &KeyboardInput, values: &mut UiValues, update: &mut NextUpdate) {
        let child = &mut self.child;
        values.with_parent_value(self.key, &self.value, |v| child.keyboard_input(input, v, update));
    }

    #[Ui]
    fn window_focused(&mut self, focused: bool, values: &mut UiValues, update: &mut NextUpdate) {
        let child = &mut self.child;
        values.with_parent_value(self.key, &self.value, |v| child.window_focused(focused, v, update));
    }

    #[Ui]
    fn mouse_input(&mut self, input: &MouseInput, hits: &Hits, values: &mut UiValues, update: &mut NextUpdate) {
        let child = &mut self.child;
        values.with_parent_value(self.key, &self.value, |v| child.mouse_input(input, hits, v, update));
    }

    #[Ui]
    fn mouse_move(&mut self, input: &UiMouseMove, hits: &Hits, values: &mut UiValues, update: &mut NextUpdate) {
        let child = &mut self.child;
        values.with_parent_value(self.key, &self.value, |v| child.mouse_move(input, hits, v, update));
    }

    #[Ui]
    fn mouse_entered(&mut self, values: &mut UiValues, update: &mut NextUpdate) {
        let child = &mut self.child;
        values.with_parent_value(self.key, &self.value, |v| child.mouse_entered(v, update));
    }

    #[Ui]
    fn mouse_left(&mut self, values: &mut UiValues, update: &mut NextUpdate) {
        let child = &mut self.child;
        values.with_parent_value(self.key, &self.value, |v| child.mouse_left(v, update));
    }

    #[Ui]
    fn close_request(&mut self, values: &mut UiValues, update: &mut NextUpdate) {
        let child = &mut self.child;
        values.with_parent_value(self.key, &self.value, |v| child.close_request(v, update));
    }
}

pub fn set_parent_val<T: 'static>(child: impl Ui, key: ParentValueKey<T>, value: impl IntoValue<T>) -> impl Ui {
    SetParentValue::new(child, key, value.into_value())
}
