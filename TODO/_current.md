* Refactor animate sleep tracking, to allow refactoring AnimationArgs to be an Rc, to allow real `Var::modify` animation.
    - Using clone for now, after merge refactor this.

* Review nodes that call `(de)init(ctx)`, are they causing a widget handle collection to grow uncontrolledly?

* Implement all `todo!` code.

# All Rc Rewrite

* Implement `RcWidget`.
* Implement `RcWidgetList`.
* Refactor API to remove all return references (turn they into visitors?).

# All Dyn Rewrite

* Finish implementing takeout or invent another method of handling these inputs.
    - We want nodes to receive `impl UiNodeList` to support the zero-cost usage of composing a node inside a property.
    - But in properties it needs to be boxed.
    - I can't just be `UiNodeList::boxed_all() -> UiNodeVec` because this loses the custom features of the implementer.
        - This method is weird, probably needs to ne removed.
        - But for now we can use it?
            - It will break some z-sorted examples.
            - But is better than rewriting it now, constrained by widget needs that may not exist after the widget rewrite.
            - lets just mockup a boxed() -> BoxedUiNode that is just a type alias for now.
    - The entire `EditableWgtNode` + snapshots API is needed because of takeout args.
        - If we had no take-outs we could just use `wgt.clone().build()` in stylable widgets and store the args as the "snapshot".
        - We could turn `impl UiNode` into [`RcNode<BoxedUiNode>`] that takes on init.
            - Even if the args are used incorrectly the node is just moved to the new parent.
        - Can we do the same for `impl UiNodeList` and `impl WidgetHandler<A>`?.
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
    - Right now

* Merge.