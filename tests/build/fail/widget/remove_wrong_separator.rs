use zero_ui::core::widget;

#[widget($crate::test_widget)]
pub mod test_widget {
    inherit!(zero_ui::widgets::button);

    properties! {
        remove { background_color, padding }
    }
}

fn main() {}
