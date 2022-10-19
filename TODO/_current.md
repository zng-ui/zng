# All Dyn Rewrite

* Finish implementing new dynamic widget.
    - Implement helper methods for doing things like moving a property to the top of the pile of its own priority.
    - Implement widget instance info, type etc.
        - Use info in `widget_base::mixin` error log.
* Refactor all widgets to use the new API.
* Remove custom docs stuff.
* Update docs of new macros.

* Merge.

# Other

* Update webrender to fx-106
* Refactor animate sleep tracking, to allow refactoring AnimationArgs to be an Rc, to allow real `Var::modify` animation.
    - Using clone for now, after merge refactor this.

* Review nodes that call `(de)init(ctx)`, are they causing a widget handle collection to grow uncontrolledly?

* Implement all `todo!` code.