# All Dyn Rewrite

* Finish implementing new dynamic widget.
    - Implement pre-bind for when expressions.
     - When assigns need to "import" private properties in the same properties! block.
    - Implement a way to generate a compile error when inheriting build from mixin.
        - Maybe widgets can define a macro that is called when inherited?
    - Can we make `styleable_mixin` now that everything is dynamic?
        - It works as a node that is immediate child of another, make build accept intrinsic "outer-most".
            - Use the new `NestPosition` to insert the intrinsic at the outermost node should work.
    - Make very basic `container` and `panel` widgets directly in `widget_base`?

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
