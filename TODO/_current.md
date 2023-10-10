# TextInput

* Implement selection.
    - Touch:
        - Research how it works.

* Implement IME.
    - See https://github.com/rust-windowing/winit/issues/1497

# Accessibility

* Implement a way to detect access state updates.
    - Right now we rebuild the entire tree every time.
    - Also need to track what widget changed children.
    - Also need to update when transform changes during render.

* Integrate access commands, states and role.
    - Set role in more widgets.
    - Scrollbar values.
    - Review all states and commands.
    - Focusable (can only be sure after the widget info builds).

* Review default action, we only set click "verb", there are others.

# View-Process

* Update to winit-29 when released.
    - Lots of breaking changes.
* Next webrender breaks image masks.
