use crate::core::*;

#[doc(hidden)]
pub struct UiItem<U: Ui> {
    child: U,
    id: UiItemId,
}

#[impl_ui_crate(child)]
impl<U: Ui> Ui for UiItem<U> {
    fn init(&mut self, values: &mut UiValues, update: &mut NextUpdate) {
        let child = &mut self.child;
        values.item_scope(self.id, |v| child.init(v, update));
    }

    fn value_changed(&mut self, values: &mut UiValues, update: &mut NextUpdate) {
        let child = &mut self.child;
        values.item_scope(self.id, |v| child.value_changed(v, update));
    }

    fn parent_value_changed(&mut self, values: &mut UiValues, update: &mut NextUpdate) {
        let child = &mut self.child;
        values.item_scope(self.id, |v| child.parent_value_changed(v, update));
    }

    fn keyboard_input(&mut self, input: &KeyboardInput, values: &mut UiValues, update: &mut NextUpdate) {
        let child = &mut self.child;
        values.item_scope(self.id, |v| child.keyboard_input(input, v, update));
    }

    fn window_focused(&mut self, focused: bool, values: &mut UiValues, update: &mut NextUpdate) {
        let child = &mut self.child;
        values.item_scope(self.id, |v| child.window_focused(focused, v, update));
    }

    fn mouse_input(&mut self, input: &MouseInput, hits: &Hits, values: &mut UiValues, update: &mut NextUpdate) {
        let child = &mut self.child;
        values.item_scope(self.id, |v| child.mouse_input(input, hits, v, update));
    }

    fn mouse_move(&mut self, input: &UiMouseMove, hits: &Hits, values: &mut UiValues, update: &mut NextUpdate) {
        let child = &mut self.child;
        values.item_scope(self.id, |v| child.mouse_move(input, hits, v, update));
    }

    fn mouse_entered(&mut self, values: &mut UiValues, update: &mut NextUpdate) {
        let child = &mut self.child;
        values.item_scope(self.id, |v| child.mouse_entered(v, update));
    }

    fn mouse_left(&mut self, values: &mut UiValues, update: &mut NextUpdate) {
        let child = &mut self.child;
        values.item_scope(self.id, |v| child.mouse_left(v, update));
    }

    fn close_request(&mut self, values: &mut UiValues, update: &mut NextUpdate) {
        let child = &mut self.child;
        values.item_scope(self.id, |v| child.close_request(v, update));
    }
}

/// Defines a group of nested Uis as a single element.
pub fn ui_item(id: UiItemId, child: impl Ui) -> impl Ui {
    UiItem { child, id }
}
