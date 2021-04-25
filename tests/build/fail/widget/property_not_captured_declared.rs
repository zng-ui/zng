use zero_ui::core::widget;

#[widget($crate::test_widget)]
pub mod test_widget {
    use zero_ui::core::var::IntoVar;

    properties! {
        foo(impl IntoVar<bool>);
    }
}

fn main() {}
