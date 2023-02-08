# Inline Bidi

* Wrap panel layout refactor:
  - Items can get resized and repositioned because of bidi sort.
  - Right now we just *flow* in the layout direction, adding offset and spacing to the next item.
  - Problems with this:
    - The bidi algorithm does not know block items.
      - See what HTML does, turn then into an *isolated insert*?
    - The `spacing` is not inserted in interleaved segments.
  - Change horizontal positioning all to resort algorithm.
    - But still track wrap the old way?
    - The 

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