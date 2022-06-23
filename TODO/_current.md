* Events for disabled commands.
    - We allow "disabled interactions" for other events.
    - This fixes the bug of arrow keys scrolling to end, then moving the focus.
    - This breaks the trick of setting "Page Down" to scroll vertical and horizontal so that when only horizontal is enabled it scrolls down.
        - Does it fully breaks it? Will still work if we don't register the vertical command if the scroll mode does not allows it.
        - Page down scrolling to the right after down is a weird effect anyway, because page up does not move to the left first then up.

* Layout context viewport.
    - The image example tries to do this manually, but there is some flickering due to Px rounding on the scroll vs on the offset.
    - CSS has the "position" property, that has sticky, webrender has something for this that we can use?
    - Current idea, have a `layout_parent = LayoutParent::Viewport`.

* Focus, see `Focus.md`.
* Scrolling, see `Scrolling.md`.
* Animation, see `Variables.md`.
