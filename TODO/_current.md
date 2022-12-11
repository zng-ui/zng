* Image render requests a parent window, it causes errors because window with parents can't be parent of the image headless window.
    - We need the parent to load the right color-scheme in the image.

* Implement `markdown!`.
    - Paragraph spacing.

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