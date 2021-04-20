use zero_ui::core::widget;

#[widget($crate::test_widget)]
pub mod test_widget {
    // cause a "first" stage error.
    inherit!(not_a_thing);
}

fn main() {
    // expect an error that indicates that `test_widget` is not compiling.
    let _ = test_widget!();
}
