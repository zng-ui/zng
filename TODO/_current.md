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

# Touch Events

* Context menu opens at the mouse cursor position on long press.
    - It also does not close on tap out.
    - Issue is probably in `LAYERS`.

* We want the same gesture in combo box of clicking and dragging to the option.
    - Maybe use the swipe/fling gesture?

* Swipe to dismiss.
    - Widget is moved with transform, when touch is released the widget animates back into place or
      out if the threshold for closing was crossed.

* Implement "Ripple!" visual.
    - Radial background overlay animation fired on every touch or click.

* Improve `touch_transform`.
    - Contextual origin.
    - Config persistence.

# Exclusive Video Mode

* How is the scale factor selected for exclusive?
* Settings to a smaller size (800x600) causes background clipping.
* Also test in non-primary monitor.