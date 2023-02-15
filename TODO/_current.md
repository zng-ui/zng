# Inline Bidi

* Wrap panel layout refactor:
  - Fixed `spacing` to apply between each inlined widget change, even in interleaved bidi segments.
  - Unfortunately there is no way to estimate the extra spacing width in interleaved cases.
    - Will need to call `layout_bidi` in `measure` if has column spacing?

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