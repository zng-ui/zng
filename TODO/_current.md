* Prebuild installed DLLs are never removed.
    - Why do we install?
    - Place in temp?
        - Antivirus issue?
    - Can we load DLL from memory?

# TextInput

* Implement selection.
    - Input replaces selection.
        - Char input, paste, IME
    - Impl cut & copy.
        - VsCode copies the current line if there is no selection, how common is this?

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

* backface_visible, sets webrender `PrimitiveFlags::IS_BACKFACE_VISIBLE`.
    - Flag can be set in any primitive, figure out why?
    - Can we just have a context push?

* `rotate_3d`.

* translate_z shows no visual change (need perspective?)

* Contextual depth?
    - Right now we compute relative Z translate on the available-width.
* Contextual perspective?
    - CSS has a perspective function and a property of the same name.
    - The function is set in transform, the property is set on a parent widget.
    - Is it layout in the parent widget?
* https://drafts.csswg.org/css-transforms-2/#perspective


# WR Items

* Finish items implemented by webrender.
    - 3D transforms stuff.
        - Backface vis.

    - Touch events.
        - Use `Spacedesk` to generate touch events.

* Review `RasterSpace::Screen` usage.
    - Firefox controls this (only Screen if stacking-context is not animating).
    - https://searchfox.org/mozilla-central/source/layout/painting/nsDisplayList.cpp#6675
