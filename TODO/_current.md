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
    - https://github.com/AccessKit/accesskit
    - https://github.com/AccessKit/accesskit/blob/main/platforms/winit/examples/simple.rs
    - https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/ARIA_Techniques
    - This crate works, tested Windows winit.
    - Need to think on how we add all of the info and action events to the view-API.
        - `accesskit` (public?) in the `zero-ui-view-api` and `accesskit_winit` in the `zero-ui-view`.
        - Have the info as display-items in each frame?
            - Maybe quick to implement, but not efficient.
        - Access-kit info tree is very similar to our own.
            - There is info that does not change much.
            - There is position of nodes that change a lot.
    - Chrome only provides access info if it detects a screen reader running.
        - https://www.chromium.org/developers/accessibility/windows-accessibility/
        - Note: Chrome doesn't enable accessibility for the main web area by default unless it detects a screen reader or other advanced assistive technology. To override this, run chrome.exe with the --force-renderer-accessibility flag.