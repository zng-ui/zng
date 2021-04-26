/*
    Property Default Value
*/

// New #a
// 
// # Pros
//
// * Its the syntax proposed in a pre-RFC for default parameters (https://internals.rust-lang.org/t/pre-rfc-named-arguments/3831)
// 
// * Cons
//
// * Same parsing problem we are trying to avoid in in-place capture_only parsing.
// * rust-analyzer does not like this syntax, this sample is inside a macro to avoid an error and this file is not even linked.
macro_rules! _t { () => {

#[property(context)]
pub fn foo(child: impl UiNode, a: impl IntoVar<u32> = 10, b: impl IntoVar<u32> = 20) -> impl UiNode {
    child
}

}}

// New #b
#[property(context, default {
    b: 10,
    a: 10,
})]
pub fn foo(child: impl UiNode, a: impl IntoVar<u32>, b: impl IntoVar<u32>) -> impl UiNode {
    child
}
#[property(context, default(10, 20))]
pub fn bar(child: impl UiNode, a: impl IntoVar<u32>, b: impl IntoVar<u32>) -> impl UiNode {
    child
}

// New #c
#[property(context, default = {
    b: 10,
    a: 10,
})]
pub fn foo(child: impl UiNode, a: impl IntoVar<u32>, b: impl IntoVar<u32>) -> impl UiNode {
    child
}
#[property(context, default = 10, 20)]
pub fn bar(child: impl UiNode, a: impl IntoVar<u32>, b: impl IntoVar<u32>) -> impl UiNode {
    child
}

// New #e
#[property(context, default(b: 10, b: 20))]
pub fn foo(child: impl UiNode, a: impl IntoVar<u32>, b: impl IntoVar<u32>) -> impl UiNode {
    child
}
#[property(context, default(10, 20))]
pub fn bar(child: impl UiNode, a: impl IntoVar<u32>, b: impl IntoVar<u32>) -> impl UiNode {
    child
}
