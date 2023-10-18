# TextInput

* Implement `txt_selectable` text.
    - Test `txt_editable=false;txt_selectable=true`, is edit blocked?
    - Test `txt_editable=true;txt_selectable=false`, cannot create selection?
    - Set cut and copy enabled flag, they must be always subscribed (when editable/selectable) but only enabled when there is a selection.
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

* Integrate access commands, states and role.
    - Review all states and commands.
    - Set role in more widgets.
    - Scroll-to.
    - Focusable (can only be sure after the widget info builds).

* All examples must be fully useable with a screen reader.
    - Test OS defaults and NVDA.

# Drag & Drop

* Drag/move inside window.
    - Integrate with `touch_transform`.
* Drag and drop across apps with visual feedback.
    - Wait for winit 29.
    - Visual can be a screen capture of the widget by default.
    - Browsers do this, with some fade-out mask effect and text selection clipping.

# View-Process

* Update to winit-29 when released.
    - Lots of breaking changes.

# Publish

* Publish if there is no missing component that could cause a core API refactor.

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