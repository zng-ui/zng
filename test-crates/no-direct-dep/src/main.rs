//! Tests that use zero-ui without directly depending on it.

fn main() {}

#[test]
fn macro_rules_zero_ui_ref() {
    // a function that returns `true` in zero_ui is referenced here.
    let r = direct_dep::zero_ui_ref_call!();
    assert!(r);
}

#[test]
fn proc_macro_zero_ui_ref() {
    // zero_ui::widget_new! is referenced here.
    let wgt = direct_dep::test_widget! {};
    assert!(direct_dep::is_test_widget(wgt));
}
