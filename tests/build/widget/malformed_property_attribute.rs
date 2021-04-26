use zero_ui::core::widget;

#[widget($crate::test_widget)]
pub mod test_widget {
    use zero_ui::properties::margin;

    properties! {
        #![allow(inner_attribute)]
        #[!foo]
        /// valid
        margin = 10;
    }
}

fn main() {}
