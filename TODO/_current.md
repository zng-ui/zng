* Implement Unicode bidi text.
    - See https://docs.rs/unicode-bidi/
    - Review how CSS does it?
    - Need to work across elements?
    - Need to be something in the `LayoutContext`?
    - How does `lang` and `direction` interact with it?

* Implement vertical text.
    - Need to expand `LayoutDirection` to define chars and lines direction.
    - Or a different layout property for the "lines" direction.
    - See `./Layout.md#Direction`.

* Continue "#Parallel UI" in `./Performance.md`.
    - Refactor services into `app_local!` backed structs, with associated functions.
        - Remove `Services`.

* Review all docs.
    - Mentions of threads in particular.