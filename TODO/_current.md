* Implement dynamic when states, see `Themes.md`.
* Review dynamic property set in the widget declaration and set again in instance.
* Review dynamic widget that captures set property.

* Finish implementing window `parent`.
    - [x] Theme fallback.
    - [x] Open center parent.
    - [x] Children list var.
    - [x] Validation.
    - [x] Close together.
    - [x] Minimize/restore together.
    - [ ] Z-order, always on-top of parent, but no focus stealing.
* Implement `modal`.
    - [ ] Steal focus back to modal.
    - [ ] Window level "interactivity", parent window must not receive any event (other than forced close).

* Review light theme in all examples.
* Implement `WindowThemeVar::map_match<T>(dark: T, light: T) -> impl Var<T>`.

# Text

* Text Editable
    - Caret.
    - Selection.
* `text_input!`.
    - Inherit from `text!`.
    - Appearance of a text-box.
* IME.
* `LineBreakVar`.
    - When char is `\n` or `\r` read this var and insert it instead. 
    - Review https://en.wikipedia.org/wiki/Newline

# Light Theme

- example    - desc
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

- text       - overline, underline and strikethrough default colors are not themed
             - colored text is hard to see in light theme
             - font size widget background is not themed

- transform  - red