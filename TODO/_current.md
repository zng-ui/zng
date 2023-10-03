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

* ArrowUp and ArrowDown should move the caret to either the start or end of the line if you're at the first or last line respectively.

* Implement IME.
    - See https://github.com/rust-windowing/winit/issues/1497

# Accessibility

* Implement transforms and size.

* Implement available commands.
    - Do we need to split scroll commands?
        - Firefox does not announce scroll?
    - `ShowContextMenu` cannot always be announced.
        - `AccessCommandName` is not a valid enum for selecting actions?
        - Or we need to split the click.
    - Technically `accesskit::Action::Blur` is not always available when focus is, ESC can return focus to same place.

* Implement or remove `SetNextTabStart`.

* Figure units for `accesskit::ActionData::ScrollTargetRect` and others.

* Implement a way to detect access state updates.
    - Right now we rebuild the entire tree every time.
    - Also need to track what widget changed children.

* Integrate access states and role.
    - Roles.
    - Scrollbar values.
    - Review all states.

* Implement way to enabled accessibility from code.
    - Some programmatic service may be interested in these values too.

# View-Process

* Update to winit-29 when released.
    - Lots of breaking changes.
* Next webrender breaks image masks.
