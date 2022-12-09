# Nested Animation Changes

* New handle for each animation.
* Shared importance for nested animations.

* Can't scroll with wheel in inspector window after it is focused by parent.

* `wrap!` bugs:
    - Need to track row height?
    - Need to track all rows in the `InlineLayout`?
    - Does not grow to fit children when possible.

* Implement `markdown!`.
* Implement inline info in bounds info.
* Implement `TextAlign` across multiple inlined texts.
* Implement `LayoutDirection` for `flow!`.

* Continue "#Parallel UI" in `./Performance.md`.
    - Refactor services into `app_local!` backed structs, with associated functions.
        - Remove `Services`.
* Review all docs.
    - Mentions of threads in particular.

# Other

* Implement window `modal`.
    - Mark the parent window as not interactive.
    - Focus modal child when the parent gets focused.
    - This is not the full Windows modal experience, the user can still interact with the parent chrome?
        - Can we disable resize, minimize, maximize?