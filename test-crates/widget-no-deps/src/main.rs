//! Test if widget_new! is called when a crate does not depend on zero-ui directly.

use custom_widget::test_widget;

fn main() {
    let _wgt = test_widget! {};
}
