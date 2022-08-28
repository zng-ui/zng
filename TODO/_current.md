* Implemented extra priority sorting to fix `toggle::tristate` and other contextual properties.
    - See `Proc-Macros.md`
* Implement dynamic when states, see `Themes.md`.
* Review dynamic property set in the widget declaration and set again in instance.
* Review dynamic widget that captures set property.

* Fix `focus_on_init`.
    - Window is not focused on open so the focus does not move?
    - Maybe we can move the focus_return of the window to the new focus request in cases like this?


* Profile icon set change in debug compilation.
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