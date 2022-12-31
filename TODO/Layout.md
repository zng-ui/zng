# Layout TODO

## Inline Align

### Inline Requirements

* Custom widgets can participate, not just text.
* Properties can have access to each row box in the widget (to clip background, or any effect like this).
* Rows can be aligned horizontally.
* Widgets can be block only, they are inlined as a block.
* Widgets that support inlining must also support block only layout.
* Flow direction, as defined by `LayoutDirection`.
* Support baseline align in-between children of different first/last row length.
* Integrate, `wrap::children_align` with `txt_align`.

### New Inline API

Parent panel defining an inline layout scope:

[x] - Measure each child using `MeasureContext::measure_inline`.
  - The returned `Option<WidgetInlineMeasure>` defines the advance left for the first line of the
    next item, and if it `None` the item is layout as inline-block.
  - The available width for the first row is communicated using `measure_inline`.
[ ] - Create a `vec node member` to store line info:
  - `max_line_height`
  - `max_line_width`
  - `first_widget_index` ?
  - `last_widget_index` ?
  - *Each child can participate in 2 lines, but needs to be layout only once.
  - Consider if there are any solutions without using a vec afterwards.
[ ] - For each child, define the first and last row rectangles.
  - Also define the extra space the mid-rows of each child must clear.
    - This is an space so that the mid-rows don't write over the current row if the next widget has a shorter first row.
  - The first and last row must be aligned in both dimensions.
    - For `FILL` align they must be resized too to distribute the space.
      - The resize range allowed is provided in the `WidgetInlineMeasure`.
      - !!: Does it need to be accessed after all children measure?
    - For `BASELINE` align find a baseline that allows all height without *clipping*.
      - Align first and last rows to this baseline.
      - The baseline of these rows is also in `WidgetInlineMeasure`.
[ ] - Layout each child using `LayoutContext::with_inline`.
  - The child outer transform is positioned with origin the same as the full row origin.
  - Except when the child is a single row, then it is positioned like an inline block.
  - In case the children don't have extra width to fill the panel can add maximum the height of a row of spacing too.
    - But don't add anything if this still does not cover half of the leftover space.

Child implementing inline layout:

[ ] - In measure use the `LayoutMetrics::inline_constrains`.
  - To detect if inline mode is enabled (it is `Some(_)`).
  - To get the max width for the first row.
[ ] - Measure every row, no alignment is needed, can skip mid rows if possible to find the last row anyway.
[ ] - Set the `WidgetMeasure::inline` config.
  - With the rect of the first and last rows.
  - With the baseline for the first and last rows.
  - With the max width the first and last rows can grow to `FILL`.
    - For text this is the `Justify` algorithm.
  - Return the desired size as the bounds of all rows.
[ ] - If measure can fill in a single row the `WidgetInlineMeasure::first` and `WidgetInlineMeasure::last` are equal.
[ ] - In layout use the `LayoutMetrics::inline_constrains`.
  - To detect if inline mode is enabled (it is `Some(_)`).
  - To get the are the first and last row must fill.
  - To get the extra vertical offset needed to start the mid-rows to clear the full first row.
[ ] - In layout if fill or justify the given rectangles if there is leftover space.
  - Items inside the first and last row are layout using the `LayoutDirection`.
  - If not possible to fill, align to `START`.
[ ] - Layout mid-rows using the widget's own align config.
  - Offset down by the `mid_clear` value.
[ ] - Set the `WidgetLayout::inline` rows list for the widget properties.

Panel as child (nested):

* !!: Just pass along the inline constrains?

### Wrap

* Make `children_align` be `TEXT_ALIGN_VAR` by default.

## Min Constrains Reset

* Review PxConstrains::min in every panel, should be zero? 

## Direction

* Expand `LayoutDirection` to support vertical text, and to control the direction *rows* are added.
  - See CSS `writing-mode`, lang define direction glyphs flow, but also direction wrap grows.
* Integrate `LayoutDirection` with all widgets.
  - grid, mirror grid indexes.

## Grid 

* Cell align.
* Column & Row align for when all fixed is less then available.
* Masonry align?
* Support `lft` in spacing.
        - And padding? Need to capture padding if the case.
* Add contextual grid info, like the text `TextLayout`, to allow custom properties to extent it (like an special border property).