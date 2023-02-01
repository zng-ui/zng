# Layout TODO

## Inline Align::FILL

* Support `Justify`, enabled by `Align::FILL`.
* Implement fill/justify in `wrap!`.
  - Panel can also add spacing? Maybe maximum row height of spacing, if it helps complete the row.

## Direction

* Expand `LayoutDirection` to support vertical text, and to control the direction *rows* are added.
  - See CSS `writing-mode`, lang define direction glyphs flow, but also direction wrap grows.
* Integrate `LayoutDirection` with all widgets.
  - grid, mirror grid indexes.

## Inline

* Fix `inline` property, to force widgets to use inline visual even when not in inline context.
  - Need a way to enable it in `WidgetMeasure` and `WidgetLayout`.
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