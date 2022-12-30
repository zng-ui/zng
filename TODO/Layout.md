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
  - Child defines a max width possible for potential justify on first and last line.
    - To justify the parent then divides the extra space to cause the less size increase among children in the row.
* Flow direction, only flows left-to-right.
  - Controlled by the `LayoutMetrics::direction`.
  - Same as align, parent panel defines first and last row rect, children defines mid-rows.

### New API

```rust
/// Info about the inline rows of the widget.
pub struct InlineInfo {
  /// Maximum fill width possible on the first row.
  pub first_row_max_fill: Px,
  /// Maximum fill width possible on the last row.
  pub last_row_max_fill: Px,
  /// Last layout rows of the widget.
  pub rows: Vec<PxRect>,
}

/// Constrains for inline layout in the parent.
/// 
/// These constrains complement the normal layout constrains and layout direction. 
pub struct InlineConstrains {
  /// First row rect, defined by the parent.
  /// 
  /// If `None` the widget must define its own first row, aligned to the *start*.
  pub first_row: Option<PxRect>,
  /// Last row rect, defined by the parent.
  /// 
  /// If `None` the widget must define its owne last row, aligned to the *start*.
  pub last_row: Option<PxRect>,
}

impl LayoutMetrics {
  // close to the normal constrains.
  pub fn inline_constrains(&self) -> Option<InlineConstrains> { todo!() }
}
```

Steps for `wrap!` layout:

* Measure children with no inline constrains.
* Compute first and last row rect to match new `children_align`.
  - For `FILL` distribute the leftover space using the `first_row_max_fill`, `last_row_max_fill` to find the least
    width change for possible for each row segment.
* Layout children, now with `first_row` and `last_row` set.

Sets for `background` render:

* Get the `InlineInfo` from the widget bounds?
* Add the rows as clips.

Open questions:

* The row rectangles origin are in what space, the inner bounds?
  - The `WidgetLayout` needs to patch when if completes the inner bounds?
  - The two widgets that can be inlined, `text!` and `wrap!`, compute rows in the *child* space.
  - Actually, all properties that affect box disable inline currently, maybe we can enforce this.

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