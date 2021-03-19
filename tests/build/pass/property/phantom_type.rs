use zero_ui::core::{property, var::*, UiNode};

#[property(context)]
fn phantom_generated<A: VarValue>(child: impl UiNode, a: impl IntoVar<A>, b: impl IntoVar<A>) -> impl UiNode {
    let _args = phantom_generated::ArgsImpl {
        a,
        b,
        _phantom: std::marker::PhantomData,
    };
    child
}

#[property(context, allowed_in_when = false)]
fn no_phantom_generated(child: impl UiNode, a: Vec<u8>) -> impl UiNode {
    let _args = no_phantom_generated::ArgsImpl { a };
    child
}

fn main() {}
