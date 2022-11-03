use zero_ui::core::{widget, widget_mixin};

#[widget($crate::base_wgt)]
pub mod base_wgt {
    inherit!(zero_ui::core::widget_base::base);
}

#[widget_mixin($crate::base_mixin)]
pub mod base_mixin {}

#[widget_mixin($crate::test_mixin)]
pub mod test_mixin {
    inherit!(super::base_wgt); // error
    inherit!(super::base_mixin); // valid
}

fn main() {}
