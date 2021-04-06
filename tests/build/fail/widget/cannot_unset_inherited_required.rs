use zero_ui::core::widget;

#[widget($crate::base_widget)]
pub mod base_widget {
    use zero_ui::properties::margin;

    properties! {
        margin = required!;
    }
}

#[widget($crate::test_widget)]
pub mod test_widget {
    inherit!(super::base_widget);

    properties! {
        margin = unset!;
    }
}

fn main() {}
