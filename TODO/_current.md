# All Dyn Rewrite

* Finish implementing new dynamic widget.
    - Implement pre-bind for when expressions.
    - When assigns need to "import" private properties in the same properties! block.
    - Use the term `child` and `children` in widgets, the rename to `content` and `items` does add value.
    - Make `styleable_mixin` now that the API supports it.
        - Remove `element` widget (already deleted need to stop using).
        - Use the new `NestPosition` to insert the intrinsic at the outermost node should work.

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
