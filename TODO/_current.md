* Review all state properties, must of then can be declared using the new `event_state` and `bind_state` helpers.

* Fix `focused_removed_by_deleting`, removed widget receiving event?
* Fix text reuse in button example when multiple buttons are added.
* Optimize/fix interactivity, can we make parent interactivity affecting child a guaranteed?

=======

* Implement layout and render optimization, see `Optimizations.md`.

* Implement `FocusRequest` new force and indicator configs.
    - And demo in focus example.

* Layout context viewport.
    - The image example tries to do this manually, but there is some flickering due to Px rounding on the scroll vs on the offset.
    - CSS has the "position" property, that has sticky, webrender has something for this that we can use?
    - Current idea, have a `layout_parent = LayoutParent::Viewport`.

* Review layout double-pass of stacks.
    - What happens for nested stacks, quadratic?
* Fix text final size, either clip or return accurate size.

* Scrolling, see `Scrolling.md`.
* Animation, see `Variables.md`.