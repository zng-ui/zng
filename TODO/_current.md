* `ContextualizedVar` can get very large.
    - `FONT_PALETTE_VAR` for example, is mapped from `COLOR_SCHEME_VAR` but otherwise not set.
       In inspector screen with many text widgets it can grow to thousands of "actual" values, all for
       the same mapped var.
    - The `DIRECTION_VAR` is mapped from `LANG_VAR` same issue.
    - Figure-out a way to have the `ContextualizedVar` only invalidate if the source variables actually change context.

* Test `scroll_to` with nested scrolls.
* Sending multiple `scroll_to` commands causes weird behavior?
    - Holding `CTRL+LEFT/RIGHT` in a `TextInput!` with many lines will sometimes cause it to scroll multiple lines at once.

# TextInput

* Scroll to caret not fully scrolling 2 lines when scrolling downwards?

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
