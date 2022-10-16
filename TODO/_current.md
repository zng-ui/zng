* Refactor animate sleep tracking, to allow refactoring AnimationArgs to be an Rc, to allow real `Var::modify` animation.
    - Using clone for now, after merge refactor this.

* Review nodes that call `(de)init(ctx)`, are they causing a widget handle collection to grow uncontrolledly?

* Implement all `todo!` code.

# All Dyn Rewrite

* Finish implementing new dynamic widget.
    - We have dynamic at the info level, need dynamic at the instantiated level?
    - Implement helper methods for doing things like moving a property to the top of the pile of its own priority.
* Implement new base widget.
* Test some widgets using the new API.
* Refactor all widgets to use the new API.
* Remove all previous proc-macros.
* Remove custom docs stuff.
* Update docs of new macros.
* Merge.