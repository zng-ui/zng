* "Loading.." of panorama image vanishes if scroll to far right.
    - Its getting culled because culling is done using the outer-bounds, but `x` sets the inner-bounds.
    - Merge outer/inner into a single bounds, there used to be a TODO for this.
        - Don't remember why the TODO was removed, maybe because if implemented we need to enter every widget up-to inner
            to reuse?
    - Move culling to `push_inner` and use inner bounds, check performance in icon example.

# Light Theme

* Review light theme in all examples.

- example    - description
- border     - text color
- calculator - display number color is not themed
- config     - text input and update status are not themed
- countdown  - text color
- cursor     - theme not implemented in cursor areas
- focus      - command status, focus target and overlay scope are not themed
 - - nested  - theme also needs tweaking
- icon       - icons and icon cards are not themed
- image      - section title backgrounds, the checkerboard background and the `loading...` placeholder are not themed
- layer      - Anchored and Layer (7,8,9) button overlays are not themed, the TOP_MOST overlay is not themed either
- respawn    - status background is not themed
- scroll     - background color and commands menu background color are not themed
- shortcuts  - shortcut text color is not themed

- text       - colored text is hard to see in light theme
             - font size widget background is not themed

- transform  - red