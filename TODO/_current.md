* Test app_local event and commands.
    - Stack overflow in calculator, focus, text.
        - Recursion in ContextVar, review previous impl.
        - Bug is on master already
        - Before we kept all context values in a vec, and marked each as busy on borrow.
            - So nested borrows went for each ancestor.
            - This only works with the delegate style `with`, can't tell if the var is already read-locked.

* Review `ContextLocal`, default is not app-local?

* Update webrender to Fx107.

* Review `AppContextMut`.
* Use `ThreadContext` in `core::task`.
    - It is not just for UI threads?
* Continue "#Parallel UI" in `./Performance.md`.
* Review all docs.
    - Mentions of threads in particular.

# Other

* Implement window `modal`.
    - Mark the parent window as not interactive.
    - Focus modal child when the parent gets focused.
    - This is not the full Windows modal experience, the user can still interact with the parent chrome?
        - Can we disable resize, minimize, maximize?