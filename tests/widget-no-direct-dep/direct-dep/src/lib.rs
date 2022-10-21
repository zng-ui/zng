use zero_ui_renamed::core::{widget, Widget};

#[widget($crate::test_widget)]
pub mod test_widget {}

/// Test util.
///
/// In the `no-direct-dep` crate we don't have direct access to the `zero-ui` types.
/// So the type-assert function is declared here.
pub fn is_test_widget(_: impl UiNode) -> bool {
    true
}

pub use zero_ui_renamed::crate_reference_call as zero_ui_ref_call;

#[test]
pub fn macros_ok_in_direct_dep() {
    // Sanity check, we want the macros to be working in a crate with direct reference to `zero-ui` before
    // the actual test in `no-direct-dep`.
    let wgt = test_widget! {};
    assert!(is_test_widget(wgt));
}
