* Fix focus highlight in the detached button.
* Fix focus_shortcut only working when focus is already inside widget.
* Fix text reuse in button example when multiple buttons are added.
* Optimize/fix interactivity, can we make parent interactivity affecting child a guaranteed?

=======

* Implement layout and render optimization, see `Optimizations.md`.

* Implement `FocusRequest` new force and indicator configs.
    - And demo in focus example.
        - Focus shortcuts are perfect for this.

* Layout context viewport.
    - The image example tries to do this manually, but there is some flickering due to Px rounding on the scroll vs on the offset.
    - CSS has the "position" property, that has sticky, webrender has something for this that we can use?
    - Current idea, have a `layout_parent = LayoutParent::Viewport`.

* Review layout double-pass of stacks.
    - What happens for nested stacks, quadratic?
* Fix text final size, either clip or return accurate size.

* Scrolling, see `Scrolling.md`.
* Animation, see `Variables.md`.