//! Tests that you can use `zero-ui` generated widget macros without directly depending on `zero-ui`.

fn main() {}

#[test]
fn macro_rules_zero_ui_ref() {
    // A function that returns `true` in zero_ui is referenced here.
    //
    // This is our sanity check, it only uses `macro_rules!` and `$crate` so if
    // our testing is correct this should always pass.

    let r = direct_dep::zero_ui_ref_call!();
    assert!(r);
}

#[test]
fn proc_macro_zero_ui_ref() {
    // zero_ui::widget_new! is referenced here.
    let wgt = direct_dep::test_widget! {};
    assert!(direct_dep::is_test_widget(wgt));
}
