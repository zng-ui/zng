use zero_ui::core::widget;

#[widget($crate::base1_wgt)]
pub mod base1_wgt {}

#[widget($crate::base2_wgt)]
pub mod base2_wgt {}

#[widget($crate::test_wgt)]
pub mod test_wgt {
    inherit!(super::base1_wgt); // ok
    inherit!(super::base2_wgt); // error
}

fn main() {}
