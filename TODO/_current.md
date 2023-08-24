# TextInput

* Implement selection.
    - Input replaces selection.
        - Char input, paste, IME
    - Impl cut & copy.

* Implement IME.
    - See https://github.com/rust-windowing/winit/issues/1497

# Text

* `txt_overflow`.
    - Test RTL.
        - Broken, current approach does not work.
        - Bidi text can make all sorts of changes.
        - We basically need to check every glyph to see if it is in the overflow region.
        - Can't even use a clip?
    - Test emoticon suffix.
    - Not enabled when editable?

# View-Process

* Update to winit-29 when released.
    - Lots of breaking changes.
* Implement OpenGL example.
    - Overlay and texture image.
* Implement automation/screen reader APIs.

# WR Items
    - Touch events.
        - Use `Spacedesk` to generate touch events.
