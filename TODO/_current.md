# Remove Layout

Remove widget outer offset, parents always implement transform using the child offset.

* Where is the children offset stored?
    - Need to store a `PxVector` for each child in panels.
    - Need to store a reference frame id? 
    - How to avoid pushing many reference-frames for children that are not in the scroll area?

## Remove `with_outer` and `with_branch`

* From `fill_node`.
* From `child_insert`, this is the most simple *panel*, test on it first.
* From `stack!`.  
* From `grid!`.
* From `wrap!`.

## Remove `outer_offset`

* From `WidgetBoundsInfo`.
* Review `end_pass`, is it needed to invalidate render reuse?
    - All widget offset changes are now known by the time the widget exits layout, so we can invalidate during the layout?

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