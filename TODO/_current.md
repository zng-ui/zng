# TextInput

* Implement selection.
    - Mouse:
        - Double-click causes selection that grows by word.
            - Conflict with normal press drag?
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

# Exclusive Video Mode

* Entering exclusive mode of different VideMode in the secondary screen causes the
  monitors to refresh (`MONITORS_CHANGED_EVENT`), this causes the monitor query to
  run again and select the primary monitor for the now exclusive window, this causes
  the scale_factor of the primary monitor to be used.