# Inline Bidi

* Wrap measure and layout can happen with different max_width, causing many reshapes.
* Test nested `wrap!` panels with bidi.

# Other

* Update webrender to fx110.
* Implement vertical text.
    - Need to expand `LayoutDirection` to define chars and lines direction.
    - Or a different layout property for the "lines" direction.
    - See `./Layout.md#Direction`.

* Continue "#Parallel UI" in `./Performance.md`.
    - Refactor services into `app_local!` backed structs, with associated functions.
        - Remove `Services`.

* Review all docs.
    - Mentions of threads in particular.