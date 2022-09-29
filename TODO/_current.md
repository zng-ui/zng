# Var/Update Rewrite

* Fix all build errors.
* AnyWhenVarBuilder::from_var has a problem with new contextualized var.
    - Maybe a custom contextualizing var that keeps the builder?
    - Or contextualize each input.
* Remove `UiNode::subscriptions`, should be mostly removed already.
    - Refactor WidgetHandle, some other context-var wrappers also.
* Docs.
* Test.

* Stop propagation when all items in delivery list visited.
* Stop propagation when it is requested.
* Refactor animate sleep tracking, to allow refactoring AnimationArgs to be an Rc, to allow real `Var::modify` animation.
    - Using clone for now, after merge refactor this.

* `Var::display` like `Path::display`,  for easy formatting.
* Review `VarCapabilities` variant names.
* Review `unsafe`, only use when there is no alternative.

* Implement event handlers using a callback in the event that queues the handlers to run once. 
    - This avoids the linear event update search.
* Review Command unload, if we only modify a command meta and don't create any handlers it does not register for cleanup.
    - Bug already existed in previous implementation.
    - Have an AppId?
* Implement all `todo!` code.

# Better Node Macro

* We really need a better way to declare nodes, some property nodes have have 20 lines of generics metadata.
    - And now they all have init/deinit to event and var handles.