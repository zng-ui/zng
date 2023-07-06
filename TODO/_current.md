* Docs error.
    - `TextInput` does not fetch all properties.
    - Tries to fetch `http://127.0.0.1:4000/zero_ui/widgets/text_input/struct.TextWrapMix.html`.
    - Is actually in `text/struct.TextWrapMix.html`.

# TextInput

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

* Spellchecker.
    - Use https://docs.rs/hunspell-rs
    - Used by Firefox and Chrome.

# Gradient

* Add cool examples of radial and conic gradients.

# Undo Service

* Test undo actions trying to register undo.

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
        - Can accept gradients as "image".
        - CSS does not support corner radius for this.
            - We could clip the border for the user.
    - Backdrop filter.
    - iFrame.
    - 3D transforms.
        - "transform-style".
            - Is flat by default?
            - If yes, we may not want to implement the other.
            - The user should use sibling widgets to preserve-3d.
        - rotate_x.
        - Perspective.
            - These are just matrix API and testing.
        - Backface vis.