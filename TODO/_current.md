* Changing system theme while minimized causes panic:
```log
thread 'main' panicked at 'expected `LayoutText` in `render_underlines`', zero-ui\src\widgets\text_wgt\nodes.rs:675:47
``` 

* Implement dynamic when states, see `Themes.md`.
* Review dynamic property set in the widget declaration and set again in instance.
* Review dynamic widget that captures set property.

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

* Review light theme in all examples.
* Implement `WindowThemeVar::map_match<T>(dark: T, light: T) -> impl Var<T>`.

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