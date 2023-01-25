# Layout TODO

## Inline Align::FILL

* Support `Justify`, enabled by `Align::FILL`.
* Implement fill/justify in `wrap!`.
  - Panel can also add spacing? Maybe maximum row height of spacing, if it helps complete the row.

## Min Constrains Reset

* Review PxConstrains::min in every panel, should be zero? 

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

## Review `WidgetLayout` Usage

The layout process can get complicated, and is easy to create subtle bugs in nodes that delegate to anything other
them a single child. Can improve the API to avoid these mistakes?

Review:

* How translate is targeted.
* When `with_branch` must be used.
* What happens if node that is not a full widget is inserted in a panel.
* What if we don't setup the widget outer bounds as a translate target when returning from `with_widget`?
  - Just target the child?

### Weird Nodes

Some nodes get inserted in panels that are not the standard widget setup, but are to useful to forbid:

* `is_state(wgt!(var), var)`: Self-contained bridge from `is_state` in the parent widget context to the `wgt!` context.
  - In a panel, the `UiNode::with_context` does not work, because `is_state` is a normal property,
    but the widget outer-target is still found and setup for panels to transform directly.
  - Worst, the `WidgetLayout::with_outer` does not work, even though the inner `wgt!` can be targeted if the node is just layout.
  - Panels must render transforms for nodes that are not full widgets?
    - Maybe there is something that can be done in the `UiNodeList` level, to just enable transforms for renders that need it.

* `flood`: and other painting nodes, can be layered in a z-stack to create complex visual without polluting the info-tree using only
  the memory needed to render it.
  - HTML/CSS can have this problem where many elements are added just to enable a CSS visual effect, we avoid the hit, but these nodes
    cannot be fully supported by panels, as they have no transform of their own.