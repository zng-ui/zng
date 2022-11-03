use zero_ui::core::widget;

#[widget($crate::test_widget)]
pub mod test_widget {
    use zero_ui::properties::{margin, states::is_pressed};

    properties! {
        margin as allowed_cfg = 0;
        margin as allowed_lints = 0;
        margin as disallowed_doc = 0;
        margin as disallowed_inline = 0;
        margin as disallowed_other1 = 0;
        margin as disallowed_other2 = 0;

        when *#is_pressed {
            #[cfg(not(any()))]
            allowed_cfg = 1;

            #[allow(unused_imports)]
            allowed_lints = {
                use std::vec;
                0
            };

            /// doc
            disallowed_doc = 1;

            #[inline]
            disallowed_inline = 1;

            #[foo(bar)]
            disallowed_other1 = 1;

            #[foo::bar(true)]
            disallowed_other2 = 1;
        }
    }
}

fn main() {}
