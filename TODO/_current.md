* Implement inline layout.
    - Just flow LTR for now.
    - `wrap!`.
        - Inline items.
        - Inline info for parent.
        - Use nested `wrap!` to chunk the icon example as a demo of the performance benefits of logical subdivision.
    - `text!`.
    - Properties like border, margin?
        - Test other frameworks.
            - WPF has an special base widget for "run" items.

* Implement `LayoutDirection` for `flow!`.

* Continue "#Parallel UI" in `./Performance.md`.
* Review all docs.
    - Mentions of threads in particular.

# Other

* Implement window `modal`.
    - Mark the parent window as not interactive.
    - Focus modal child when the parent gets focused.
    - This is not the full Windows modal experience, the user can still interact with the parent chrome?
        - Can we disable resize, minimize, maximize?