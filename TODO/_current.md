# TextInput

* Add an emoji font.
    - See `./Text.md`
    - '🙎🏻‍♀️'

* Implement cursor position.
    - Need to find closest insert point from mouse cursor point.
        - Support ligatures (click in middle works).
* Support replace (Insert mode in command line).
* Support buttons:
    - up and down arrows
    - page up and page down
* Implement selection.
    - Input replaces selection.
        - Char input, paste, IME
    - Impl cut & copy.
* Research text editors.

* Implement custom node access to text.
    - Clone text var in `ResolvedText`?
    - Getter property `get_transformed_text`, to get the text after whitespace transforms?

* Implement IME.
    - See https://github.com/rust-windowing/winit/issues/1497

* Watermark text shows caret, it should not fore multiple reasons:
    - The txt property is not set to a read-write var.
    - Background widgets are not interactive.

* Ctrl+Shift+I when focusing TextInput inserts a tab and still opens the Inspector.
    - We are receiving a TAB for some reason, but we are stopping propagation.
    - Char event is not linked with key press event stop propagation does nothing.
        - Is a different event from Winit.
        - The next version of Winit will fix this: https://github.com/rust-windowing/winit/issues/753

# View!

* `view` really needs to be a widget.
    - In the icon example the view is not centered in the stack because
      stack align does not work with non-widget nodes.

# WINDOW_CTRL

* Refactor into an extension trait.
    - Is more discoverable as an extension trait, maybe suggested by tooling (rustc, ra)?

# View-Process

* Test async dialogs in Windows.
    - Use it if they are modal.
    - Async can continue animations.
* Implement custom event sender.
* Implement OpenGL example.
    - Overlay and texture image.

# Tooltip

* `tooltip` -> `disable_tooltip` do not swap when widget is disabled while the tooltip is visible.

# is_hovered

* `is_hovered` does not update back to `true` when the hovered widget is enabled.

# Image Paste

* Image paste some pixel columns swapped (wrap around start).
    - Some corrupted pixels, probably same reason.
* Screenshot paste does not have scale-factor.