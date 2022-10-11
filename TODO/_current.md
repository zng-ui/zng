* Review Event hooks and Command data unload.
    - If we unload a headless app and reuse the thread hooks stay alive.
    - Only command metadata for commands that where subscribed once is unloaded.
    - Have an AppId and compare it?

* Use `impl_ui_node(struct Node { ..})` syntax everywhere.
    - Rename to `#[ui_node]`.
    - Custom delegate with pseudo-attribute `#[delegate]` applied to member.
    - Document all this.
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