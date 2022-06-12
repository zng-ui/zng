* Optimize/fix interactivity, can we make parent interactivity affecting child a guarantee?
* Different visual for focused and disabled.

=======

* Implement layout and render optimization, see `Optimizations.md`.

* Implement `FocusRequest` new force and indicator configs.
    - And demo in focus example.
        - Focus shortcuts are perfect for this.
        - Also need an example of focus stealing.
        - Unify `WindowFocusChangedEvent` to include the previous and new focused window.

* Gestures info, like last shortcut pressed, primed chord, list of shortcuts used.
* Implement window close cancel when OS is shutting down.
    - Apparently this is implemented with the winapi function `ShutdownBlockReasonCreate`

* Layout context viewport.
    - The image example tries to do this manually, but there is some flickering due to Px rounding on the scroll vs on the offset.
    - CSS has the "position" property, that has sticky, webrender has something for this that we can use?
    - Current idea, have a `layout_parent = LayoutParent::Viewport`.

* Fix text final size, either clip or return accurate size.

* Implement reverse `UiNodeList`.

* Scrolling, see `Scrolling.md`.
* Animation, see `Variables.md`.