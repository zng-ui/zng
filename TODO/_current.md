# Inline Bidi

* Can't combine levels?
  - levels of a single text run are different from the same text in parts in a wrap panel.
  - Only space segments for now, but this highlights a real issue, an entire widget can be quoted by
    special bidi chars before and after it.
  - We need to compute levels on the `wrap!`.
  - No need to store levels in the measure info?
* Wrap panels need to do something about blocks.
  - Treat then like an isolated insert?
* Wrap panels need to shape the "row" for each widget in its row to cover all reordered segments.
  - Change horizontal positioning all to resort algorithm.
  - But still track wrap the old way?

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