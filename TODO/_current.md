# TextInput

* Touch selection.
    - Test with RTL and bidirectional text.
    - Implement `touch_carets` touch drag.
        - Implement in the layered shape?
        - Hit-test area full shape rectangle.

* Implement selection toolbar.
    - Like MS Word "Mini Toolbar" on selection and the text selection toolbar on mobile?
    - Has to be anchored in relation to the selected text.
    - Implement `selection_toolbar_fn`.
        - Should it be a context-var?
            - Context-menu is not.
            - Flutter has a SelectableText widget.
            - Maybe we can have one, with a style property and DefaultStyle.
                - It sets the context_menu and selection_toolbar.
        - Needs to open when a selection finishes creating (mouse/touch release)
            - And close with any interaction that closes POPUP + any mouse/touch/keyboard interaction with the Text widget.
```rust
TextInput! {
    txt = var_from("select text to show toolbar");
    text::selection_toolbar = Wgt! {
        size = 40;
        background_color = colors::GREEN.with_alpha(50.pct());
    }
}
```

* Implement IME.
    - Implement pre-edit preview (!!: TODO IME).
    - Popup window covers entire widget (area size ignored in Windows?)
        - Temporary fix implemented for Windows in `set_ime_area`, need to debug this.
    - Review IME area, we can't just use bounds, a large text area widget may want to
      set the IME area as the current line.
    - Firefox uses the selection rectangles and also sets the "interest point".
        - Winit sets the point to the exclusion area origin.

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