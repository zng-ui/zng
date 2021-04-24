use zero_ui::core::{property, var, widget_mixin};

#[widget_mixin($crate::test1_mixin)]
pub mod test1_mixin {
    use zero_ui::core::var::IntoVar;

    properties! {
        foo { impl IntoVar<bool> }
    }
}

#[widget_mixin($crate::test2_mixin)]
pub mod test2_mixin {
    properties! {
        #[allowed_in_when = false]
        foo { bool }
    }
}

#[property(capture_only)]
pub fn bar(bar: impl var::IntoVar<bool>) -> ! {}
#[widget_mixin($crate::test3_mixin)]
pub mod test3_mixin {
    properties! {
        super::bar
    }
}

#[widget_mixin($crate::test_ok_mixin)]
pub mod test_ok_mixin {
    properties! {
        // expect no error here.
        zero_ui::properties::margin = 10;
    }
}

fn main() {}
