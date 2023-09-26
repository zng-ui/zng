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
            - When a selection already exists, double-clicking to the left of it is incorrect
        - Triple-click and Quadruple-click?
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

# View-Process

* Update to winit-29 when released.
    - Lots of breaking changes.

* Implement automation/screen reader APIs.
    - Add access info to info tree.
        - Some are cheap, like role, are there any that are excessive?
            - Even role is one byte per node, or much more if we use the meta dictionary.
            - On the other hand, if we only enable access on demand and use meta we don't add any cost.
            - Property name is `access_role`, all properties with `access` prefix.
            - Only accept other metadata if the `access_role` is set?
                - Firefox has role=generic in some entries in the access tree.
        - Track invalidations?
        - We only send changes to view-process.
    - Some state like
    - Implement access API info from info tree.
        - Figure out units, transforms.
    - Review !!: TODO
    - Implement way to enabled accessibility from code.
        - Some programmatic service may be interested in these values too.