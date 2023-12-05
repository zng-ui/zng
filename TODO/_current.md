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
```rust
TextInput! {
    txt = var_from("select text to show toolbar");
    text::selection_toolbar = Wgt! {
        size = 40;
        background_color = colors::GREEN.with_alpha(50.pct());
    }
}
```

# Hit-test

* Root hit-test items outside the root inner are lost.

# Accessibility

*  panicked at 'assertion failed: self.nodes.contains_key(&self.focus)'
    - Run icon example, search z, click a button.
* All examples must be fully useable with a screen reader.
    - Test OS defaults and NVDA.

# Split Crates

* App crate.
    - Everything needed to create an App::minimal, spawn a view-process and render some nodes.
    - After it is building do not try to replace in zero-ui-core, just implement the extension
      crates too, replace core in main crate when done.

* Implement `TRANSFORM_CHANGED_EVENT` in app crate.
    - Implement `VISIBILITY_CHANGED_EVENT` in app crate.
        - This one is fully new, visibility was subscribing to FRAME_IMAGE_READY_EVENT.
        - Maybe both transform and visibility changed can be refactored into bulk event like INTERACTIVITY_CHANGED_EVENT.

* Config.
    - Needs app crate.
    - Needs var.

* L10n.
    - Needs app crate.
    - Needs var.

* Text Shaping.
    - Mostly decoupled, needs l10n's Lang.
        - Can use underlying type LanguageIdentifier?
        - Yes, lets not depend on app crate, this should be useful as standalone.
            - Also only text widget crate needs to depend on it.
        - Needs to integrate with `app::render::Font`.

* Focus.
    - Needs app crate.
    - Needs WINDOWS.

* File-System Watcher
    - Needs app crate.
    - Needs timers.

* Gesture
    - Needs app crate.
    - Needs WINDOWS, keyboard and mouse.

* Image
    - Needs app crate.
    - Needs ViewImage.

* Keyboard, mouse and touch.
    - Needs app crate.
    - Needs FOCUS.
    - Needs TIMERS.

* Pointer Capture.
    - Needs mouse and touch.
    - Needs WINDOWS.

* Undo.
    - Needs app crate.

* WINDOWS.
    - Needs app crate.
    - Needs ViewWindow.
    - Needs UiNode.
    - Needs Image for icon?

* Move widget events to wgt crate.
    - Implement `on_show`, `on_collapse`.

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