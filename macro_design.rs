/*
    In-place capture_only Declaration
*/ 
properties! {
    /// Unnamed now
    foo: impl IntoVar<u32> = 10;
    fuz: impl IntoVar<u32>, impl IntoVar<32> = 10, 10;

    /// Named now
    bar: {
        a: u32,
        b: u32
    } = 10, 20;

    /// New unnamed #a
    foo: (impl IntoVar<u32>) = 10;
    fuz: (impl IntoVar<u32>, impl IntoVar<u32>) = 10, 20;
    // named stays the same.

    /// New unnamed #b
    foo(impl IntoVar<u32>) = 10;
    fuz(impl IntoVar<u32>, impl IntoVar<u32>) = 10, 20;
    /// New named #b
    bar {
        a: u32,
        b: u32
    } = 10, 20;

    /// New named #c
    fuz(a: impl IntoVar<u32>, b: impl IntoVar<u32>) = 10, 20;

    /// New unnamed #d
    foo: { impl IntoVar<u32> } = 10;
    foo: { impl IntoVar<u32>, impl IntoVar<u32> } = 10, 20;

    /// New Radical #a
    fn new_child(
        /// Capture Property
        fuz: (impl IntoVar<u32>, impl IntoVar<32>)
    ) -> imp UiNode {
        !
    }
    fn new_child(
        /// Capture Property
        fuz: (a: impl IntoVar<u32>, b: impl IntoVar<32>)
    ) -> imp UiNode {
        !
    }
}

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

/*
    Property Order Analysis
*/

macro_rules! __ {
    () => {
        // the widget:
        blank! {
            context1 = 'a';
            context2 = 'b';

            outer1 = 0;
            outer2 = 1;

            inner1 = RED;
            inner2 = GREEN;
        }

        // expands to:
        let node = new_child();

        let node = inner1::set(node, RED);
        let node = inner2::set(node, GREEN);

        let node = outer1::set(node, 0);
        let node = outer2::set(node, 1);

        let node = context1::set(node, 'a');
        let node = context2::set(node, 'b');

        // so the widget node is:
        context2 {
            sets: 'b'

            context1 {
                sets: 'a'

                outer2 {
                    sets: 1

                    outer1 {
                        sets: 0

                        inner2 {
                            sets: GREEN

                            inner1 {
                                sets: RED

                                new_child()
                            }
                        }
                    }
                }
            }
        }

        // so the final values are:
        context = 'a'
        outer = 1
        inner = RED

        // but we expected, right?
        context = 'b'
        outer = 1
        inner = GREEN

        //   note that we already have to invert for the priority to work.
    };
}