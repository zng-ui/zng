* Rename "theme" to "style", avoid confusion with `WindowTheme`.
    - The world "theme" implies a more encompassing thing.
    - Right now we can have multiple `theme!` instances per widget, the name `style!` is better.
    - Rename `themable!` to `stylable!`.
    - Rename `WindowTheme` to `Theme`.
    - Implement `theme` context property to change the `Theme` of parts of the screen.

* Implement all `todo!` code.

# Light Theme

* Review light theme in all examples.

- example    - description
- *all*      - focus highlight is not themed (the border is the same color as the button in light theme).
- icon       - icons and icon cards are not themed
- layer      - Anchored and Layer (7,8,9) button overlays are not themed, the TOP_MOST overlay is not themed either
- respawn    - status background is not themed
- scroll     - background color and commands menu background color are not themed
- shortcuts  - shortcut text color is not themed

- text       - colored text is hard to see in light theme
             - font size widget background is not themed

- transform  - red
