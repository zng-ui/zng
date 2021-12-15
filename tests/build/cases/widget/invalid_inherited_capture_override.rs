use zero_ui::core::{widget, widget_mixin};

#[widget($crate::base1_widget)]
pub mod base1_widget {
    use zero_ui::core::{var::IntoVar, NilUiNode, UiNode};

    properties! {
        foo(impl IntoVar<bool>);
        bar(impl IntoVar<bool>);
    }

    fn new_child(foo: impl IntoVar<bool>) -> NilUiNode {
        let _ = foo;
        NilUiNode
    }

    fn new(child: impl UiNode, bar: impl IntoVar<bool>) {
        let _ = (child, bar);
    }
}

#[widget_mixin($crate::base2_mixin)]
pub mod base2_mixin {
    use zero_ui::properties::margin;

    properties! {
        margin as foo = 10;
        margin as bar = 20;
    }
}

#[widget($crate::test_widget)]
pub mod test_widget {
    inherit!(super::base1_widget);
    inherit!(super::base2_mixin);
}

fn main() {}
