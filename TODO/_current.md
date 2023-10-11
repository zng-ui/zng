# TextInput

* Implement read-only text.
    - `read_only` property and read-only variable.
    - Selectable text is `editable` true and `read_only` true?
* Implement selection.
    - Touch:
        - Long-press to start selecting text:
            - Selects a word initially (if there is text)
                - Shows draggable "cursors" to extend or shrink selection
            - Context menu appears when selecting (or just interacting, if it's an editable field)
                - Show a draggable "cursor" when interacting with editable fields, to move the insertion point
* Implement default context menu?

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
