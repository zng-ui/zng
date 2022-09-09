# Light Theme

* Review light theme in all examples.

- example    - description
- countdown  - text color
- cursor     - theme not implemented in cursor areas
- focus      - command status, focus target and overlay scope are not themed
 - - nested  - theme also needs tweaking
- icon       - icons and icon cards are not themed
- layer      - Anchored and Layer (7,8,9) button overlays are not themed, the TOP_MOST overlay is not themed either
- respawn    - status background is not themed
- scroll     - background color and commands menu background color are not themed
- shortcuts  - shortcut text color is not themed

- text       - colored text is hard to see in light theme
             - font size widget background is not themed

- transform  - red

# ContextVar

* Use `From` in default value, this makes the same property values work for it:
```rust
static FOO_VAR: Size = (3, );
```

* Derived context vars, when it is not set directly the *parent* var is used:
```rust
static UNDERLINE_COLOR_VAR => TEXT_COLOR_VAR;
```
- Use the same type `ContextVar<TEXT_COLOR_VAR::T>`?