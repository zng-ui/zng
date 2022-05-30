* Implement `Windows::focus` demo in window example.
* Implement initial focus request.
    - Use it on respawn.
* Implement `focus_indicator` var.
    - Is also init indicator.
* Implement `FocusRequest` new force and indicator configs.

* Layout context viewport.
    - The image example tries to do this manually, but there is some flickering due to Px rounding on the scroll vs on the offset.
    - CSS has the "position" property, that has sticky, webrender has something for this that we can use?
    - Current idea, have a `layout_parent = LayoutParent::Viewport`.

* Review render_update optimization, need to update children if parent transform changes.
* Implement render optimization, see `Optimizations.md`.
* Review layout double-pass of stacks.
    - What happens for nested stacks, quadratic?
* Fix text final size, either clip or return accurate size.

* Scrolling, see `Scrolling.md`.
* Animation, see `Variables.md`.