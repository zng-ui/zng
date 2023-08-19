# TextInput

* Implement selection.
    - Input replaces selection.
        - Char input, paste, IME
    - Impl cut & copy.

* Implement IME.
    - See https://github.com/rust-windowing/winit/issues/1497

# Text

* `txt_overflow`.
    - Overflow size correction.
        - Layout size already correct.
        - Render size/align needs to be corrected?
    - Interaction with txt_wrap?
        - Could fit part of the word wrapped into overflow.
    - Append suffix.
        - Need to find last segment from overflow.
        - Use direction of it?
    - Test RTL.
    - Not enabled when editable?
    - `get_overflow`, gets OverflowInfo.
    - `is_overflow`.
    - `get_overflow_txt`, gets the text starting at overflow.

# Tooltip

* Tooltips stop showing upon interaction (click/tab/enter/etc.) in HTML.
    - Ours doesn't.

# View-Process

* Update to winit-29 when released.
    - Lots of breaking changes.
* Implement OpenGL example.
    - Overlay and texture image.
* Implement automation/screen reader APIs.

# WR Items
    - Touch events.
        - Use `Spacedesk` to generate touch events.
