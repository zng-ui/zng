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
  /// Extra space in-between the first row and the mid-rows.
  /// 
  /// This is only valid if `first` is set, it *clears* the entire first row so that the mid-rows
  /// don't acidently overlap another larger segment. It must not be applied to the `last` row.
  pub mid_clear: Px,
  /// Last row rect, defined by the parent.
  /// 
  /// If `None` the widget must define its owne last row, aligned to the *start*.
  pub last: Option<PxRect>,
}

impl LayoutMetrics {
  /// Inline constrains, if the parent widget supports inline layout.
  /// 
  /// These constrains complement the [`constrains`] that define the total are the inline layout has. Inline
  /// widgets can ignore the `constrains.y`, if the parent is implemented correctly `y` is unbounded.
  /// 
  /// [`constrains`]: Self::constrains
  pub fn inline_constrains(&self) -> Option<InlineConstrains> { }
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
  - Vertical alignment applied to first and last rows of each child.
    - In the vertical space of the tallest segment on the row.
  - Also computes the  `mid_clear` for each child, so they can wrap properly.

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
* How is single row child handled?
  - This has caused bugs in the current API, and it is supposed to support it already.
  - What if `InlineInfo::rows` len defines how parent constrains are made?
    - This forces us to always have a measure pass.
  - The first and last stuff are equal.
  - We will have lots of these, every bold word or link in a markdown is one.
  - We can disable inline layout for it and just inline-block it.
    - Can we still support `BASELINE` in this case?
* Do we really need a measure pass for every Align?
  - `Align::START` only needs the previous child's last how width for the next.
  - Any other align, including `BASELINE` align needs measure.
    - Baseline will be very common.
    - If we figure out a way to support `BASELINE_START` without measure this is a perf win.
      - There is no way, we need every baseline to get the tallest row.
      - Actually we need to tallest row anyway right?
* How do we align vertically in the row?
  - Say we have a last row with twice the height as the next first row.
  - We apply vertical align to it, among row segments.
  - The next widget is positioned so that its mid rows clear the entire previous row.
  - This may cause the first row offset to be negative.
    - This causes backgrounds to get clipped incorrectly?
  - We need an extra constrain that defines the vertical offset of the mid-rows.
    - With this constrain we can position the child widget at the top of the row always?
    - Anyway, we can always avoid negative first rows.
* How is the child widget it self positioned?
  - Always start with the row and span the entire width.
  - Do we force the width or is the child that needs to consider the first/last row constrains?
  - What about widgets that are a single row?
    - Depends on how they are handled, we will probably just inline block it, or something similar.
* To `FILL` or justify can we insert spaces in the parent too?
  - Right now is just the children that handle this.
  - Maybe only if there is a small amount of space that left?
  - Maybe can use height the estimate what extra space we can get away with?

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