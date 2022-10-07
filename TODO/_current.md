# Var/Update Rewrite

* Animation example: panic.
* Gradient example: resize causes white flashes.
    - Scrollbars don't highlight.

* Use `impl_ui_node(struct Node { ..})` syntax everywhere.
* Review "!!:".
* Review context vars usage, we abused the previous API to pass "service like" references to a context, now these values get cloned.
    - Maybe we can make an official API for these?
        - A `ContextValue<T>` that is a boxed `RcVar<T>` internally, but allows immediate modification?
* Review context var stack, nested borrows.

* Docs.
* Test.

* Stop propagation when all items in delivery list visited.
* Stop propagation when it is requested.
* Refactor animate sleep tracking, to allow refactoring AnimationArgs to be an Rc, to allow real `Var::modify` animation.
    - Using clone for now, after merge refactor this.

* `Var::display` like `Path::display`,  for easy formatting.
* `RcVar::set` and other infallible overrides.
* Review `VarCapabilities` variant names.
* Review `Var::actual_var`, can we make it `actual_var(self)` instead of a ref?
    - We use this method in context-vars, so values get cloned a lot.
* Review `unsafe`, only use when there is no alternative.
* Review nodes that call `(de)init(ctx)`, are they causing a widget handle collection to grow uncontrolledly?

* Implement event handlers using a callback in the event that queues the handlers to run once. 
    - This avoids the linear event update search.
    - This causes lets us unify all event handles to a single `EventHandle` like the `VarHandle`.
* Review Command unload, if we only modify a command meta and don't create any handlers it does not register for cleanup.
    - Bug already existed in previous implementation.
    - Have an AppId?
* Implement all `todo!` code.
