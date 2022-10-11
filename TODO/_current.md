* Review context vars usage, we abused the previous API to pass "service like" references to a context, now these values get cloned.
    - Maybe we can make an official API for these?
        - A `ContextValue<T>` that is a boxed `RcVar<T>` internally, but allows immediate modification?
        - `resolve_text` now alloc a box every UiNode method call for example.

* Refactor animate sleep tracking, to allow refactoring AnimationArgs to be an Rc, to allow real `Var::modify` animation.
    - Using clone for now, after merge refactor this.

* Review nodes that call `(de)init(ctx)`, are they causing a widget handle collection to grow uncontrolledly?

* Implement all `todo!` code.