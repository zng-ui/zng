use zero_ui::core::{property, widget};

#[property(capture_only)]
pub fn foo(foo: impl zero_ui::core::var::IntoVar<bool>) -> ! {}

#[widget($crate::test_widget)]
pub mod test_widget {
    properties! {
        super::foo;
    }
}

fn main() {}
