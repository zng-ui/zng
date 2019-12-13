use crate::core::*;

#[doc(hidden)]
pub struct Cursor<T: Ui, C: Value<CursorIcon>> {
    child: T,
    cursor: C,
}

#[impl_ui_crate(child)]
impl<T: Ui, C: Value<CursorIcon>> Ui for Cursor<T, C> {
    fn value_changed(&mut self, values: &mut UiValues, update: &mut NextUpdate) {
        if self.cursor.touched() {
            update.render_frame();
        }

        self.child.value_changed(values, update);
    }

    fn render(&self, f: &mut NextFrame) {
        f.push_cursor(*self.cursor, &self.child)
    }
}

/// Property like Ui that sets the cursor.
///
/// This function can be used as a property in ui! macros.
///
/// # Arguments
/// * `child`: The cursor target.
/// * `cursor`: The cursor to use for `child`, can be a direct [value](CursorIcon) or a [variable](zero_ui::core::Var).
///
/// # Example
/// ```
/// # mod example { use zero_ui::primitive::text; fn doc() {
/// ui! {
///     cursor: CursorIcon::Hand;
///     => text("Mouse over this text shows the hand cursor")
/// }
/// # }}
/// ```
#[ui_property]
pub fn cursor(child: impl Ui, cursor: impl IntoValue<CursorIcon>) -> impl Ui {
    Cursor {
        child,
        cursor: cursor.into_value(),
    }
}
