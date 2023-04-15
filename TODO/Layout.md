# Layout TODO

* Track widget bounds `LayoutPassId`?

## Inline Align::FILL

* Support `Justify`, enabled by `Align::FILL`.
* Implement fill/justify in `Wrap!`.
  - Use segments API to control spacing all from the parent?
    - Remove `first_max_fill` and `last_max_fill`.

## Direction

* Expand `LayoutDirection` to support vertical text, and to control the direction *rows* are added.
  - See CSS `writing-mode`, lang define direction glyphs flow, but also direction wrap grows.
  - Or we can have specialized vertical text and wrap widgets.
    - Text and wrap code is already very complex, a separate widget may be more easy to maintain.
  - Does vertical text need to implement bidi sorting?
  - Can vertical and horizontal be mixed?
    - Yes, and we can have any widget in wrap so a `vertical_text` can be inserted as a block.
  - CSS has vertical text that is just rotated.
    - This can be done with something like `rotate_layout` that implements layout rotation in  90º increments 
      (swaps the constrain axis and renders a transform).
        - Need to set a context flag for properties like `cursor` to swap visual.
        - Need to clear inline info.
* Integrate `LayoutDirection` with all widgets.
  - grid, mirror grid indexes.

## Inline

* Review properties that disable inline, maybe they can support it with the new API?
  - First `foreground_highlight`, as it is used to draw the keyboard focus indicator in text links.

## Grid 

* Cell align.
* Column & Row align for when all fixed is less then available.
* Masonry align?
* Support `lft` in spacing.
        - And padding? Need to capture padding if the case.
* Add contextual grid info, like the text `TextLayout`, to allow custom properties to extent it (like an special border property).

## Single Child Panels

* Can maybe avoid some measure passes in panels with a single child.
  - Is it worth-it? Its a single extra measure and a special behavior that can mess-up tests.
  - See what other frameworks do.