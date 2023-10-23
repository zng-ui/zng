# Winit Upgrade

* Wait for `winit` deadlock fix: https://github.com/rust-windowing/winit/pull/3172
* Wait for `accesskit` upgrade: https://github.com/AccessKit/accesskit/pull/256
* Fix all "!!:".
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

* Integrate access commands, states and role.
    - Review all states and commands.
    - `ScrollIntoView`.
        - Right now we set this in the viewport widget, it must be set in all children?

* All examples must be fully useable with a screen reader.
    - Test OS defaults and NVDA.

# Focus

* `navigation_origin`.
    - Issue: Focusable with single non-focusable child not fully centered.
        - Click on the child places origin in it, directional nav can find the parent as the closest.
        - Logical navigation does not have this problem.
            - Non-focusable is TabIndex::SKIP, and it is already inside the parent, so it exits to parent sibling.
            - What about prev_tab?
        - Need to implement something for directional nav queries.
            - Non-focusable directional to sibling focusable: OK.
            - Non-focusable directional to parent: SKIP.

* Scroll-to-focus happens when Scroll (that is a focus scope) receives focus and immediately transfers
  focus to first child.
  - Use `navigation_origin`, don't scroll if it is set?

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