# Layout TODO

## Inline Align

### Inline Requirements

[x] Custom widgets can participate, not just text.
[x] Properties can have access to each row box in the widget (to clip background, or any effect like this).
[x] Rows can be aligned horizontally.
[x] Widgets can be block only, they are inlined as a block.
[x] Widgets that support inlining must also support block only layout.
[x] Flow direction, as defined by `LayoutDirection`.
[x] Integrate, `wrap::children_align` with `txt_align`.
[ ] Support baseline align in-between children of different first/last row height.
[ ] Support `Justify`, enabled by `Align::FILL`.

### Final TODOs

* Implement inline for `text!`.
  - Text shaping now needs to support external first and last row rectangle?
  - Implement fill/justify for mid-rows.
* Implement baseline in `wrap!`.
  - Review normal baseline first, maybe just align bottom for each row + baseline offset already works?
    - Can try in a horizontal stack first, two texts one larger, see if they already align.
* Implement fill/justify in `wrap!`.
  - Panel can also add spacing? Maybe maximum row height of spacing, if it helps complete the row.
* Review other widgets, they need to mark no-inline?
* Review `inline` property.
* Review `!!:`.
* Merge.
* Remove TODO.

## Min Constrains Reset

* Review PxConstrains::min in every panel, should be zero? 

## Direction

* Expand `LayoutDirection` to support vertical text, and to control the direction *rows* are added.
  - See CSS `writing-mode`, lang define direction glyphs flow, but also direction wrap grows.
* Integrate `LayoutDirection` with all widgets.
  - grid, mirror grid indexes.

## Inline

* Review properties that disable inline, maybe they can support it with the new API?

## Grid 

* Cell align.
* Column & Row align for when all fixed is less then available.
* Masonry align?
* Support `lft` in spacing.
        - And padding? Need to capture padding if the case.
* Add contextual grid info, like the text `TextLayout`, to allow custom properties to extent it (like an special border property).