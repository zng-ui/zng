# All Dyn Rewrite

* Focus example, no highlight focus after button move to new window.
    - is_focused_hgl is `true`, but focus_highlight does not reflect the state.
    - Its the style reload that happens because of (re)init.
        - When not working after reuse?
* Review "!!:"

* Refactor to minimal docs generation that does not require custom post-processing?
* Update docs of new macros.
* Test all.
* Test build all.
* Merge.

# Other

* Update webrender to fx-106
* Refactor animate sleep tracking, to allow refactoring AnimationArgs to be an Rc, to allow real `Var::modify` animation.
    - Using clone for now, after merge refactor this.

* Review nodes that call `(de)init(ctx)`, are they causing a widget handle collection to grow uncontrolledly?

* Implement all `todo!` code.

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