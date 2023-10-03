# TextInput

* Implement selection.
    - Mouse:
        - Double-click causes selection that grows by word or.
        - Triple-click and Quadruple-click?
            - Also needs to grow selection by line and not change, respectively
    - Touch:
        - Research how it works.
    - Clear selection on typing and clicking (when not holding shift).
    - Draw selection for line break.
    - Input replaces selection.
        - Char input, paste, IME

* ArrowUp and ArrowDown should move the caret to either the start or end of the line if you're at the first or last line respectively.

* Implement IME.
    - See https://github.com/rust-windowing/winit/issues/1497

# Accessibility

* Implement or remove `SetNextTabStart`.

* Figure units for `accesskit::ActionData::ScrollTargetRect` and others.

* Implement a way to detect access state updates.
    - Right now we rebuild the entire tree every time.
    - Also need to track what widget changed children.
    - Also need to update when transform changes during render.

* Integrate access commands, states and role.
    - Set commands in `on_click` (can't use macro?)
    - Set role in more widgets.
    - Scrollbar values.
    - Review all states.

* Implement way to enabled accessibility from code.
    - Some programmatic service may be interested in these values too.

# View-Process

* Update to winit-29 when released.
    - Lots of breaking changes.
* Next webrender breaks image masks.
