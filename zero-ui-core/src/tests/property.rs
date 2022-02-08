//! Tests for `#[property(..)]` macro.
//!
//! Note: Compile error tests are in the integration tests folder: `tests/build/property`

use crate::context::TestWidgetContext;
use crate::var::*;
use crate::{property, NilUiNode, UiNode};

#[allow(dead_code)]
#[property(context)]
fn basic_context(child: impl UiNode, arg: impl IntoVar<u8>) -> impl UiNode {
    let _arg = arg;
    child
}
#[test]
fn basic_gen() {
    use basic_context::{code_gen, Args, ArgsImpl};
    let a = ArgsImpl::new(1);
    let b = code_gen! { named_new basic_context, __ArgsImpl { arg: 2 } };
    let test = TestWidgetContext::new();
    assert_eq!(1, a.unwrap().into_var().into_value(&test.vars));
    assert_eq!(2, b.unwrap().into_var().into_value(&test.vars));
}

#[allow(dead_code)]
#[property(context)]
fn is_state(child: impl UiNode, state: StateVar) -> impl UiNode {
    let _ = state;
    child
}
#[test]
fn default_value() {
    use is_state::{code_gen, Args};
    let _ = is_state::default_args().unwrap();
    let is_default;
    let is_not_default = false;
    code_gen! {
        if default=> {
            is_default = true;
        }
    };
    code_gen! {
        if !default=> {
            is_not_default = true;
        }
    };
    assert!(is_default);
    assert!(!is_not_default);
}

#[test]
fn not_default_value() {
    use basic_context::code_gen;
    let is_default = false;
    let is_not_default;
    code_gen! {
        if default=> {
            is_default = true;
        }
    };
    code_gen! {
        if !default=> {
            is_not_default = true;
        }
    };
    assert!(!is_default);
    assert!(is_not_default);
}

mod all_priorities {
    use crate::{property, var::IntoVar, UiNode};

    #[property(context)]
    pub fn context_property(child: impl UiNode, input: impl IntoVar<bool>) -> impl UiNode {
        let _ = input;
        child
    }

    #[property(event)]
    pub fn on_event_property(child: impl UiNode, input: impl IntoVar<bool>) -> impl UiNode {
        let _ = input;
        child
    }

    #[property(layout)]
    pub fn outer_property(child: impl UiNode, input: impl IntoVar<bool>) -> impl UiNode {
        let _ = input;
        child
    }

    #[property(size)]
    pub fn size_property(child: impl UiNode, input: impl IntoVar<bool>) -> impl UiNode {
        let _ = input;
        child
    }

    #[property(border)]
    pub fn border_property(child: impl UiNode, input: impl IntoVar<bool>) -> impl UiNode {
        let _ = input;
        child
    }

    #[property(fill)]
    pub fn fill_property(child: impl UiNode, input: impl IntoVar<bool>) -> impl UiNode {
        let _ = input;
        child
    }

    #[property(capture_only)]
    pub fn capture_only_property(input: impl IntoVar<bool>) -> ! {}
}
#[test]
fn all_priorities() {
    use all_priorities::*;

    let _ = context_property(NilUiNode, true);
    let args = context_property::ArgsImpl::new(true);
    let _ = context_property::set(args, NilUiNode);

    let _ = on_event_property(NilUiNode, true);
    let args = on_event_property::ArgsImpl::new(true);
    let _ = on_event_property::set(args, NilUiNode);

    let _ = outer_property(NilUiNode, true);
    let args = outer_property::ArgsImpl::new(true);
    let _ = outer_property::set(args, NilUiNode);

    let _ = size_property(NilUiNode, true);
    let args = size_property::ArgsImpl::new(true);
    let _ = size_property::set(args, NilUiNode);

    let _ = border_property(NilUiNode, true);
    let args = border_property::ArgsImpl::new(true);
    let _ = border_property::set(args, NilUiNode);

    let _ = fill_property(NilUiNode, true);
    let args = fill_property::ArgsImpl::new(true);
    let _ = fill_property::set(args, NilUiNode);
}

mod attr_args {
    use crate::{property, var::IntoVar, UiNode};

    #[property(context)]
    pub fn trailing_comma_1(child: impl UiNode, input: impl IntoVar<bool>) -> impl UiNode {
        let _ = input;
        child
    }

    #[property(context, allowed_in_when = true)]
    pub fn allowed_in_when(child: impl UiNode, input: impl IntoVar<bool>) -> impl UiNode {
        let _ = input;
        child
    }

    #[property(context, allowed_in_when = false)]
    pub fn not_allowed_in_when(child: impl UiNode, input: bool) -> impl UiNode {
        let _ = input;
        child
    }

    #[property(context, allowed_in_when = false)]
    pub fn trailing_comma_2(child: impl UiNode, input: bool) -> impl UiNode {
        let _ = input;
        child
    }
}
#[test]
fn attr_args() {
    use attr_args::*;

    let _ = trailing_comma_1(NilUiNode, true);
    let args = trailing_comma_1::ArgsImpl::new(true);
    let _ = trailing_comma_1::set(args, NilUiNode);

    let _ = allowed_in_when(NilUiNode, true);
    let args = allowed_in_when::ArgsImpl::new(true);
    let _ = allowed_in_when::set(args, NilUiNode);

    let _ = not_allowed_in_when(NilUiNode, true);
    let args = not_allowed_in_when::ArgsImpl::new(true);
    let _ = not_allowed_in_when::set(args, NilUiNode);

    let _ = trailing_comma_2(NilUiNode, true);
    let args = trailing_comma_2::ArgsImpl::new(true);
    let _ = trailing_comma_2::set(args, NilUiNode);
}

mod generic_name_collision {
    use crate::{property, var::IntoVar, UiNode};

    #[property(context)]
    pub fn test<TC: UiNode>(child: TC, c: impl IntoVar<char>) -> TC {
        let _ = c;
        child
    }
}
#[test]
fn generic_name_collision() {
    let _ = generic_name_collision::test(NilUiNode, 'a');
    let args = generic_name_collision::test::ArgsImpl::new('a');
    let _ = generic_name_collision::test::set(args, NilUiNode);
}

mod generics {
    use crate::{property, var::*, UiNode};

    #[property(context, allowed_in_when = false)]
    pub fn unbounded<A>(child: impl UiNode, a: A) -> impl UiNode {
        let _a = unbounded::ArgsImpl::new(a);
        child
    }

    #[property(context, allowed_in_when = false)]
    pub fn unbounded_phantom<A, B: Into<A>, C: UiNode>(child: C, b: B) -> impl UiNode {
        let _b = unbounded_phantom::ArgsImpl::new(b);
        child
    }

    #[property(context, allowed_in_when = false)]
    pub fn generic_child<A: UiNode, B>(child: A, b: B) -> impl UiNode {
        let _b = generic_child::ArgsImpl::new(b);
        child
    }

    #[property(context)]
    pub fn where_bounds<A, C, B>(child: C, a: impl IntoVar<A>, b: B) -> C
    where
        C: UiNode,
        A: VarValue,
        B: IntoVar<A>,
    {
        let _ = (a, b);
        child
    }
}
#[test]
fn generics() {
    use generics::*;

    let _ = unbounded(NilUiNode, 'a');
    let args = unbounded::ArgsImpl::new('a');
    let _ = unbounded::set(args, NilUiNode);

    fn value() -> impl Into<char> {
        'a'
    }
    let _ = unbounded_phantom(NilUiNode, value());
    let args = unbounded_phantom::ArgsImpl::new(value());
    let _ = unbounded_phantom::set(args, NilUiNode);

    let _ = generic_child(NilUiNode, 'a');
    let args = generic_child::ArgsImpl::new('a');
    let _ = generic_child::set(args, NilUiNode);

    let _ = where_bounds(NilUiNode, 'a', 'b');
    let args = where_bounds::ArgsImpl::new('a', 'b');
    let _ = where_bounds::set(args, NilUiNode);
}

mod not_into_var_inputs {
    use crate::{property, var::Var, UiNode};

    #[property(context)]
    pub fn not_into_var_input(child: impl UiNode, input: impl Var<&'static str>) -> impl UiNode {
        let _ = input;
        child
    }

    #[property(context, allowed_in_when = false)]
    pub fn not_var_input(child: impl UiNode, input: &'static str) -> impl UiNode {
        let _ = input;
        child
    }
}
#[test]
fn not_into_var_inputs() {
    use not_into_var_inputs::*;

    let _ = not_into_var_input(NilUiNode, var("a"));
    let args = not_into_var_input::ArgsImpl::new(var("a"));
    let _ = not_into_var_input::set(args, NilUiNode);

    let _ = not_var_input(NilUiNode, "a");
    let args = not_var_input::ArgsImpl::new("a");
    let _ = not_var_input::set(args, NilUiNode);
}

mod phantom_type {
    use crate::{property, var::*, UiNode};

    #[property(context)]
    pub fn phantom_generated<A: VarValue>(child: impl UiNode, a: impl IntoVar<A>, b: impl IntoVar<A>) -> impl UiNode {
        let _args = phantom_generated::ArgsImpl {
            a,
            b,
            _phantom: std::marker::PhantomData,
        };
        child
    }

    #[property(context, allowed_in_when = false)]
    pub fn no_phantom_generated(child: impl UiNode, a: Vec<u8>) -> impl UiNode {
        let _args = no_phantom_generated::ArgsImpl { a };
        child
    }
}
#[test]
fn phantom_type() {
    use phantom_type::*;

    let _ = phantom_generated(NilUiNode, 'a', 'a');
    let args = phantom_generated::ArgsImpl::new('a', 'a');
    let _ = phantom_generated::set(args, NilUiNode);

    let _ = no_phantom_generated(NilUiNode, vec![]);
    let args = no_phantom_generated::ArgsImpl::new(vec![]);
    let _ = no_phantom_generated::set(args, NilUiNode);
}

mod sub_pattern_input {
    use crate::{property, UiNode};

    // This will be how we support destructuring in the input while getting
    // a name for the property named assign.
    //
    // For now only @ _ is stable.
    #[allow(clippy::redundant_pattern)]
    #[property(context, allowed_in_when = false)]
    pub fn sub_pattern_all(child: impl UiNode, input @ _: bool) -> impl UiNode {
        let _ = input;
        child
    }
}
#[test]
fn sub_pattern_input() {
    use sub_pattern_input::*;

    let _ = sub_pattern_all(NilUiNode, true);
    let args = sub_pattern_all::ArgsImpl::new(true);
    let _ = sub_pattern_all::set(args, NilUiNode);
}

mod defaults {
    use crate::{property, UiNode};

    #[property(context, allowed_in_when = false, default(b: 2567, a: true))]
    pub fn named(child: impl UiNode, a: bool, b: u32) -> impl UiNode {
        let _ = (a, b);
        child
    }

    #[property(context, allowed_in_when = false, default(true, 2567))]
    pub fn unnamed(child: impl UiNode, a: bool, b: u32) -> impl UiNode {
        let _ = (a, b);
        child
    }
}

#[test]
fn named_default() {
    use defaults::*;

    let _ = named(NilUiNode, false, 0);
    let args = named::default_args();

    assert_eq!(&true, args.__a());
    assert_eq!(&2567, args.__b());
}

#[test]
fn unnamed_default() {
    use defaults::*;

    let _ = unnamed(NilUiNode, false, 0);
    let args = unnamed::default_args();

    assert_eq!(&true, args.__a());
    assert_eq!(&2567, args.__b());
}

mod macro_rules_generated {
    use crate::{property, var::IntoVar, UiNode};

    macro_rules! test {
        ($name:ident) => {
            #[property(context)]
            pub fn $name(child: impl UiNode, value: impl IntoVar<$crate::units::SideOffsets>) -> impl UiNode {
                let _ = value;
                child
            }
        };
    }

    test! {
        bar
    }
}

#[test]
fn macro_rules_generated() {
    use macro_rules_generated::*;
    let _ = bar(NilUiNode, 0);
    let _ = bar::ArgsImpl::new(0);
}
