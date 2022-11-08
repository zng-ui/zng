* Hacks to remove
    - Properties with the same name as widgets cause conflict that need to be resolved manually.
        - See `style` and `style_property`.
        - Resolved by not having properties be modules.
        - Can be `struct`.
            - Need to figure-out generics for things that don't need generic, like ID.
                - Could just require it.
            - Users need to pub import to pub use property.
                - Is this even a downside?
    - Properties re-export module causes we to have to add a `super::` to types and doc links.
        - It cannot be perfect, we are guessing types from tokens.
        - Properties could be re-exported directly in the widget module.
            - It makes visual sense, we set then directly in the widget, why not `widget::property`.
            - Can remove special syntax of `property_id!/_args!`.

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

* Review `image` widget property names.
    - Remove `image_` prefix?
    - Or rename-it to `img_`?
    - Same for `text::text_color`.