* Refactor animate sleep tracking, to allow refactoring AnimationArgs to be an Rc, to allow real `Var::modify` animation.
    - Using clone for now, after merge refactor this.

* Review nodes that call `(de)init(ctx)`, are they causing a widget handle collection to grow uncontrolledly?

* Implement all `todo!` code.

# All Rc Rewrite

* Refactor API to remove all return references.
    - Like `UiNode::try_state` or `UiNodeList`.
    - Implement a visitor `UiNode::with_info(f: FnOnce(&WidgetInfo) -> R) -> Option<R>` that groups every thing.
        - This lets we add stuff a lot quickly and without breaking changes too, right now we have multiple info related methods.

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

* Refactor `UiNodeList` and `WidgetList` to be actually boxable.

* Merge.

* Update webrender to fx-106