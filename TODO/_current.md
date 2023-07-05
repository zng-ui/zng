# TextInput

* Make `TextInput!` an undo scope by default.
* Refactor all text edit actions into `TextEditOp` to support undo.

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

# Gradient

* Add cool examples of radial and conic gradients.

# Undo Service

* Clear.

* Custom `Text!` undo action.
    - No need to clone the entire text.
    - Track caret position?
        - Or at least set it when undoing an insert for example.

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