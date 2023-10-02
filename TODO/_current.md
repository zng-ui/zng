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
    - Impl cut & copy.

* PageUp and PageDown should move the caret to either the start or end of the line if you're at the first or last line respectively
* Shift delete removes line.
* Ctrl delete removed word.

* Implement IME.
    - See https://github.com/rust-windowing/winit/issues/1497

# Accessibility

* Track focus update.

* Integrate access states.
    - Text sets label to the text.
    - Toggle sets checked.
    - Modal.
    - Review all states.

* Implement accessibility properties for each state?
    - We support building widgets on instantiation only.
    - Like using `Wgt!` with custom properties to form a new widget.
    - For these widgets the properties are useful.

* Implement way to enabled accessibility from code.
    - Some programmatic service may be interested in these values too.

# View-Process

* Update to winit-29 when released.
    - Lots of breaking changes.
* Next webrender breaks image masks.
