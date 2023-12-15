# TextInput

* Touch selection.
    - Test with RTL and bidirectional text.
    - Implement `touch_carets` touch drag.
        - Implement in the layered shape?
        - Hit-test area full shape rectangle.

* Implement selection toolbar.
    - Like MS Word "Mini Toolbar" on selection and the text selection toolbar on mobile?
    - Implement `selection_toolbar_fn`.
        - Needs to open when a selection finishes creating (mouse/touch release)
            - And close with any interaction that closes POPUP + any mouse/touch/keyboard interaction with the Text widget.
                - Test touch.
```rust
TextInput! {
    txt = var_from("select text to show toolbar");
    text::selection_toolbar = Wgt! {
        size = 40;
        background_color = colors::GREEN.with_alpha(50.pct());
    }
}
```

* Issue:
```
ERROR zero_ui_app::running: updated 1000 times without rendering, probably stuck in an infinite loop
will start skipping updates to render and poll system events
top 20 most frequent update requests (in 500 cycles):
WindowManager//WindowId(1) update var of type Factor (250 times)
WindowManager//WindowId(1) update (250 times)
```
    - Observed multiple times, after a slow build when the VS window is not focused.
    - Is not scale_factor vars, added debug panics in scale_factor vars of window and monitor, did not panic.
    - Observed in 'button' example, when it opened VSCode was visible in the background of Firefox and not focused,
      build time was 2m, when focus was moved the the example window it had not rendered fonts yet.

# Hit-test

* Root hit-test items outside the root inner are lost.

# Accessibility

*  panicked at 'assertion failed: self.nodes.contains_key(&self.focus)'
    - Run icon example, search z, click a button.
* All examples must be fully useable with a screen reader.
    - Test OS defaults and NVDA.

# Split Crates

* Review prelude.
    - Include more properties, `enabled`, `visibility`?

# Publish

* Publish if there is no missing component that could cause a core API refactor.

* Rename crates (replace zero-ui with something that has no hyphen). 
    - Z meaning depth opens some possibilities, unfortunately "zui" is already taken.
    - `znest`: Z-dimension (depth) + nest, Z-Nest, US pronunciation "zee nest"? 
    - `zerui`.

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