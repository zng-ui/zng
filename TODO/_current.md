* Image example background does not render.
* Add ImageSource args to equality.
    - Otherwise if the same source is used in two different windows the cached image for the first window is shared,
        and parents can have different scale_factor and theme.
* Add other icon fonts to font example.
    - Add search.
* Window example exit button closes the wrong window when there is more than one open.
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