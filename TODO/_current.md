* Refactor animate sleep tracking, to allow refactoring AnimationArgs to be an Rc, to allow real `Var::modify` animation.
    - Using clone for now, after merge refactor this.

* Review nodes that call `(de)init(ctx)`, are they causing a widget handle collection to grow uncontrolledly?

* Implement all `todo!` code.

# Build Time / LLVM Lines

* We need to simplify code generation, less items more instances.
    - The biggest gain so-far was refactoring events and context vars to static instances of a single type.
    - Can we do the same for properties?

* List all property and widget requirements now that it seems covered all needs.

* Don't be afraid to use some dyn runtime processing to form a widget, we have not seen a 
  performance impact of the fully dynamic styled widgets.

## Property Requirements

* Control its own priority "position" in a widget.
* Have default values.
* Be a simple function that can be used directly in other properties.
* Accept generic inputs `impl IntoVar<T>` and `impl IntoValue<T>`.
    - We can force only those two inputs types, that makes all properties "allowed_in_when".
    - We don't use any other types (outside of testing).
    - What we really want is a build time assert that a property input does not change (`IntoValue<WidgetId>`).
* Accept explicit generic params `T`.
    - The toggle `value<u8> = 10u8` is a firm requirement, can't avoid this.
* Have extra inspector info.

## Widget Requirements

* Insert custom nodes at each priority level.
* Collect custom nodes in a dynamic list for styling.
    - We could make all widgets "dynamic/stylable" by default.
    - Then widget creation can be some sort of `WidgetBuilder` instance creation.
* Finetune priority of a property within its own priority group.
* Declare capture-only "properties", that are redirected to the custom nodes.

## New Property

Limitations:

* All previous limitations, (no async, unsafe, const).
* Input bindings can only be `ident: impl T`.
* Input types can only be `impl UiNode`, `impl IntoVar<T>`, `impl IntoValue<T>`, we validate and replace with the full path.
* Return type also can only be `impl UiNode`.
* Explicit generic types can only be `ident: VarValue`.
* No attributes allowed in input/output or generic params.
* No retention of actual types to the extreme we have today, values are actualized (and vars boxed) early.
    - Property args type is dyn safe.

These 

```rust
/// Foo docs.
#[property(context, default(Default::default(), 32))]
pub fn foo<T: VarValue>(child: impl UiNode, foo: impl IntoVar<T>, bar: impl IntoValue<u8>) -> impl UiNode {
    // .. node
    child
}

// EXPANDS:

/// Foo docs.
pub fn foo<T: zero_ui_core::VarValue>(
    child: impl zero_ui_core::UiNode, 
    foo: impl zero_ui_core::var::IntoVar<T>, 
    bar: impl zero_ui_core::var::IntoValue<u8>,
) -> impl zero_ui_core::UiNode {
    // .. node
    child
}

#[doc(hidden)]
pub struct foo_Args<T: VarValue> {
    foo: BoxedVar<T>,
    bar: u8,
}
impl<T: VarValue> foo_Args<T> {
    pub fn __ctor__(foo: impl IntoVar<T>, bar: impl IntoValue<u8>) -> Box<dyn zero_ui_core::property::Args> {
        Box::new(foo_Args {
            foo: IntoVar::into_var(foo).boxed(),
            bar: bar.into()
        })
    }

    // used by new named.
    pub fn foo(foo: impl IntoVar<T>) -> BoxedVar<T> {
        foo.into_var().boxed()
    }
    pub fn bar(bar: impl IntoValue<u8>) -> u8 {
        bar.into()
    }
}
impl<T: VarValue> zero_ui_core::property::Args for foo_Args<T> {
    fn default() -> Option<Box<dyn zero_ui_core::property::Args>> {
        Some(Box::new(Self::__ctor__(Default::default(), 32)))
    }

    fn new_boxed(inputs: Box<[Box<dyn Any>]>) -> Box<dyn zero_ui_core::property::Args> {
        zero_ui_core::property::assert_new_boxed_len(Self::inputs(), &inputs);
        Box::new(Self {
            foo: zero_ui_core::property::set_var_downcast::<bool>(inputs[0], Self::inputs(), 0),
            bar: zero_ui_core::property::set_value_downcast::<u8>(inputs[1], Self::inputs(), 1),
        })
    }

    fn clone_boxed(&self) -> Box<dyn zero_ui_core::property::Args> {
        Box::new(Self {
            foo: self.foo.clone(),
            bar: self.bar.clone(),
        })
    }

    fn name() -> &'static str {
        "foo"
    }

    fn priority() -> zero_ui_core::property::Priority {
        zero_ui_core::property::Priority::Context
    }

    fn source_loc() -> zero_ui_core::property::SourceLoc {
        zero_ui_core::property::source_loc!(/* fn foo span */)
    }

    fn inputs() -> &'static [zero_ui_core::property::Input] {
        static INPUTS = [
            zero_ui_core::property::Input {
                name: "foo",
                kind: InputKind::Var,
                ty: TypeId::of::<T>(),
                ty_name: type_name::<T>(), // always include inspector info, #[cfg(inspector)] will control if we inject inspector nodes only.
                is_input_ty: zero_ui_core::property::is_input_var::<T>,
            },
            zero_ui_core::property::Input {
                name: "bar",
                kind: InputKind::Value,
                ty: TypeId::of::<u8>(),
                ty_name: type_name::<u8>(),
                is_input_ty: zero_ui_core::property::is_input_value::<u8>,
            }
        ];
        &INPUTS
    }

    // if there no value in args this method does not need to generate, the default implement covers it.
    fn input_value(&self, i: usize) -> &dyn AnyVarValue {
        match i {
            1 => &self.bar,
            n => zero_ui_core::property::get_value_panic(Self::inputs(), n)
        }
    }

    fn input_var(&self, i: usize) -> &dyn AnyVar {
        match i {
            0 => &self.foo,
            n => zero_ui_core::property::get_var_panic(Self::inputs(), i)
        }
    }

    fn set_value(&mut self, i: usize, boxed_value: Box<dyn Any>) {
       match i {
            1 => {
                self.bar = zero_ui_core::property::set_value_downcast::<u8>(double_boxed_var, Self::inputs(), i);
            }
            n => zero_ui_core::property::set_value_panic(Self::inputs(), i)
       }
        
    }

    fn set_var(&mut self, i: usize, double_boxed_var: Box<dyn Any>) {
        match i {
            0 => {
                self.foo = zero_ui_core::property::set_var_downcast::<T>(double_boxed_var, Self::inputs(), i)
            }
            n => zero_ui_core::property::set_var_panic(Self::inputs(), i)
        }
    }

    fn instantiate(&self, child: BoxedUiNode) -> BoxedUiNode {
        foo(child, self.foo.clone(), self.bar.clone()).boxed()
    }
}

#[doc(hidden)]
#[macro_export]
macro_rules! foo_code_gen_h1sh {
    (if priority(context) {
        $($tt:tt)*
    }) => {
        $($tt)*
    };
    (if priority($other:ident) {
        $($tt:tt)*
    }) => {
        // ignore
    };

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

    (if input(foo) { 
        $($tt:tt)*
    }) => {
        $($tt)*
    };
    (if !input(foo) { 
        $($tt:tt)*
    }) => {
        // ignore
    };
    (if input(bar) { 
        $($tt:tt)*
    }) => {
        $($tt)*
    };
    (if !input(bar) { 
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

    // used by when builder.
    (input_index(foo)) => { 
        0
    };
    (input_index(bar)) => { 
        1
    };

    // assist the named constructor mode, fields are sorted in the macro.
    (<$foo_Args:ty>::__ctor__($bar:ident, $foo:ident)) => {
        $foo_args::__ctor__($foo, $bar)
    };    
}
#[doc(hidden)]
pub use foo_code_gen_h1sh;

#[doc(hidden)]
pub mod foo {
    pub use super::{foo_Args as Args, foo_code_gen_h1sh as code_gen};
}
```

Widget expands too:

```rust
my_wgt! {
    foo<bool> = true, 33;
}
// EXPANDS:
let __foo__ = foo::Args::<bool>::__ctor__(true, 33);

// -------
my_wgt! {
    foo<bool> = {
        foo: true,
        bar: 33,
    };
}
// EXPANDS:
let __foo__ = {
    foo::code_gen!(if input(foo) { 
        let foo = foo::Args::foo(true);
    });
    foo::code_gen!(if input(bar) {
        let bar = foo::Args::bar(33);
    });
    foo::code_gen!(if !input(foo) {
        compile_error!("unexpected input `foo`");
    });
    foo::code_gen!(if !input(foo) {
        compile_error!("unexpected input `bar`");
    });

    foo::code_gen!(<foo::Args>::__ctor__(bar, foo))
};
// EXPANDS:
let __foo__ = {
    let foo = foo::Args::foo(true);
    let bar = foo::Args::bar(33);
    foo::Args::__ctor__(foo, bar)
};
```

The property can then be added to a styleable widget collection or instantiated.

```rust
// INSTANTIATE.
let __node__ = __foo__.instantiate(__node__);

// STYLEABLE.
__widget__.push_context(__foo__);
```

