//! Property helper types.

use std::{
    any::{Any, TypeId},
    cell::RefCell,
    rc::Rc,
};

use crate::{
    handler::WidgetHandler,
    inspector::SourceLocation,
    var::{AnyVar, AnyVarValue, StateVar},
    BoxedUiNode, BoxedWidget, UiNode, UiNodeList, Widget, WidgetList,
};

/// Property priority in a widget.
///
/// See [the property doc](crate::property#priority) for more details.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Priority {
    /// [Context](crate::property#context) property.
    Context,
    /// [Event](crate::property#event) property.
    Event,
    /// [Layout](crate::property#layout) property.
    Layout,
    /// [Size](crate::property#size) property.
    Size,
    /// [Border](crate::property#border) property.
    Border,
    /// [Fill](crate::property#fill) property.
    Fill,
    /// [Child Context](crate::property#child-context) property.
    ChildContext,
    /// [Child Layout](crate::property#child-layout) property.
    ChildLayout,
}

/// Kind of property input.
#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum InputKind {
    /// Input is `impl IntoVar<T>`, build value is `BoxedVar<T>`.
    Var,
    /// Input and build value is `StateVar`.
    StateVar,
    /// Input is `impl IntoValue<T>`, build value is `T`.
    Value,
    /// Input is `impl UiNode`, `impl Widget`, `impl WidgetHandler<A>`, ``, build value is `InputTakeout`.
    Takeout,
}

/// Represents a value that cannot be cloned and can only be used in one instance.
pub struct InputTakeout {
    val: Rc<RefCell<Option<Box<dyn Any>>>>,
}
impl InputTakeout {
    fn new(val: Box<dyn Any>) -> Self {
        InputTakeout {
            val: Rc::new(RefCell::new(Some(val))),
        }
    }

    /// New from `impl UiNode` input.
    pub fn new_ui_node(node: impl UiNode) -> Self {
        Self::new(Box::new(node.boxed()))
    }

    /// New from `impl Widget` input.
    pub fn new_widget(wgt: impl Widget) -> Self {
        Self::new(Box::new(wgt.boxed_wgt()))
    }

    /// New from `impl WidgetHandler<A>` input.
    pub fn new_widget_handler<A>(handler: impl WidgetHandler<A>) -> Self
    where
        A: Clone + 'static,
    {
        todo!("AnyBoxed version")
    }

    /// New from `impl UiNodeList` input.
    pub fn new_ui_node_list(list: impl UiNodeList) -> Self {
        todo!("Boxed version")
    }

    /// New from `impl WidgetList` input.
    pub fn new_widget_list(list: impl WidgetList) -> Self {
        todo!("Boxed version")
    }

    /// If the args was not spend yet.
    pub fn is_available(&self) -> bool {
        self.val.borrow().is_some()
    }

    fn take<T: Any>(&self) -> T {
        *self
            .val
            .borrow_mut()
            .take()
            .expect("input takeout already used")
            .downcast::<T>()
            .expect("input takeout was of the requested type")
    }

    /// Takes the value for an `impl UiNode` input.
    pub fn take_ui_node(&self) -> BoxedUiNode {
        self.take()
    }

    /// Takes the value for an `impl UiNode` input.
    pub fn take_widget(&self) -> BoxedWidget {
        self.take()
    }

    // UiNodeList, WidgetHandler, etc. don't have a boxed version.
}

/// Property info.
#[derive(Debug)]
pub struct Info {
    /// Property insert order.
    pub priority: Priority,

    /// Unique type ID that identifies the property.
    pub type_id: fn() -> TypeId,
    /// Property original name.
    pub name: &'static str,

    /// Property declaration location.
    pub location: SourceLocation,

    /// Function that constructs the default args for the property.
    pub default: Option<fn() -> Box<dyn Args>>,

    /// Property inputs info, always at least one.
    pub inputs: Box<[Input]>,
}

/// Property input info.
#[derive(Debug)]
pub struct Input {
    /// Input name.
    pub name: &'static str,
    /// Input kind.
    pub kind: InputKind,
    /// Type as defined by kind.
    pub ty: fn() -> TypeId,
    /// Type name.
    pub ty_name: fn() -> &'static str,
}

/// Represents a property builder with input values.
pub trait Args {
    /// Property info.
    fn property(&self) -> Info;

    /// Gets a [`InputKind::Value`].
    fn value(&self, i: usize) -> &dyn AnyVarValue {
        panic_input(&self.property(), i, InputKind::Value)
    }

    /// Gets a [`InputKind::Var`].
    ///
    /// Is a `BoxedVar<T>`.
    fn var(&self, i: usize) -> &dyn AnyVar {
        panic_input(&self.property(), i, InputKind::Var)
    }

    /// Gets a [`InputKind::StateVar`].
    fn state_var(&self, i: usize) -> &StateVar {
        panic_input(&self.property(), i, InputKind::StateVar)
    }

    /// Gets a [`InputKind::Takeout`].
    fn takeout(&self, i: usize) -> &InputTakeout {
        panic_input(&self.property(), i, InputKind::Takeout)
    }

    /// Create a property instance with args clone or taken.
    fn instantiate(&self, child: BoxedUiNode) -> BoxedUiNode;
}

#[doc(hidden)]
pub fn panic_input(info: &Info, i: usize, kind: InputKind) -> ! {
    if i > info.inputs.len() {
        panic!("index out of bounds, the input len is {}, but the index is {i}", info.inputs.len())
    } else if info.inputs[i].kind != kind {
        panic!(
            "invalid input request `{:?}`, but `{}` is `{:?}`",
            kind, info.inputs[i].name, info.inputs[i].kind
        )
    } else {
        panic!("invalid input `{}`", info.inputs[i].name)
    }
}

#[cfg(test)]
mod tests {
    use std::any::type_name;

    use crate::{
        source_location,
        var::{var, BoxedVar, IntoVar, Var, VarValue},
    };

    use super::*;

    pub fn boo<T: VarValue>(child: impl UiNode, boo: impl IntoVar<bool>, too: impl IntoVar<Option<T>>) -> impl UiNode {
        let _ = (boo, too);
        tracing::error!("boo must be captured by the widget");
        child
    }

    #[doc(hidden)]
    #[allow(non_camel_case_types)]
    pub struct boo_Args<T: VarValue> {
        boo: BoxedVar<bool>,
        too: BoxedVar<Option<T>>,
    }
    impl<T: VarValue> boo_Args<T> {
        pub fn __new__(boo: impl IntoVar<bool>, too: impl IntoVar<Option<T>>) -> Box<dyn Args> {
            Box::new(Self {
                boo: Self::boo(boo),
                too: Self::too(too),
            })
        }

        pub fn __default__() -> Box<dyn Args> {
            Self::__new__(var(true), None)
        }

        pub fn boo(boo: impl IntoVar<bool>) -> BoxedVar<bool> {
            boo.into_var().boxed()
        }

        pub fn too(too: impl IntoVar<Option<T>>) -> BoxedVar<Option<T>> {
            too.into_var().boxed()
        }
    }
    impl<T: VarValue> Args for boo_Args<T> {
        fn property(&self) -> Info {
            Info {
                name: "boo",
                priority: Priority::Context,
                type_id: TypeId::of::<Self>,
                location: source_location!(),
                default: Some(Self::__default__),
                inputs: Box::new([
                    Input {
                        name: "boo",
                        kind: InputKind::Var,
                        ty: TypeId::of::<bool>,
                        ty_name: type_name::<bool>,
                    },
                    Input {
                        name: "too",
                        kind: InputKind::Var,
                        ty: TypeId::of::<T>,
                        ty_name: type_name::<T>,
                    },
                ]),
            }
        }

        fn var(&self, i: usize) -> &dyn AnyVar {
            match i {
                0 => &self.boo,
                1 => &self.too,
                n => panic_input(&self.property(), n, InputKind::Var),
            }
        }

        fn instantiate(&self, child: BoxedUiNode) -> BoxedUiNode {
            boo(child, self.boo.clone(), self.too.clone()).boxed()
        }
    }

    #[doc(hidden)]
    #[macro_export]
    macro_rules! boo_hash {
        (if default {
            $($tt:tt)*
        }) => {
            $($tt)*
        };
        (if !default {
            $($tt:tt)*
        }) => {
            // ignore
        };

        (if input(boo) {
            $($tt:tt)*
        }) => {
            $($tt)*
        };
        (if !input(boo) {
            $($tt:tt)*
        }) => {
            // ignore
        };

        (if input($other:ident) {
            $($tt:tt)*
        }) => {
            // ignore
        };
        (if !input($other:ident) {
            $($tt:tt)*
        }) => {
            $($tt:tt)*
        };

        (input_index(boo)) => {
            0
        };

        // sorted named input
        (<$Args:ty>::__new__($boo:ident, $too:ident)) => {
            $Args::__new__($foo, $too)
        };
    }
    #[doc(hidden)]
    pub use boo_hash;

    #[doc(hidden)]
    pub mod boo {
        pub use super::{boo_Args as Args, boo_hash as code_gen};
    }
}
