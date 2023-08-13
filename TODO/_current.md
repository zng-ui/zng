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

* Implement perspective update in render_update.
    - We are applying perspective in update.
    - Test if changes to it cause a full render or just render_update.
    - If just render_update it needs to be compared.
    - Also need to update the render info for the widgets?
* Fix visible.
    - Cube example not considered visible if 90º rotated.
* Fix filters.
    - Webrender disables 3D if any filter is used in a stacking-context.
    - We also can't just have a nested context just for the filters because this breaks the Preserve3D chain.
    - This issue is documented in the CSS specs.
    - CSS users work around this by coping effects onto the inner parts.

* backface_visible, sets webrender `PrimitiveFlags::IS_BACKFACE_VISIBLE`.
    - Flag can be set in any primitive, figure out why?
    - Can we just have a context push?
    - For now is enabled for all display items.

# WR Items

* Finish items implemented by webrender.
    - Perspective and backface stuff.

    - Touch events.
        - Use `Spacedesk` to generate touch events.

* Review `RasterSpace::Screen` usage.
    - Firefox controls this (only Screen if stacking-context is not animating).
    - https://searchfox.org/mozilla-central/source/layout/painting/nsDisplayList.cpp#6675
