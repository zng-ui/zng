# Layout TODO

* Can a stack panel be omni-directional controlled by a vector that defines origin of each
  subsequent item in the layout bounds of the previous item.
```rust
pub struct StackDirection {
  pub x: Length,
  pub y: Length,
}
impl StackDirection {
  pub fn horizontal() -> Self {
    Self {
      x: 100.pct().into(),
      y: 0.into()
    }
  }
    pub fn vertical() -> Self {
    Self {
      x: 0.into()
      y: 100.pct().into(),
    }
  }
}
```
- How does `FILL` work with this?

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