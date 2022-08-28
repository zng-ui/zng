* Add other icon fonts to font example.
    - Add search.
* Fix icon docs, font not loading for demos (not implemented?)

* Finish implementing window `parent`.
    - [x] Theme fallback.
    - [x] Open center parent.
    - [x] Children list var.
    - [x] Validation.
    - [x] Close together.
    - [x] Minimize/restore together.
    - [ ] Z-order, always on-top of parent, but no focus stealing.
* Implement `modal`.
    - [ ] Steal focus back to modal.
    - [ ] Window level "interactivity", parent window must not receive any event (other than forced close).

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