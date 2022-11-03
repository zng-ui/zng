use zero_ui::core::widget;

#[widget($crate::test_widget)]
pub mod test_widget {
    use zero_ui::properties::{margin, states::is_pressed};

    properties! {
        margin = 0;

        // invalid attributes
        #[foo(bar)]
        #[foo::bar(true)]
        // valid attributes
        #[cfg(not(any()))]
        #[allow(unused_imports)]
        /// doc
        when {
            use std::vec; // expect no warnings here
            *#is_pressed
        } {
            margin = 1;
        }

        #[inline] // invalid attribute
        when {
            use std::vec; // expect unused import warning here
            *#is_pressed
        } {
            margin = 2;
        }
    }
}

fn main() {}
