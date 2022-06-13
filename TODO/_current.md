* Optimize/fix interactivity, can we make parent interactivity affecting child a guarantee?
* Different visual for focused and disabled.

=======

* Implement render optimization, see `Optimizations.md`.
* Implement `FocusRequest` window indicator + example.
* Add example of focus request that steals focus from other app.


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