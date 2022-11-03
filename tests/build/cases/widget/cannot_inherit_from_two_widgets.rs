use zero_ui::core::widget;

#[widget($crate::base1_wgt)]
pub mod base1_wgt {
    inherit!(zero_ui::core::widget_base::base);
}

#[widget($crate::base2_wgt)]
pub mod base2_wgt {
    inherit!(zero_ui::core::widget_base::base);
}

#[widget($crate::test_wgt)]
pub mod test_wgt {
    inherit!(super::base1_wgt); // ok
    inherit!(super::base2_wgt); // error
}

fn main() {}
