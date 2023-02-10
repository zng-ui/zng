# Inline Bidi

* Some times the `ensure_layout_for_render` gets an older version of the `layout_metrics`?
  - Seg counts don't match.
  - Even after requesting full reshape when counts don't match.

* Wrap panel layout refactor:
  - Optimize:
    - `item_segs` owns heap, and is not used if `bidi_layout_fresh`, two optimizations.
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