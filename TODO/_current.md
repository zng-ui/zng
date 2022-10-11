* Document `#[ui_node(struct NewNode { .. })]` feature.

* Implement an APP_DEINIT thread local that gets run on app shutdown.
    - Use it to clear all other thread_local items associated with the lifetime of an app.
        - Event.
        - Command.

* Review context vars usage, we abused the previous API to pass "service like" references to a context, now these values get cloned.
    - Maybe we can make an official API for these?
        - A `ContextValue<T>` that is a boxed `RcVar<T>` internally, but allows immediate modification?
        - `resolve_text` now alloc a box every UiNode method call for example.

* Refactor animate sleep tracking, to allow refactoring AnimationArgs to be an Rc, to allow real `Var::modify` animation.
    - Using clone for now, after merge refactor this.

* Review `VarCapabilities` variant names.
* Review `Var::actual_var`, can we make it `actual_var(self)` instead of a ref?
    - We use this method in context-vars, so values get cloned a lot.
* Review nodes that call `(de)init(ctx)`, are they causing a widget handle collection to grow uncontrolledly?

* Implement all `todo!` code.

* Opening the image example flashes white.
    - Resizing gradient also flashes.