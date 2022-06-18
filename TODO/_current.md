* Avoid filter work when widget interactivity already flags what it flags.
    - Current filters:
        - `enabled`: Disables all widgets in self and inside.
        - `interactive`: Blocks all widgets in self and inside.
        - `layer`: Copy interactivity of other to self.
            - All tree can be local to the widget.
        - `modal`: Blocks all widgets **not** inside self and parents.
            - Must be global.

* Layout context viewport.
    - The image example tries to do this manually, but there is some flickering due to Px rounding on the scroll vs on the offset.
    - CSS has the "position" property, that has sticky, webrender has something for this that we can use?
    - Current idea, have a `layout_parent = LayoutParent::Viewport`.

* Scrolling, see `Scrolling.md`.
* Animation, see `Variables.md`.