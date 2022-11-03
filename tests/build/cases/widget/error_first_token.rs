use zero_ui::core::widget;

#[widget($crate::test)]
pub mod test {
    use zero_ui::properties::states::is_pressed;

    properties! {
        =
    }

    properties! {
        when *#is_pressed {
            =
        }
    }
}

fn main() {}
