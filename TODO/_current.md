# Remove Layout

Remove widget outer offset, parents always implement transform using the child offset.

* Implement optimized `push_child`, that delays the reference_frame until the first inner boundary.
    - We will use `ChildSpatialKey` to track when a reference frame was needed, but how do we track just some children needing it?
        - Maybe use a bit-field for compact storage of one flag for each child.
            - https://crates.io/crates/bit-vec
        - Maybe just enable from the first child that needs to the rest?
            - This means we can end-up using reference frames for full widgets, but the perf impact for unnecessary frames is minimal.
            - Usually a children collection only has widgets or non-widgets?
    - If multiple children need a ref-frame we use the `FrameValueKey::to_wr_child` to mix-in a child index, so we don't need to
      store a key for each child.
* Remove outer offset from `WidgetBoundsInfo`.
* Finish `!!:` TODOs.
* Review examples.

# Other

* Implement `switch_style!` for toggle.

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