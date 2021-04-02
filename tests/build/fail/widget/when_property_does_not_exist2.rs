use zero_ui::core::widget;

#[widget($crate::test_widget)]
pub mod test_widget {
    properties! {
        when self.is_pressed { }
    }
}

fn main() {}
