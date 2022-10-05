# Var/Update Rewrite

* Remove `UiNode::subscriptions`, should be mostly removed already.
    - Use `impl_ui_node(struct Node { ..})` syntax everywhere.
    - Refactor render image "retain", can't see if windows subscribe now (previous impl was iffy).
        - Make it explicit, only if requested.
* Review "!!:".
* Docs.
* Test.

* Stop propagation when all items in delivery list visited.
* Stop propagation when it is requested.
* Refactor animate sleep tracking, to allow refactoring AnimationArgs to be an Rc, to allow real `Var::modify` animation.
    - Using clone for now, after merge refactor this.

* `Var::display` like `Path::display`,  for easy formatting.
* `RcVar::set` and other infallible overrides.
* Review `VarCapabilities` variant names.
* Review `unsafe`, only use when there is no alternative.
* Review nodes that call `(de)init(ctx)`, are they causing a widget handle collection to grow uncontrolledly?

* Implement event handlers using a callback in the event that queues the handlers to run once. 
    - This avoids the linear event update search.
    - This causes lets us unify all event handles to a single `EventHandle` like the `VarHandle`.
* Review Command unload, if we only modify a command meta and don't create any handlers it does not register for cleanup.
    - Bug already existed in previous implementation.
    - Have an AppId?
* Implement all `todo!` code.
