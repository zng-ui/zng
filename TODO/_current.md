* Headless window not updating?
    - It is, just not the theme..

* Finish implementing window `parent`.
    - [x] Validation.
    - [x] Theme fallback.
    - [ ] Close together.
    - [ ] Minimize together.
    - [ ] Z-order, always on-top of parent, but no focus stealing.

# Text

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