use zero_ui::core::{property, var::*, UiNode};

#[property(context, allowed_in_when = false)]
fn unbounded<A>(child: impl UiNode, a: A) -> impl UiNode {
    let _a = unbounded::ArgsImpl::new(a);
    child
}

#[property(context, allowed_in_when = false)]
fn unbounded_phantom<A, B: Into<A>>(child: impl UiNode, b: B) -> impl UiNode {
    let _b = unbounded_phantom::ArgsImpl::new(b);
    child
}

#[property(context, allowed_in_when = false)]
fn generic_child<A: UiNode, B>(child: A, b: B) -> impl UiNode {
    let _b = generic_child::ArgsImpl::new(b);
    child
}

#[property(context)]
fn where_bounds<A, C, B>(child: C, a: impl IntoVar<A>, b: B) -> C
where
    C: UiNode,
    A: VarValue,
    B: IntoVar<A>,
{
    let _ = (a, b);
    child
}

fn main() {}
