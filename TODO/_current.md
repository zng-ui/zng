# TextInput

* Test disabled TextInput.

* Touch selection.
    - Test with RTL and bidirectional text.
    - Context menu appears when selecting (or just interacting, if it's an editable field)
    - Not a normal context menu, "floating toolbar"?
    - Make `TOUCH_CARET_OFFSET` public and document that it needs to be set in layout.

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

* Implement IME.
    - See https://github.com/rust-windowing/winit/issues/1497
    - Issue still open, but after winit update API is ready?

* Change selection color for dark mode.

# Accessibility

* All examples must be fully useable with a screen reader.
    - Test OS defaults and NVDA.

# Inspector

* Focus/scroll to selected.
* Live info, auto update after every info rebuild.
* Add "computed-values/properties" section.
* Flash widget that has property change.
* Test inspecting two windows at the same time.
* Implement widget search.

# Publish

* Publish if there is no missing component that could cause a core API refactor.

* Rename crates (replace zero-ui with something that has no hyphen).
    - `nestor`: All our nodes and properties are nesting constructors. (name already taken)
    - `ctorx`: Constructor/Context.
    - `xctor`: Context/Constructor.
    - `xnest`: Context nesting.
    - `nestx`: Nesting context.
    - `nestc`: Nesting constructor. 
    - `nestcx`, `cxnest`. 
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