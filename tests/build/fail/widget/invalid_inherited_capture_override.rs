use zero_ui::core::widget;

#[widget($crate::base1_widget)]
pub mod base1_widget {
    use zero_ui::core::{var::IntoVar, NilUiNode};

    properties! {
        margin: impl IntoVar<bool>;
    }

    fn new_child(margin: impl IntoVar<bool>) -> NilUiNode {
        let _ = margin;
        NilUiNode
    }
}

#[widget($crate::base2_widget)]
pub mod base2_widget {
    use zero_ui::properties::margin;

    properties! {
        margin = 10;
    }
}

#[widget($crate::test_widget)]
pub mod test_widget {
    inherit!(super::base1_widget);
    inherit!(super::base2_widget);
}

fn main() {
    let _ = test_widget!();
    let _ = test_widget! {
        margin = true;
    };
    let _ = test_widget! {
        margin = 20;
    };
}
