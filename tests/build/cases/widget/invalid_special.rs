use zero_ui::core::widget;

#[widget($crate::test_widget)]
pub mod test_widget {
    use zero_ui::properties::margin;

    inherit!(zero_ui::core::widget_base::base);

    properties! {
        margin = foo!;
    }
}

fn main() {}
