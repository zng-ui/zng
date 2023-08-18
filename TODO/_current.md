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
    - Track overflow point, that is, a char that starts overflowing.
        - Can use this char to implement fade-out.
        - Can use it to implement continuation in another text, bound `get_overflow_index` of the first
          text as the starting index of the second text.

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
