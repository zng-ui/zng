# TextInput

* What happens with undo if the text var is modified externally?

* Support replace (Insert mode in command line).
* Support buttons:
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

* Ctrl+Shift+I when focusing TextInput inserts a tab and still opens the Inspector.
    - We are receiving a TAB for some reason, but we are stopping propagation.
    - Char event is not linked with key press event stop propagation does nothing.
        - Is a different event from Winit.
        - The next version of Winit will fix this: https://github.com/rust-windowing/winit/issues/753

* "Zalgo" text causes glyph.x > line.max_x.
* Test clicks in:
1. ﷽

2. 𒐫

3. 𒈙

4. ⸻

5. ꧅

Number 1 followed by line break causes panic by keyboard and mouse.


# Gradient

* Add cool examples of radial and conic gradients.

# Undo Service

* Can still redo after new undo.
* Clear.

- UNDO_CMD.
    - Add undo list info to the command meta?
        - For use in "UndoStack" widgets.
    - Can undo/redo.

- `UndoStack!` widget.
    - Pop-up hover down selecting (See Word widget).

# View-Process

* Implement OpenGL example.
    - Overlay and texture image.

## WR Items

* Finish items implemented by webrender.
    - Nine-patch border.
    - Backdrop filter.
    - iFrame.
    - Review why there is a `push_shadow`?
        - Can already do shadow with filters.