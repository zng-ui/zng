# TextInput

* Implement selection.
    - Input replaces selection.
        - Char input, paste, IME
    - Impl cut & copy.

* Implement IME.
    - See https://github.com/rust-windowing/winit/issues/1497

# Text

* Implement text clip.
    - Ellipses, fade-out.
    - Very visible in icon example.

# View-Process

* Update to winit-29 when released.
    - Lots of breaking changes.
* Implement OpenGL example.
    - Overlay and texture image.
* Implement automation/screen reader APIs.

# Transform 3D

* Tested 3D hit-test in text for position of caret.

* Implement angle animation config property.
    - Animate 358 to 2 by going all the way around by default.
    - Config enables the shorter path instead.
* Fix filters.
    - Webrender disables 3D if any filter is used in a stacking-context.
    - We also can't just have a nested context just for the filters because this breaks the Preserve3D chain.
    - This issue is documented in the CSS specs.
    - CSS users work around this by coping effects onto the inner parts.

# WR Items
    - Touch events.
        - Use `Spacedesk` to generate touch events.
