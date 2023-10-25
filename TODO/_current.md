# Winit Upgrade

* Wait for `winit` deadlock fix: https://github.com/rust-windowing/winit/pull/3172
* Wait for `accesskit` upgrade: https://github.com/AccessKit/accesskit/pull/256
* Merge.

# TextInput

* Implement `txt_selectable` text.
    - Should `txt_editable=false;txt_selectable=true` really be focusable? It currently is.
    - Set cut and copy enabled flag, they must be always subscribed (when editable/selectable) but only enabled when there is a selection.

* Touch selection.
    - Set a flag that indicates caret or selection from touch.
    - Context menu appears when selecting (or just interacting, if it's an editable field)
    - Not a normal context menu, "floating toolbar"?

* Configurable caret.
    - Implement `touch_caret` react to selection.
    - Property sets a `WidgetFn<CaretPosition>` that generates the caret node.
    - `enum CaretPosition { SelectionStart, SelectionEnd, Insert }`.
    - Have a different properties for normal caret and touch caret?
        - Can make one for touch only to begin with.
    - Caret node must configure some context data that sets the exact offset of the caret line?
        - They need to be positioned by the host node.
    - Touch carets must not be clipped by parent widgets.
        - Use `LAYERS`?
        - They still need to interact with the parent, if the node is in `LAYERS` it will not be in
          the text context. Capture context?
    - More expensive than current caret, but we will have two carets max so not a problem.

* Implement default context menu?

* Implement IME.
    - See https://github.com/rust-windowing/winit/issues/1497
    - Issue still open, but after winit update API is ready?

# Accessibility

* Implement more commands in `ACCESS`.
* `ACCESS_TOOLTIP_EVENT`.
    - Test it in icon example copy label.
* `ACCESS_SCROLL_EVENT`.
* `ACCESS_TEXT_EVENT` and `ACCESS_SELECTION_EVENT`.
* `ACCESS_NUMBER_EVENT` and `ACCESS_INCREMENT_EVENT`.
    - Implement ParseInput, IntInput.

* All examples must be fully useable with a screen reader.
    - Test OS defaults and NVDA.

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