//! Tests for `#[property(..)]` macro.

use crate::var::*;
use crate::{property, UiNode};

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
    let a = a.unwrap().into_local();
    let b = b.unwrap().into_local();
    assert_eq!(1, *a.get_local());
    assert_eq!(2, *b.get_local());
}

#[allow(dead_code)]
#[property(context)]
fn is_state(child: impl UiNode, state: StateVar) -> impl UiNode {
    let _ = state;
    child
}
#[test]
fn default_value() {
    use is_state::{code_gen, Args, ArgsImpl};
    let _ = ArgsImpl::default().unwrap();
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
