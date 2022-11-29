* Implement inline layout.
    - Just flow LTR for now.
    - `wrap!`.
        - Spacing bug, if we have spacing 5 in main and nested, partial rows end-up with double spacing.
            - Nested applies spacing for the row fragment, main applies spacing normally.
            - If we make main not apply row spacing for partial rows then there is no spacing when nested has no spacing.
            - Can this be solved without an extra value in `InlineLayout`?
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