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
    - Instead of a `#[priority]` pseudo we can have a method for this in the `WidgetBuilder`.
* Capture property args to use in a different way.
* Declare `when` blocks that dynamically merge with the widget assigns.
    - Properties used in the condition expr need to be rebind-able.
    - When assigns added by styles need to join in the property instance.

Widget problems:

* Massive code gen in the user crate, the biggest LLVM counts for single items if for widget instantiation functions.
    - Most of this is caused by the fact the entire widget provided properties is copied in every usage.
* If we refactor widgets to always be dynamic we can reduce all this to a single function call to get the default widget then override by
the user changes.
* Rustfmt does not work inside macro! { .. } style macros.
    - Can't really solve this one, they are working on it.
* Rust-analyzer does not find the property paths to go-to-def and show help on hover.
    - If we simplify enough we can expand the widget in one go, that maybe fixes this.

Limitations:

Massive code-gen is solved by dynamic only, so the biggest problem left is the usability, because rust-analyzer can't resolve
properties, this is all caused by the macro recursion simulate "early eval" to get everything in one place. If we add limitations
until the widget can expand without recursion all items become interactive.

* Change `inherit!(#path);` to just expand to a `pub use #path::*;`, (and a call to `#path::__intrinsic__`).
    - Expand the path with the span set, let Rust validate name conflicts.
* No more `remove`, `#[required]` or any of that stuff.
* Instantiation is just a `*` import in a block scope too.
    - This actually adds some cool things we can do, like auto import a specific enum that is only useful inside a widget.