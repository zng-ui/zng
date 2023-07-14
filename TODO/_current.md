* Review foreground_color in popup.

# TextInput

* Large single word does not wrap (it wraps for a bit then it becomes a single line again).
* Support replace (Insert mode in command line).
* Implement scroll integration:
    - scroll to caret
* Implement selection.
    - Input replaces selection.
        - Char input, paste, IME
    - Impl cut & copy.
* Implement `CTRL+Left/Right` advance caret by word.
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
* Getter property `get_transformed_txt`, to get the text after whitespace & transforms?
    - Transformed should be in the SegmentedText already.
    - Whitespace needs processing.

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

# Menu

* Implement styleable stack `Menu!`.
    - Background, drop-shadow, automatic set `Toggle!` style inside to a menu loop.
    - Nested menus, auto-close on click.
    - Arrow key open close.
* Implement `toggle::MenuStyle!`.
* Implement `toggle::ListItemStyle!`.
    - Use it in the combo-box example.

# Undo Service

* `UndoHistory!` widget.
    - Property to select the stack?
    - Need to support pop-up hover down selecting.
        - Need a `HOVERED_TIMESTAMP_VAR`?
        - To highlight the covered entries.
        - Reuse the `toggle::ListItemStyle!` looks.

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
    - Touch events.
        - Use `Spacedesk` to generate touch events.