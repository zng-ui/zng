use zero_ui::core::{property, UiNode};

struct PrivateFoo;

#[property(context, allowed_in_when = false)]
pub fn bar(child: impl UiNode, value: PrivateFoo) -> impl UiNode {
    let _ = value;
    child
}

fn main() {}

// NOTE: this one is pretty bad, first Rust highlights the full function signature instead of just the private type,
// second the types get expanded into the context of a struct member type, generics, trait method signatures and associated
// types, all witch also generate the error highlighting more then the type, in the end we are getting multiple errors
// (and warnings) at line 5, the warnings cannot be suppressed either.
