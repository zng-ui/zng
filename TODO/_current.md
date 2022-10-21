# All Dyn Rewrite

* Finish implementing new dynamic widget.
    - Implement pre-bind for when blocks.
        - And cfg, lints.
    - Implement helper methods for doing things like moving a property to the top of the pile of its own priority.
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
