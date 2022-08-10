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

# Toggle

* Button example exiting hover from the corner with the background `show_checked` sticks the hover state.
* Panic if we try to use `self.is_checked` with `is_checked` declared in the widget.