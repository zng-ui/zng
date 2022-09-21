* Try linear search in delivery list.
    - Merge.
* Review event docs.
* Update webrender.
    - https://github.com/servo/webrender/commit/244a0ff74b57aa64b3760445ea6f71fb856dbe45
* Implement delivery-list/subscribers for variables.
* Implement delivery-list for raw update requests.
* Remove UiNode::subscriptions.
* Stop propagation when all items in delivery list visited.
* Stop propagation when it is requested.

* Review `unsafe`, only use when there is no alternative.

* Review Command unload, if we only modify a command meta and don't create any handlers it does not register for cleanup.
    - Bug already existed in previous implementation.
    - Have an AppId?
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
