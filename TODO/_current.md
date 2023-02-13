# Inline Bidi

* If `text!` depends on `viewport` metrics the segmented texts can be wrapped incorrectly and the lines not aligned properly.

* Wrap panel layout refactor:
  - Review `spacing`, how does it work for segmented widgets.
    - Using spacing with fragmented text can affect row width?
    - Maybe we can say that horizontal spacing is one per widget only.

* Test nested `wrap!` panels with bidi.

# Other

* Implement vertical text.
    - Need to expand `LayoutDirection` to define chars and lines direction.
    - Or a different layout property for the "lines" direction.
    - See `./Layout.md#Direction`.

* Continue "#Parallel UI" in `./Performance.md`.
    - Refactor services into `app_local!` backed structs, with associated functions.
        - Remove `Services`.

* Review all docs.
    - Mentions of threads in particular.