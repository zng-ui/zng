use zero_ui::core::widget;

#[widget($crate::test_widget)]
pub mod test_widget {
    inherit!(zero_ui::core::widget_base::base);

    properties! {
        pub foo() = 10;
    }
}

fn main() {}
