//! Common widget properties.

mod background_;
mod border_;
mod context_var;
mod cursor_;
mod focus;
mod hit_testable_;
mod id_;
mod is_state_;
mod layout;
mod on_event_;
mod text;
mod title_;

pub use background_::*;
pub use border_::*;
pub use context_var::*;
pub use cursor_::*;
pub use focus::*;
pub use hit_testable_::*;
pub use id_::*;
pub use is_state_::*;
pub use layout::*;
pub use on_event_::*;
pub use text::*;
pub use title_::*;

/// Tests on the #[property(..)] code generator.
#[cfg(test)]
mod build_tests {
    use crate::core::var::*;
    use crate::prelude::*;
    use crate::property;

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
        use basic_context::{args, Args};
        let a = args(1);
        let _ar = a.arg();
        let (_a,) = a.pop();
    }

    #[property(context)]
    fn phantom_gen<A: VarValue>(child: impl UiNode, a: impl IntoVar<A>, b: impl IntoVar<A>) -> impl UiNode {
        println!("{:?}", a.into_local().get_local());
        println!("{:?}", b.into_local().get_local());
        let _args = phantom_gen::args(0, 1);
        child
    }

    #[property(context)]
    fn no_phantom_required<A: VarValue>(child: impl UiNode, a: Vec<A>) -> impl UiNode {
        println!("{:?}", a);
        let _args = no_phantom_required::args(vec![0, 1]);
        child
    }

    #[property(context)]
    fn not_arg_gen<C: UiNode>(child: C, arg: impl IntoVar<u8>) -> C {
        let _arg = arg;
        let _arg = not_arg_gen::args(1);
        child
    }

    #[property(context)]
    fn no_bounds<A>(child: impl UiNode, a: A) -> impl UiNode {
        let _a = no_bounds::args(a);
        child
    }

    #[property(context)]
    fn no_bounds_phantom<A, B: Into<A>>(child: impl UiNode, b: B) -> impl UiNode {
        let _b = no_bounds_phantom::args(b);
        child
    }

    #[property(context)]
    fn no_bounds_not_arg<A, B>(_child: A, b: B) -> impl UiNode {
        let _b = no_bounds_not_arg::args(b);
        crate::widgets::text("")
    }
}
