use zero_ui::core::widget;

#[widget($crate::test_widget)]
pub mod test_widget {
    use zero_ui::properties::margin;

    properties! {
        #[cfg(not(any()))]
        margin as allowed_cfg;

        #[allow(unused_imports)]
        margin as allowed_lints = {
            use std::vec;
            0
        };

        /// doc
        margin as allowed_doc;

        #[inline]
        margin as disallowed_inline;

        #[foo(bar)]
        margin as disallowed_other1;

        #[foo::bar(true)]
        margin as disallowed_other2;
    }
}

fn main() {
    let _ = test_widget! {
        allowed_cfg = 0;
        allowed_lints = 0;
        allowed_doc = 0;
        disallowed_inline = 0;
        disallowed_other1 = 0;
        disallowed_other2 = 0;
    };
}
