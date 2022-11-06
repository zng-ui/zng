* Refactor to minimal docs generation that does not require custom post-processing?
    - What if we make `properties!` be a normal macro that expands to a `properties` mod?
        - The properties defined in the macro become public `#[doc(no_inline)]` re-exports.
        - We have many widgets that already define a `pub mod properties`, but only for the
            properties declared for the widget, maybe we can join mods?
            - If `pub mod properties { }` is present it is used for the `properties!` pseudo-macro expansion.
            - Some widgets define a full named property and re-export for self with smaller name.
                - `text!` defines `text_color`, re-exports as `color`.
                - Can still work.
            - To support existing `pub mod properties` we need to collect every property declaration name to avoid import conflict.
                - `foo = true;` can't be pub use in context that declares `pub fn foo(..)`.
            - Right now we avoid declaring properties nested inside the widget::properties module, we manually re-export.
                - Small confusion, but uses can just avoid manually re-export with `*`.
                - Widget macro can check manual `pub use foo` to avoid re-export too.
        - If we use `#[doc(no_inline)]` the custom docs for `properties!` does not render.
        - If we don't use it, rustdoc gets confused and starts linking to properties inside the widget, instead of their original decl.
        - We can have docs in the `pub mod properties`, they link to the property function inside properties.
            - This works, we already need to do this for `when` docs.

* Refactor animate sleep tracking, to allow refactoring AnimationArgs to be an Rc, to allow real `Var::modify` animation.
    - Using clone for now, after merge refactor this.

* Review nodes that call `(de)init(ctx)`, are they causing a widget handle collection to grow uncontrolledly?

* If `ui_list![]` auto boxes in the macro params, but the returned type does not auto-boxes on `.push`.
    - Before refactor we had `WidgetVec(pub Vec<BoxedWidget>)`.

* Implement `widget::path.property` syntax support in widget instantiation and `when` expressions.
    - in when expressions: `when #foo.foo.1`, has ambiguity with `when #foo.foo`.
        - Mostly want it to support `#self.exported_prop`, nice callback to the previous syntax.
        - If we can't establish unambiguity, assume `#property_ident`, support `#::widget_ident` to select a widget.

* Implement pre-bind for when expressions.
* Implement when assign for `UiNode`, `UiNodeList` and `WidgetHandler`.
* Rename `toggle::selection` to `toggle::selector` or rename `Selector` to `Selection`.
* Merge `Property.new/new_when` into a single constructor that uses some kind of provider interface.
    - Like the nightly Any provider.
* Improve widget property imports, when inheriting from widgets a `use self::*;` can override inherited properties.
    - In the `image` example we need the full path to set the window size because of this.
* Refactor property priority into constants: `#[property(CONTEXT)]`.
* Test property generics `value::<bool> = true; when *#is_something { value::<u32> = 32; }`.
* Review `IntoVar` and `IntoVarValue` constrains, we don't need then to be debug/clone anymore?
* Const errors don't show if  there is a compile error, so the when `!foo::ALLOWED_IN_WHEN_EXPR` does not show.
    - Generate a placeholder `__w_0__`?

* Implement all `todo!` code.