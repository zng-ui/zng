# TextInput

* Fix RTL text caret bugs.
    - `txt = var_from("الإعلان"); lang = lang!("ar");`.
    - Click place working correctly.
        - `nearest_char_index`.
        - Disabled ligature handling for RTL to avoid a panic.
* Support replace (Insert mode in command line).
* Support buttons:
    - page up and page down
* Remember caret x position when up/down and page up/down.
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

* Watermark text shows caret, it should not fore multiple reasons:
    - The txt property is not set to a read-write var.
    - Background widgets are not interactive.

* Ctrl+Shift+I when focusing TextInput inserts a tab and still opens the Inspector.
    - We are receiving a TAB for some reason, but we are stopping propagation.
    - Char event is not linked with key press event stop propagation does nothing.
        - Is a different event from Winit.
        - The next version of Winit will fix this: https://github.com/rust-windowing/winit/issues/753

# View-Process

* Implement custom event sender.
* Implement OpenGL example.
    - Overlay and texture image.

# Clipboard

* Merge.