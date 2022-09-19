* Finish event rewrite.
    - UiNodeBoxed not needed anymore?
        - And AppExtensionBoxed?
        - AppEventObserverDyn?
    - Stop propagation when this is requested.
    - Stop propagation when all items in delivery list visited.
    - Auto doc of command metadata needs a proc-macro.
    - Subscription to scoped commands?

* Review Command unload, if we only modify a command meta and don't create any handlers it does not register for cleanup.
    - Bug already existed in previous implementation.
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
