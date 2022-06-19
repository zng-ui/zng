* Layout context viewport.
    - The image example tries to do this manually, but there is some flickering due to Px rounding on the scroll vs on the offset.
    - CSS has the "position" property, that has sticky, webrender has something for this that we can use?
    - Current idea, have a `layout_parent = LayoutParent::Viewport`.

* Scrolling, see `Scrolling.md`.
* Animation, see `Variables.md`.


# Focus notes(icon example):
* Focus moving to scrollable when using left and right arrows in the middle row
* Focus moving twice when cycling from one of the icons

* The priority of keyboard focus should be high when highlighting and low when not?

* `move_focus` changes highlight to true when scrolling with arrow keys