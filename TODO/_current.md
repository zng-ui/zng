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

* Implement angle animation config property.
    - Animate 358 to 2 by going all the way around by default.
    - Config enables the shorter path instead.
    - This is called "slerp".
        - https://en.wikipedia.org/wiki/Slerp
    - Euclid has Rotation3D type that is a quaternion and can slerp.
        - https://docs.rs/euclid/latest/euclid/struct.Rotation3D.html#method.slerp

# WR Items
    - Touch events.
        - Use `Spacedesk` to generate touch events.
