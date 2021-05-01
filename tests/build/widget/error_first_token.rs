use zero_ui::core::widget;

#[widget($crate::test)]
pub mod test {
    use zero_ui::properties::states::is_pressed;

    properties! {
        =
    }

    properties! {
        when self.is_pressed {
            =
        }
    }

    properties! {
        child {
            =
        }
    }

    properties! {
        remove {
            =
        }
    }
}

fn main() {}
