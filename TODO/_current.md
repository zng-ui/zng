* Add other icon fonts to font example.
    - Add search.
* Review minimized render-update.
* Review Into/IntoVar of `T` for `Option<T>`

* Finish implementing window `parent`.
    - [x] Validation.
    - [x] Theme fallback.
    - [ ] Open center parent.
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