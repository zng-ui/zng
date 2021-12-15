use zero_ui::core::widget;

#[widget($crate::test_widget)]
pub mod test_widget {
    properties! {
        // cause a "final stage" error.
        remove { id }
    }
}

fn main() {
    // expect an error that indicates that `test_widget` is not compiling.
    let _ = test_widget!();
}
