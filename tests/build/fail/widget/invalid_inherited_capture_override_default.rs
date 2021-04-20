use zero_ui::core::{widget, widget_mixin};

#[widget_mixin($crate::base2_mixin)]
pub mod base2_mixin {
    use zero_ui::properties::margin;

    properties! {
        margin as id = 10;
    }
}

#[widget($crate::test_widget)]
pub mod test_widget {
    // base_1 implicit
    inherit!(super::base2_mixin);
}

fn main() {}
