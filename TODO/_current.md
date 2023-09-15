# TextInput

* Implement selection.
    - Mouse:
        - Press and drag selection.
        - Double-click causes selection that grows by word.
        - Triple-click and Quadruple-click?
    - Touch:
        - Research how it works.
    - Clear selection on typing and clicking (when not holding shift).
    - Draw selection for line break.
    - Input replaces selection.
        - Char input, paste, IME
    - Impl cut & copy.

* PageUp and PageDown should move the caret to either the start or end of the line if you're at the first or last line respectively

* Implement IME.
    - See https://github.com/rust-windowing/winit/issues/1497

# View-Process

* Update to winit-29 when released.
    - Lots of breaking changes.
* Implement automation/screen reader APIs.

# Scroll

* Implement touch scroll inertia.
    - Overflow from inertia.
* Implement test mode that generates touch events from mouse.
    - Setup for testing touch inertia is too slow.

# Exclusive Video Mode

* How is the scale factor selected for exclusive?
* Settings to a smaller size (800x600) causes background clipping.
* Also test in non-primary monitor.