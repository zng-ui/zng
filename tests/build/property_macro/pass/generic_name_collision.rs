use zero_ui::core::{property, var::IntoVar, UiNode};

#[property(context)]
fn generated_generic_name_collision<TC: UiNode>(child: TC, c: impl IntoVar<char>) -> TC {
    let _ = c;
    child
}

fn main() {}
