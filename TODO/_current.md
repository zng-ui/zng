# TextInput

* Implement selection.
    - Touch:
        - Research how it works.
    - Draw selection for line break.

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
    - Button text child not used as a label.
        - Just says "button" (role?).
        - Inspect.exe shows "none" in label.
    - Check boxes read as "disabled button".
        - Need to set the role probably.
    - Default action does nothing.
        - `on_click` sets the support for Click command.
        - Focus already works, so not a problem with all events from accessibility.

# View-Process

* Update to winit-29 when released.
    - Lots of breaking changes.
* Next webrender breaks image masks.
