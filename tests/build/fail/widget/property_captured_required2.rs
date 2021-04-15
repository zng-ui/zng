use zero_ui::core::widget;

#[widget($crate::base_widget)]
pub mod base_widget {
    use zero_ui::core::{var::IntoVar, NilUiNode};

    properties! {
        #[required]
        foo: impl IntoVar<u32>;
    }

    fn new_child(foo: impl IntoVar<u32>) -> NilUiNode {
        let _ = foo;
        NilUiNode
    }
}

#[widget($crate::test_widget)]
pub mod test_widget {
    use zero_ui::core::NilUiNode;

    inherit!(super::base_widget);

    fn new_child() -> NilUiNode {
        NilUiNode
    }
}

fn main() {}
