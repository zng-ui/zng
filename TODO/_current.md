# All Dyn Rewrite

* Intrinsic problems:
    - If we capture a property it is removed, a derived widget can add the same property again after.
    - We need to resolve all property overrides and unsets first, then generate the intrinsics.
        - Two step intrinsics, remove captures, generate node.

* `#[ui_node(..)]` tries to validate `with_context` overrides.
    - Need a list of all delegates, only validate those, we have other methods in `UiNode`.

* Re-implement `SortedWidgetVecRef` for use in window layers.

* Implement `widget::path.property` syntax support in widget instantiation and `when` expressions.
    - in when expressions: `when #foo.foo.1`, has ambiguity with `when #foo.foo`.
        - Mostly want it to support `#self.exported_prop`, nice callback to the previous syntax.
        - If we can't establish unambiguity, assume `#property_ident`, support `#::widget_ident` to select a widget.

* Finish implementing new dynamic widget.
    - Fix intrinsics.
    - Improve "capture-only" properties.
    - Use pre-bind in widget intrinsics.
        - This will fix the when assign on renamed properties and import conflicts like the `style` property in button.
    - Implement pre-bind for when expressions.
    - When assigns need to "import" private properties in the same properties! block.
    - Use the term `child` and `children` in widgets, the rename to `content` and `items` does add value.
    - Use `style_mixin` in places.

* Refactor all widgets to use the new API.

* Reimplement inspector.
    - Implement widget instance info, type etc.
        - Use info in `widget_base::mixin` error log.
* Remove custom docs stuff.
    - Refactor to minimal docs generation that does not require custom post-processing.
* Update docs of new macros.
* Test all.

* Merge.

# Other

* Update webrender to fx-106
* Refactor animate sleep tracking, to allow refactoring AnimationArgs to be an Rc, to allow real `Var::modify` animation.
    - Using clone for now, after merge refactor this.

* Review nodes that call `(de)init(ctx)`, are they causing a widget handle collection to grow uncontrolledly?

* Implement all `todo!` code.
