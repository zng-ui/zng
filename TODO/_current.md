# Remove Layout

Remove widget outer offset, parents always implement transform using the child offset.

* Refactor `stack!`.
    - Need to figure out `translate_baseline`.
    - How does `wrap!` baseline works again?
* Refactor `grid!`.
    - Grid only z-sorted children, can't do this anymore?
        - Can use 3 `PanelList` entries, users can z-sort columns or rows in isolation then, minimal perf impact.
* Remove `with_outer` and `with_branch`.
    - Review all other layout methods, the baseline stuff can't work in panels for example?
* Implement optimized `push_child`, that delays the reference_frame until the first inner boundary.
    - The idea is that it automatically creates a reference frame if something tries to render.
    - The id and item index are requested up-front, but the method returns a flag that indicates if a 
      reference frame was actually created.
* Remove outer offset from bounds.
* Finish `!!:` TODOs. 

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