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

# Widget Macro Panic

* Panic if we try to use a property declared in the widget without value in the when condition.
    - If property is only declared in the when condition it is auto declared in the widget with default value,
      but the assertion panic fires if it is declared and without a default value.
    - Promote normal declarations to auto-declarations with default value?