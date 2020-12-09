//! Common widget properties.

mod align_;
mod attached;
pub mod background;
pub mod border;
pub mod button_theme;
pub mod capture_only;
mod clip_to_bounds_;
mod cursor_;
pub mod drag_move;
mod enabled_;
pub mod events;
pub mod filters;
pub mod focus;
pub mod foreground;
mod hit_testable_;
mod margin_;
mod position_;
pub mod size;
pub mod states;
pub mod text_theme;
mod title_;
pub mod transform;
mod visibility_;

pub use align_::*;
pub use attached::*;
pub use clip_to_bounds_::*;
pub use cursor_::*;
pub use enabled_::*;
pub use hit_testable_::*;
pub use margin_::*;
pub use position_::*;
pub use title_::*;
pub use visibility_::*;

/// Tests on the #[property(..)] code generator.
#[cfg(test)]
mod build_tests {
    use crate::core::property;
    use crate::core::var::*;
    use crate::prelude::*;

    #[property(context)]
    fn basic_context(child: impl UiNode, arg: impl IntoVar<u8>) -> impl UiNode {
        let _arg = arg;
        child
    }

    #[property(event)]
    fn basic_event(child: impl UiNode, arg: impl IntoVar<u8>) -> impl UiNode {
        let _arg = arg;
        child
    }

    #[property(outer)]
    fn basic_outer(child: impl UiNode, arg: impl IntoVar<u8>) -> impl UiNode {
        let _arg = arg;
        child
    }

    fn _basic_gen() {
        use basic_context::{args, ArgsNamed, ArgsUnwrap};
        let a = args(1);
        let _ar = a.arg();
        let _a = a.unwrap();
    }

    #[property(context)]
    fn phantom_gen<A: VarValue>(child: impl UiNode, a: impl IntoVar<A>, b: impl IntoVar<A>) -> impl UiNode {
        println!("{:?}", a.into_local().get_local());
        println!("{:?}", b.into_local().get_local());
        let _args = phantom_gen::args(TestInput, TestInput);
        child
    }

    #[derive(Debug, Clone)]
    struct TestInput;

    #[property(context)]
    fn no_phantom_required<A: VarValue>(child: impl UiNode, a: Vec<A>) -> impl UiNode {
        println!("{:?}", a);
        let _args = no_phantom_required::args(vec![TestInput, TestInput]);
        child
    }

    #[property(context)]
    fn not_arg_gen<C: UiNode>(child: C, arg: impl IntoVar<u8>) -> C {
        let _arg = arg;
        let _arg = not_arg_gen::args(1);
        child
    }

    #[property(context, allowed_in_when: false)]
    fn no_bounds<A>(child: impl UiNode, a: A) -> impl UiNode {
        let _a = no_bounds::args(a);
        child
    }

    #[property(context, allowed_in_when: false)]
    fn no_bounds_phantom<A, B: Into<A>>(child: impl UiNode, b: B) -> impl UiNode {
        let _b = no_bounds_phantom::args(b);
        child
    }

    #[property(context, allowed_in_when: false)]
    fn no_bounds_not_arg<A: UiNode, B>(_child: A, b: B) -> impl UiNode {
        let _b = no_bounds_not_arg::args(b);
        crate::widgets::text::text("")
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

    #[property(context)]
    fn generated_generic_name_collision<TC: UiNode>(child: TC, c: impl IntoVar<char>) -> TC {
        let _ = c;
        child
    }
}
