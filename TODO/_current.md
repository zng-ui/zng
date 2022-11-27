* Implement inline layout.
    - Implement `InlineLayout`.
        - How to mark widgets as **not** inline when inside inline without requiring they all to call another wrapper?
        - Do we need to? The bounds check already cancels it.
            - Cancels it, but the child thinks it is inlined.
            - This is a problem for a single widget too.
                - Say a text child adjusts the first line advance to inline, but a border property causes it to become box.
                  the text ends up advanced inside the box.

* Implement `LayoutDirection` for `flow!`.
* Use nested `wrap!` to chunk the icon example as a demo of the performance benefits of logical subdivision.

* Continue "#Parallel UI" in `./Performance.md`.
* Review all docs.
    - Mentions of threads in particular.

# Other

* Implement window `modal`.
    - Mark the parent window as not interactive.
    - Focus modal child when the parent gets focused.
    - This is not the full Windows modal experience, the user can still interact with the parent chrome?
        - Can we disable resize, minimize, maximize?