use zero_ui::core::{widget, widget_mixin};

#[widget_mixin($crate::test_mixin)]
pub mod test_mixin {
    use zero_ui::properties::margin;

    properties! {
        margin = 0;
    }
}

#[widget($crate::test_widget)]
pub mod test_widget {
    // expected unused here
    use zero_ui::properties::margin;

    inherit!(crate::test_mixin);

    properties! {
        margin = 0;
    }
}

fn main() {
    let _ = test_widget! {
        margin = 0;
    };
    compile_error!("expected warning @ line 15")
}
