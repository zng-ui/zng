We emit interaction events (except focus) without checking if interaction is enabled for the widget, the default
`concerns_widget` then checked the interaction for each widget, the idea was to allows the creation of "disabled hovered"
states, like a tooltip the says disabled?

The interaction filter did not exist as a general thing when we made this, now if we send disabled events we may accidentally
allow interaction with "really not-interactable" widgets, such as those blocked by a modal overlay.

Either we stop emitting disabled interactions or we expand the interaction filter to have levels of blockage.

The args that removed default interaction filter where:

* Review click args and allow_interaction, removed check from args.
* Review mouse move capture, removed check from args.
* Review mouse input args, same removes.
* Review mouse click args.
* Review mouse hover args.
* Review mouse cap args.

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