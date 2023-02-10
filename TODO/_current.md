# Inline Bidi

* Wrap panel layout refactor:
  - Review layout offset in general, need to be removed or get a better name?
    - Still used to implement wrap?
  - Review alloc during layout, any way to avoid it at least for instances that only have block items.
  - Review `spacing`, how does it work for segmented widgets.
    - Using spacing with fragmented text can affect row width?
    - Maybe we can say that horizontal spacing is one per widget only.

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