* ShapedText Issues
    - Can't use glyph offset to determinate width of segments.
    - The problem is RTL text glyphs are reordered, they can end-up mixed all over the line.
    - Need to rethink all direct manipulations in `ShapedText`.
        - `slip`, `slit_remove` and `extend` can't be implemented correctly.
        - These methods can still work at the line level, like can only split at a line break.
    - We only use these methods to implement font fallback.
        - Current font fallback impl is incorrect even in pure LTR text.
        - The fallback fonts can easily have different widths, so the wrap can end-up incorrectly.
        - Can we implement font fallback as it is shaping?
        - If we implement font fallback in a different way we can remove the edit methods.
            - They are complicated and if we are only going to support line level editing a user can
              just use multiple `ShapedText` instances if they really need to be modifying a single line.

* Review bidi text across inlined widgets.
    - HTML reorders across any span in the line, the background
        `<p>النص ثنائي الاتجاه (بالإنجليزية: Bi-directi<span>onal text)‏ هو </span>نص يحتوي على نص في كل من</p>`
        the span in the sample is split in two parts, the Arabic text ends up at the visual right-most.
    - To support this *automatically* at the layout level would require the text widgets releasing all control of the
      positioning of segments in the inline joiner rows.
    - We already don't support embedding widgets directly in text, if we ever implement HTML-to-widget the example
        becomes 3 span widgets, we can make it become 4 spans.
    - Can the users work around this issue by using a more semantic division of widgets?
    - Test Arabic with embedded English phrases in markdown.
        - Make a work in the English phrase **bold**.

* Implement vertical text.
    - Need to expand `LayoutDirection` to define chars and lines direction.
    - Or a different layout property for the "lines" direction.
    - See `./Layout.md#Direction`.

* Continue "#Parallel UI" in `./Performance.md`.
    - Refactor services into `app_local!` backed structs, with associated functions.
        - Remove `Services`.

* Review all docs.
    - Mentions of threads in particular.