# Inline Bidi

* Wrap panel layout refactor:
  - Fixed `spacing` to apply between each inlined widget change, even in interleaved bidi segments.
  - Unfortunately there is no way to estimate the extra spacing width in interleaved cases.
    - Will need to call `layout_bidi` in `measure` if has column spacing?
    - No, we need to add the spacing as the widgets are measured, because the next constrain is affected by the width consumed.
    - Is there any way to detect an "interleave insert point" without having all segments?
        - No, can only detect changes in direction, no guarantee they will all get a foreign segment insert.
    - Could say that spacing is added between all segments in inlined stuff?
        - No, the current way works in the icon example inlined nested wraps.
    - Can document the limitation, spacing is not designed for text inline.
    - Can change to only add spacing once for each child widget.
        - Still causes visual defects in bidi sorted content, but at least all segments stay inside the row span.

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