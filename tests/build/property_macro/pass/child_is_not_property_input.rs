use zero_ui::core::{property, UiNode, var::IntoVar};

#[property(context)]
fn test_property<C: UiNode>(child: C, arg: impl IntoVar<u8>) -> C {
    let _arg = test_property::ArgsImpl::new(arg);
    child
}

fn main() { }