* Text Editable
    - Caret.
    - Selection.
* `text_input!`.
    - Inherit from `text!`.
    - Appearance of a text-box.
* IME.
* `LineBreakVar`.
    - When char is `\n` or `\r` read this var and insert it instead. 
    - Review https://en.wikipedia.org/wiki/Newline
* Make `ImageSource::render` use system theme.
* Button `light_theme` does not load when using `theme::pair`;
* Button `light_theme` does not work for pressed state.