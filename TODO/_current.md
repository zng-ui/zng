# TextInput
* Implement selection.
    - Text edit/text change must use selection.
        - Delete selection.
        - Replace selection when typing starts.
    - Arrow keys must use selection.
        - Does not move caret, moves from the end-point in the direction.
    - Mouse:
        - Double-click causes selection that grows by word.
            - Conflict with normal press drag?
            - Prevent double-click from selecting the line break segment?
        - Triple-click and Quadruple-click?
    - Touch:
        - Research how it works.
    - Clear selection on typing and clicking (when not holding shift).
    - Draw selection for line break.
    - Input replaces selection.
        - Char input, paste, IME
    - Impl cut & copy.
    - Commands.
        - Select All (CTRL+a).
        - Must not scroll to caret in this one.

* PageUp and PageDown should move the caret to either the start or end of the line if you're at the first or last line respectively

* Implement IME.
    - See https://github.com/rust-windowing/winit/issues/1497

# View-Process

* Update to winit-29 when released.
    - Lots of breaking changes.

* Implement automation/screen reader APIs.
    - Implement `accesskit` from API info.
    - Implement some flag for "access_enabled".
        - Handle `ACCESS_INIT_EVENT`.
    - Implement access API info from info tree.