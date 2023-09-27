# TextInput

* Implement selection.
    - Text edit/text change must use selection.
        - Delete selection.
    - Arrow keys must use selection.
        - Does not move caret, moves from the end-point in the direction.
    - Mouse:
        - Double-click causes selection that grows by word.
            - Conflict with normal press drag?
            - Prevent double-click from selecting the line break segment?
            - When a selection already exists, double-clicking to the left of it is incorrect
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

* Implement info full build and send.
    - Skip nodes without any accessibility info?
        - Review what HTML elements are included.
    - Optimize updates.
* Access states from existing info:
    - AccessState::Modal - Derived from interactivity.
    - AccessState::ActiveDescendant - Derived from focused (we just use the normal focus nav for these widgets).
    - AccessState::FlowTo - Derived from tab index.
* How to integrate with focus service without depending on it?
    - Have focus service set a focusable flag on the tree?
    - The API has a special value for just the focused.
* Integrate access states.
    - Text sets label to the text.
    - Toggle sets checked.
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