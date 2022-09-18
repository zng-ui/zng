* Review event/commands, can it be a static instance like context_var?
    - Specially for command this reduces the number of types by a lot, because it is all the same args type.
* Review `unsafe`, only use when there is no alternative.
* Implement all `todo!` code.

# Light Color Scheme

* Review light color scheme in all examples.

- example    - description
- *all*      - focus highlight is not changed (the border is the same color as the button in light mode).
- icon       - icons and icon cards are not changed
- layer      - Anchored and Layer (7,8,9) button overlays are not changed, the TOP_MOST overlay is not changed either
- respawn    - status background is not changed
- scroll     - background color and commands menu background color are not changed
- shortcuts  - shortcut text color is not changed

- text       - colored text is hard to see in light mode
             - font size widget background is not changed

- transform  - red
