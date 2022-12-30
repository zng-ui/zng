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
* Support baseline align in-between children of different first/last row length.
* Integrate, `wrap::children_align` with `txt_align`.

### Current API

Currently have `InlineLayout`, `WidgetMeasure::inline`, `WidgetLayout::inline` and `LayoutMetrics::inline_advance`.
Plus some methods to check if is inlining and disable inline in the widget.

Limitations:

* No detailed row info about the widget, background of wrapped text does not is not clipped correctly.
  - Have a `rows: Vec<PxRect>` in `InlineLayout`?  
* Rows cannot be aligned.
* Flow direction, only flows left-to-right.
* Baseline, only supports for the same height currently?
* Inline does not even have a `children_align`.

### New API

```rust
/// Info about the inline rows of the widget.
pub struct InlineInfo {
  /// Maximum fill width possible on the first row.
  pub first_max_fill: Px,
  /// Maximum fill width possible on the last row.
  pub last_max_fill: Px,

  /// Offset from the bottom of the first row, positive up, that is the baseline of the first item in the row.
  pub first_baseline: Px,
  /// Offset from the bottom of the last row, positive up, that is the baseline of the last item in the row.
  pub last_baseline: Px,

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
  pub first: Option<PxRect>,
  /// Last row rect, defined by the parent.
  /// 
  /// If `None` the widget must define its owne last row, aligned to the *start*.
  pub last: Option<PxRect>,
}

impl LayoutMetrics {
  /// Inline constrains, if the parent widget supports inline layout.
  pub fn inline_constrains(&self) -> Option<InlineConstrains> { todo!() }
}

impl WidgetLayout {
  // !!: review 
}
```

Steps for `wrap!` layout:

* Measure children with no inline constrains.
* Compute first and last row rect to match new `children_align`.
  - For `FILL` distribute the leftover space using the `first_max_fill`, `last_max_fill` to find the least
    width change for possible for each row segment.
  - For `BASELINE` find the baseline that does not *clip* any of the row segments, offset all to align with it.
* Layout children, now with `first` and `last` set.

Steps for `text!` or `wrap!` nested inside another `wrap!`:

* If the `InlineConstrains` are not set layout like a block, become the new inline root.
* If the `InlineConstrains` is set without the `first` and `last` rows:
  - During measure, measure and define the `rows` rectangles, ignore align on the first and last row (align left).
                    define a maximum the first and last row can fill (justify).
  - During layout, behave as if `InlineConstrains` is not set, this is an error.
* If the `InlineConstrains` is set with the `first` and `last` rows defined:
  - During measure, just copy the definitions to `rows` and measure the mid-rows.
  - During layout, apply the definitions, update `rows`.
    - If the defined `first` and `last` are wider then the measured width, apply `FILL` or `Justify` to these rows.
    - Apply the align defined on the widget, on the mid-rows.

Sets for `background` render:

* Get the `InlineInfo` from the widget bounds?
* Add the rows as clips.

### Final Questions

* The row rectangles origin are in what space, the inner bounds?
  - The `WidgetLayout` needs to patch when if completes the inner bounds?
  - The two widgets that can be inlined, `text!` and `wrap!`, compute rows in the *child* space.
  - Actually, all properties that affect box disable inline currently, maybe we can enforce this.
* Baseline align assumes that all children do the extra baseline transform that widgets do when they are blocks, is this ok?
  - Widgets that don't know inline just work.
  - Widget that do know inline can work with this.
  - Actually, the `children_align` of the parent does not apply on the child automatically, only for the first and last rows,
    so the child does not know the baseline.
* Inheritable `inline_align`.
  - We want to set align in the outer `wrap!` panel and it automatically applies to all nested inline widgets.
  - Can't we just make the `wrap::children_align` default be `TXT_ALIGN_VAR`?
  - This pattern can be used by new inline widgets too, the `TXT_ALIGN_VAR` is unifier.
    - Name and mod is tied to `text`, is there inline widget that does not include text runs?

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