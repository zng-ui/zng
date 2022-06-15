* Optimize/fix interactivity, can we make parent interactivity affecting child a guarantee?

* Gestures info, like last shortcut pressed, primed chord, list of shortcuts used.

* Layout context viewport.
    - The image example tries to do this manually, but there is some flickering due to Px rounding on the scroll vs on the offset.
    - CSS has the "position" property, that has sticky, webrender has something for this that we can use?
    - Current idea, have a `layout_parent = LayoutParent::Viewport`.

* Implement text align demo in text example.
* Implement text start/end aligns.
* Implement text clip.

* Implement reverse `UiNodeList`.

* Scrolling, see `Scrolling.md`.
* Animation, see `Variables.md`.