use zero_ui::core::widget;

#[widget($crate::test_base)]
pub mod test_base {
    use zero_ui::core::var::IntoVar;
    use zero_ui::core::NilUiNode;

    properties! {
        foo: impl IntoVar<bool>;
    }

    fn new_child(foo: impl IntoVar<bool>) -> NilUiNode {
        let _ = foo;
        NilUiNode
    }
}
#[widget($crate::test_widget)]
pub mod test_widget {
    inherit!(super::test_base);
}

fn main() {
    let _ = test_widget!();
}
