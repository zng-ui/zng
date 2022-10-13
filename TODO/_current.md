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

## Property

Requirements:

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
* Allow strongly typed read in `when #expr` for var and value.

Ok Limitations:

* All previous limitations, (no async, unsafe, const).
* Input bindings can only be `ident: impl T`.
* Input types can only be `impl UiNode`, `impl Widget`, `impl WidgetList`, `impl IntoVar<T>`, `impl IntoValue<T>`, we validate and replace with the full path.
    - For `on_` properties only `impl WidgetHandler` (will need to refactor widget events).
    - For `is_` only `StateVar`.
    - The first param can only be `impl UiNode`.
* Return type also can only be `impl UiNode`.
* Explicit generic types can only be `ident: VarValue`.
* No attributes allowed in input/output or generic params.
* No retention of full types to the extreme we have today, values are actualized (and vars boxed) early.
    - Property args type is dyn safe.

These changes lets us simplify the macro a a lot (no more associated type transform), and clamps on the type explosion at the
moment of instantiation, (no more cfg(dyn_))

## Widget Requirements

* Insert custom nodes at each priority level.
* Collect custom nodes in a dynamic list for styling.
    - We could make all widgets "dynamic/stylable" by default.
    - Then widget creation can be some sort of `WidgetBuilder` instance creation.
* Finetune priority of a property within its own priority group.
* Declare capture-only "properties", that are redirected to the custom nodes.


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

## New Widget

Widget problems:

* Massive code gen in the user crate, the biggest LLVM counts for single items if for widget instantiation functions.
    - Most of this is caused by the fact the entire widget provided properties is copied in every usage.
* If we refactor widgets to always be dynamic we can reduce all this to a single function call to get the default widget then override by
the user changes.
* Rustfmt does not work inside macro! { .. } style macros.
    - Can't really solve this one, they are working on it.
* Rust-analyzer does not find the property paths to go-to-def and show help on hover.
    - If we simplify enough we can expand the widget in one go, that maybe fixes this.

```rust
#[widget]
pub mod container {
    inherit!(base);

    properties! {
        /// Docs.
        super::child as content = required!;
    }

    // Builder already contains all `base` and properties of container set by user, including content.
    // there is no more "capture-only" properties, properties like "content" or "id" can log an error and just return the input node.
    fn intrinsics(wgt: &mut WidgetBuilder) {
        let args = wgt.take_property(property_id!(content)).unwrap();// property_id! expands to ("content", TypeId::of::<content::Args>()).
        let child = args.get_node(0);
        wgt.set_child(child);
    }
}

#[widget]
pub mod foo {
    inherit!(super::container);

    properties! {
        /// Docs.
        super::margin = 10;
        content = DefaultNode;
    }

    fn intrinsics(wgt: &mut WidgetBuilder) {
        // instead of `new_context` the widgets can do this.
        wgt.insert_intrinsic(Priority::Context, |child| {
            with_context_var(child, FOO_VAR, true)
        });
    }

    // constructs the widget, can have any return type.
    fn build(builder: WidgetBuilder) -> CustomWidgetType {
        // base::build extracts the `id` here.
        CustomWidgetType(builder.build())
    }
}

// EXPANDS TO:

pub mod container {
    pub use zero_ui_core::base::{id, __build__};
    pub use super::child as content;

    fn intrinsics(wgt: &mut WidgetBuilder) {
        let args = wgt.take_property(property_id!(content)).unwrap();// property_id! expands to ("content", TypeId::of::<content::Args>()).
        let child = args.get_node(0);
        wgt.set_child(child);
    }

    pub fn __intrinsics__(wgt: &mut WidgetBuilder) {
        zero_ui_core::base::__intrinsics__(wgt);
        intrinsics(wgt, content)
    }
}

pub mod foo {
    pub use super::container::{id, content};

    pub use super::margin;

    fn intrinsics(wgt: &mut WidgetBuilder) {
        wgt.insert_intrinsic(Priority::Context, |child| {
            with_context_var(child, FOO_VAR, true)
        });
    }

    fn __default_content__() -> Box<dyn property::Args> {
        content::Args::new(DefaultNode)
    }

    fn __default_margin__() -> Box<dyn property::Args> {
        margin::Args::new(10)
    }

    pub fn __intrinsics__(wgt: &mut WidgetBuilder) {
        super::container::__intrinsics__(wgt);

        // defaults 
        wgt.insert_property(property_id!(content), __default__content__());
        wgt.insert_property(property_id!(margin), __default__margin__());
   
        intrinsics(wgt);
    }

    fn build(builder: WidgetBuilder) -> CustomWidgetType {
        CustomWidgetType(builder.build())
    }

    pub fn __build__(builder: WidgetBuilder) -> CustomWidgetType {
        build(builder)
    }
}

#[macro_export]
macro_rules! foo {
    (inherit=> $($continue:tt)*) => {
        $crate::foo::widget_inherit! {
            data { 
                property {
                    ident { margin }
                    default { true }
                    required { false }
                    cfg { }
                },
                property {
                    ident { content }
                    default { true }
                    required { true }
                    cfg { }
                },
                property {
                    ident { id }
                    default { true }
                    required { true }
                    cfg { }
                },
            }
            continue {
                $($continue)*
            }
        }
    }
    ($($tt:tt)*) => {
        $crate::foo::__widget_new! {
            data { $crate::foo }
            instance { $($tt)* }
        }
    }
}
```

Used like:

```rust
let foo = foo! {
    background_color = colors::RED;
    margin = 20;
};

// EXPANDS:

let foo = {
    let mut wgt = WidgetBuilder::new();
    foo::__intrinsics__(&mut wgt);
    wgt.insert_property(property_id!(background_color), background_color::Args::new(colors::RED));
    wgt.insert_property(property_id!(foo::margin), foo::margin::Args::new(20));
    foo::__build__(wgt)
};
```