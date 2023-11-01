# Winit Upgrade

* Wait for `accesskit` upgrade: https://github.com/AccessKit/accesskit/pull/256
* Merge.

# TextInput

* Implement `txt_selectable` text.
    - Should `txt_editable=false;txt_selectable=true` really be focusable? It currently is.
    - Set cut and copy enabled flag, they must be always subscribed (when editable/selectable) but only enabled when there is a selection.

* Touch selection.
    - Test with RTL and bidirectional text.
    - Set a flag that indicates caret or selection from touch.
    - Context menu appears when selecting (or just interacting, if it's an editable field)
    - Not a normal context menu, "floating toolbar"?
    - On monitor DPI `1.0`, the calculated blinking caret and touch caret positions are off by 1 pixel.

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

* `ACCESS_NUMBER_EVENT` and `ACCESS_INCREMENT_EVENT`.
    - Implement ParseInput, IntInput.

* All examples must be fully useable with a screen reader.
    - Test OS defaults and NVDA.

# DATA Notes

* Property to set error in an widget?
    - Say if `txt_parse` passes, but the data is invalid.
    - Same thing can be used for "what's new".
    - This a `var(vec![notes])` input?

* Implement `required`.
    - It must set error, but not from the start.
    - The initial value needs to display as an empty string?

* Define default colors for the 3 levels.
    - Similar to button `base_colors`.

* Implement error style for `TextInput!`.
    - Info and warning too.
    - Implement `get_top_notes` to only get error/warn/info.
* Implement "What's new" info indicator.
    - Blue dot on the top-end of widgets.
    - Can lead users on a trail into sub-menus, until the new one is shown.

* Implement `RESTORE_CMD` or `CANCEL_CMD` to set the text back to the current value.

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