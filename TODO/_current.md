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

* Fix filters.
    - Webrender disables 3D if any filter is used in a stacking-context.
    - We also can't just have a nested context just for the filters because this breaks the Preserve3D chain.
    - This issue is documented in the CSS specs.
    - CSS users work around this by coping effects onto the inner parts.

# WR Items

* Finish items implemented by webrender.
    - Perspective and backface stuff.

    - Touch events.
        - Use `Spacedesk` to generate touch events.

* Review `RasterSpace::Screen` usage.
    - Firefox controls this (only Screen if stacking-context is not animating).
    - https://searchfox.org/mozilla-central/source/layout/painting/nsDisplayList.cpp#6675
