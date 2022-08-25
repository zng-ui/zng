* Refactor view to use only use clip chains, this is a requirement of the new webrender version.
* Review minimized render-update.

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