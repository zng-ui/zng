* Review `child_insert_right`, it does not work because `child_layout` is not applied because a widget is detected.
    - Problem, we actually have two widgets, so the child layout is set on it?
    - Review `WidgetLayout`, need a way to make `child_layout` work like `children_layout` in the event of multiple direct
        children widgets.
    - When implemented `children_layout` can be removed?
    - Even if we detect multiple children, how to detect children collections with only a single child?
        - Is it ok to use the single child outer-transform in this case?
        - Also can end-up with a panel that only has a single full widget child, but has multiple simple node children.
            - Padding in the case only applies to the widget child if we are auto-detecting.
        - Could change `with_children` to flag `with_child`.
            - The `child_insert_x` properties need to flag it too.
            - Maybe just a method in `WidgetLayout::use_first_child_outer_transforms`
        - Could change `with_child` to always be like `with_children`.
            - Already did not implement optimizations for `with_children`.
            - Adds another reference-frame per-widget.

* Review checkbox with different font, is the mark affected?
* Implement `radio_style!` for toggle.
* Implement `child_insert_start/end/top/bottom`.

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