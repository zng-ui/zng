* Scroll inside other scroll adds padding?
* Scroll should focus first child if possible.

# TextInput

* Implement scroll integration:
    - Scroll to caret.

* Implement selection.
    - Input replaces selection.
        - Char input, paste, IME
    - Impl cut & copy.
        - VsCode copies the current line if there is no selection, how common is this?
* Research text editors.

* Review `WhiteSpace::Merge`.
    - HTML `textarea` with `white-space: normal` does not show extra spaces, but it does add the spaces if typed.
* Review `TextTransformFn::Uppercase`.
    - Same behavior as whitespace, only a visual change.
    - See https://drafts.csswg.org/css-text/#text-transform
    - How does `TextTransformFn::Custom` event works?
        - CSS is always the same char length?
        - Maybe when editable transform the text by grapheme?
            - User may have a find/replace transform.
        - Custom needs to be a trait that maps caret points back to the source text.
    - Maybe don't allow `WhiteSpace` and `TextTransformFn` when the text is editable.
        - Apply it when the text is changed to not editable.

* Implement IME.
    - See https://github.com/rust-windowing/winit/issues/1497

* Ctrl+Shift+I when focusing TextInput inserts a tab and still opens the Inspector.
    - We are receiving a TAB for some reason, but we are stopping propagation.
    - Char event is not linked with key press event stop propagation does nothing.
        - Is a different event from Winit.
        - The next version of Winit will fix this: https://github.com/rust-windowing/winit/issues/753

# View-Process

* Update to winit-29 when released.
    - Lots of breaking changes.
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
    - Touch events.
        - Use `Spacedesk` to generate touch events.

# Bugs

* See `Variables.md` ContextualizedVar issue.
* Test scroll inside grid, not sized right.
    - Text editor example with scroll will demonstrate this?
