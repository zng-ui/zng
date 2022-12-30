# Layout TODO

## Inline Align

Current inline API has problems.

### Inline Requirements

* Custom widgets can participate, not just text.
* Properties can have access to each row box in the widget (to clip background, or any effect like this).
* Rows can be aligned horizontally.
* Widgets can be block only, they are inlined as a block.
* Widgets that support inlining must also support block only layout.
* Flow direction, as defined by `LayoutDirection`.

### Current API

Currently have `InlineLayout`, `WidgetMeasure::inline`, `WidgetLayout::inline` and `LayoutMetrics::inline_advance`.

Limitations:

* No detailed row info about the widget, background of wrapped text does not is not clipped correctly.
  - Have a `rows: Vec<PxRect>` in `InlineLayout`?  
* Rows cannot be aligned.
  - **Parent needs measure of first and last row already wrapped to align!!**
  - Parent panel controls align of first and last row, children control align of its own mid-rows.
* Flow direction, only flows left-to-right.
  - Controlled by the `LayoutMetrics::direction`.
  - Same as align, parent panel defines first and last row rect, children defines mid-rows.

## Min Constrains Reset

* Review PxConstrains::min in every panel, should be zero? 

## Direction

* Integrate `LayoutDirection` with all widgets.
  - wrap
  - grid

## Grid 

* Cell align.
* Column & Row align for when all fixed is less then available.
* Masonry align?
* Support `lft` in spacing.
        - And padding? Need to capture padding if the case.
* Add contextual grid info, like the text `TextLayout`, to allow custom properties to extent it (like an special border property).