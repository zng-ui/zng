use zero_ui::core::widget;

#[widget($crate::test_widget)]
pub mod test_widget {
    use zero_ui::properties::margin;

    inherit!(zero_ui::core::widget_base::base);

    properties! {
        #![allow(inner_attribute)]
        #[!foo]
        /// valid
        margin = 10;
    }
}

fn main() {}
