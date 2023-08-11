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

* Need to invalidate children render?
    - Testing `transform_style` now.
        - Need to invalidate if parent changed only (same widget the property invalidates already).
        

* Perspective is computed on the parent.
    - Need to be tracked in the frame builder?
        - Yes, we don't have the position of the child yet in the parent.
        - The final child transform needs to be built in `push_inner`.
    - Can it be applied as a transform in the parent?
        - This way we don't need to invalidate the children somehow.
        - Preserve3D is not a problem for this, the parent is the new context.
* Perspective matrix:
```
1. Start with the identity matrix.

2. Translate by the computed X and Y values of perspective-origin

3. Multiply by the matrix that would be obtained from the perspective() transform function, 
   where the length is provided by the value of the perspective property

4. Translate by the negated computed X and Y values of perspective-origin

```

* backface_visible, sets webrender `PrimitiveFlags::IS_BACKFACE_VISIBLE`.
    - Flag can be set in any primitive, figure out why?
    - Can we just have a context push?

# WR Items

* Finish items implemented by webrender.
    - Backface vis.

    - Touch events.
        - Use `Spacedesk` to generate touch events.

* Review `RasterSpace::Screen` usage.
    - Firefox controls this (only Screen if stacking-context is not animating).
    - https://searchfox.org/mozilla-central/source/layout/painting/nsDisplayList.cpp#6675
