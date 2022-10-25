# All Dyn Rewrite

* Finish implementing new dynamic widget.
    - Implement when instantiation.

* Reimplement inspector.
    - Prompt print.
    - Tracing nodes?
        - Maybe just trace at the widget level by default, property trace level was to verbose.
    - Can we inspect capture?
    - Avoid intermediary debug alloc.
        - Need to move `Var::debug` to `VarAny`.

* Review "!!:"

* Remove custom docs stuff?
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

* If `ui_list![]` auto boxes in the macro params, but the returned type does not auto-boxes on `.push`.
    - Before refactor we had `WidgetVec(pub Vec<BoxedWidget>)`.

* Implement `widget::path.property` syntax support in widget instantiation and `when` expressions.
    - in when expressions: `when #foo.foo.1`, has ambiguity with `when #foo.foo`.
        - Mostly want it to support `#self.exported_prop`, nice callback to the previous syntax.
        - If we can't establish unambiguity, assume `#property_ident`, support `#::widget_ident` to select a widget.
* Implement pre-bind for when expressions.