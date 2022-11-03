use zero_ui::core::widget;

#[widget($crate::test_widget)]
pub mod test_widget {
    inherit!(zero_ui::core::widget_base::base);

    properties! {
        {}
    }
}

fn main() {}
