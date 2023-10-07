# TextInput

* Panic, open text, select, new empty, type.

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
    - Focusable (can only be sure after the widget info builds)

* Issues discovered testing with Windows Narrator:
    - Toggle button does not get label from child text like button.
        - Implement that by role instead of Click command support?
    - Review toggle button for other UIs, are they read as check-box?
    - Default action does nothing.
        - `on_click` sets the support for Click command.
        - Focus already works, so not a problem with all events from accessibility.

# View-Process

* Update to winit-29 when released.
    - Lots of breaking changes.
* Next webrender breaks image masks.
