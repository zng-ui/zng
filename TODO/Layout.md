# Layout TODO

* Can a stack panel be omni-directional controlled by a vector that defines origin of each
  subsequent item in the layout bounds of the previous item.
```rust
pub struct StackDirection {
  pub x: Length,
  pub y: Length,
}
impl StackDirection {
  /// h_stack
  pub fn horizontal() -> Self {
    Self {
      x: 100.pct().into(),
      y: 0.into()
    }
  }
  /// v_stack
  pub fn vertical() -> Self {
    Self {
      x: 0.into(),
      y: 100.pct().into(),
    }
  }

  /// z_stack
  pub fn z() -> Self {
    Self { 
      x: 0.into(),
      y: 0.into(),
    }
  }
}
```
- How does `FILL` work with this?
  - Same as currently, measure to position, then fill?
  - Only fill if fully locked on a dimension?
  - What about spacing? Just add to the direction?
  - Panel size is a bounding box (fill_or_exact, or clamped bounding box).

```text
children = ui_list![w!(0), w!(1), w!(2)];

direction = (1.fct(), 0);
layout:
|-------|-------|-------|
|   0   |   1   |   2   |
|-------|-------|-------|
0,0      1,0     2,0

direction = (1.fct(), 0.5.fct());
layout:
|-------|
|   0   |-------|
|-------|   1   |------|
0,0     |-------|  2   |
         1,0.5  |------|
                 1,1

direction = (0.5.fct(), 0.5.fct());
layout:
|-------|
|   0|-------|
|----|   1|------|
     |----|  2   |
          |------|

direction = (0, 1.fct());
layout
|-------|
|   0   | 0,0
|-------|
|   1   | 0,1
|-------|
|   2   | 0,2
|-------|

direction = (-1.fct(), 0);
layout:
|-------|-------|-------|
|   2   |   1   |   0   |
|-------|-------|-------|
-2,0     -1,0    0,0
```

* Integrate `LayoutDirection` with all widgets.
  - h_stack::children_align
  - v_stack::children_align
  - wrap
  - grid

## Inline Align

* Right now we can't align the `wrap!` rows to the right.
* This is a limitation of the current inline layout API, it can't work, 
  we need the full row width to compute the align offset.
* Can this be done with a measure pass?

## Grid 

* Cell align.
* Column & Row align for when all fixed is less then available.
* Masonry align?
* Support `lft` in spacing.
        - And padding? Need to capture padding if the case.
* Add contextual grid info, like the text `TextLayout`, to allow custom properties to extent it (like an special border property).