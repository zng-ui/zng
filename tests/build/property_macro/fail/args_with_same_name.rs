use zero_ui::core::{property, var::IntoVar};

#[property(capture_only)]
pub fn args_with_same_name(name: impl IntoVar<bool>, name: impl IntoVar<bool>) -> ! { }

fn main() { }