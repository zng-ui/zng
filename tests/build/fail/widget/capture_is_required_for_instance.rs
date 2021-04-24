use zero_ui::core::widget;

#[widget($crate::test_widget)]
pub mod test_widget {
    use zero_ui::core::var::IntoVar;
    use zero_ui::core::NilUiNode;

    properties! {
        foo { impl IntoVar<bool> };
    }

    fn new_child(foo: impl IntoVar<bool>) -> NilUiNode {
        let _ = foo;
        NilUiNode
    }
}

fn main() {
    let _ = test_widget!();
}
