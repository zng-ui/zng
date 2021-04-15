use zero_ui::core::widget;

#[widget($crate::base_widget)]
pub mod base_widget {
    use zero_ui::core::{var::IntoVar, NilUiNode};

    properties! {
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

fn main() {
    let _base = base_widget! {
        // expected missing required property error
    };

    let _test = test_widget! {
        // expected no error
    };

    let _test = test_widget! {
        foo = 42; // expected undeclared crate or module error
    };
}
