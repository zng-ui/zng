# Scroll Menu Issues

* Scroll shortcuts don't work after closing menu if it sets `modal`.
    - Focus returned to root, not scroll.
    - Two issues?
        - `cleanup_returns` wipes the scroll from return because it cannot be focused.
        - Window scope does not focus first focusable child.
    - Is same issue, `FocusScopeOnFocus::LastFocused` thinks it has last-focused, but
      the return was changed to just the scope..

# TextInput

* Caret not always positioned at end of wrapped line.
    - Include a line_index in the caret position.

* Large single word does not wrap (it wraps for a bit then it becomes a single line again).
* Support replace (Insert mode in command line).
* Implement scroll integration:
    - scroll to caret
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

- `UndoHistory!` widget.
    - Property to select the stack?
    - Need to support pop-up hover down selecting (See Word widget).
        - Need a `HOVERED_TIMESTAMP_VAR`?
        - To highlight the covered entries.

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