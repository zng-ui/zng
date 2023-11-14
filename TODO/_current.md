# TextInput

* Test disabled TextInput.

* Touch selection.
    - Test with RTL and bidirectional text.
    - Context menu appears when selecting (or just interacting, if it's an editable field)
        - Not a normal context menu, "floating toolbar"?
    - Implement `touch_carets` touch drag.
        - Implement in the layered shape?
        - Hit-test area full shape rectangle.

* Implement IME.
    - See https://github.com/rust-windowing/winit/issues/1497
    - Issue still open, but after winit update API is ready?

# Accessibility

* All examples must be fully useable with a screen reader.
    - Test OS defaults and NVDA.

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
    - `nidulus` or `nidula`: Small nest fungus name. +Fungus related like Rust, -Fungus disguised as a bird nest, not related with our
    nesting stuff.

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