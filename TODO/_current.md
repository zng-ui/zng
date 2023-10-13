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
* Touch, set a flag that indicates caret or selection from touch.

* Implement default context menu?

* Implement IME.
    - See https://github.com/rust-windowing/winit/issues/1497

# Accessibility

* Implement a way to detect access state updates.
    - Right now we rebuild the entire tree every time.
* Update when transform & visibility changes during render.

* Integrate access commands, states and role.
    - Set role in more widgets.
    - Scrollbar values.
    - Review all states and commands.
    - Focusable (can only be sure after the widget info builds).

* Language not changing for text (in Windows Narrator at least).
    - Firefox sets "Culture" in UI automation interface.
    - Our own apps do not.
        - We set the language, so AccessKit is not doing it.

* All examples must be fully useable with a screen reader.
    - Test OS defaults and NVDA.

# View-Process

* Update to winit-29 when released.
    - Lots of breaking changes.

# Publish

* Rename crates (replace zero-ui with something that has no hyphen).
    - `nestor`: All our nodes and properties are nesting constructors.
* Review all docs.
* Review prebuild distribution.
* Pick license and code of conduct.
* Create a GitHub user for the project?
* Create issues for each TODO.

* Publish (after all TODOs in this file resolved).
* Announce in social media.

* After publish only use pull requests.
    - We used a lot of partial commits during development.
    - Is that a problem in git history?
    - Research how other projects handled this issue.