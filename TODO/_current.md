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
    - Currently implemented in notify_transform_changes.
        - This tracks all subscriber transforms in a map.
        - Need to move this map to the info tree?
    - Implement `VISIBILITY_CHANGED_EVENT` in app crate.
        - This one is new, visibility was subscribing to FRAME_IMAGE_READY_EVENT.
        - Use the same method of storing subscriber data.
        - Maybe both transform and visibility changed can be refactored into bulk event like INTERACTIVITY_CHANGED_EVENT.

* Multiple crates need shortcuts only to set on commands.
    - Including -app actually, we removed the shortcuts from EXIT_CMD.
    - Unfortunately this pulls the entire input and focus stuff.
    - Maybe we can have shortcut type in app crate.
        - Shortcut is demonstration of command extension, so have it on a crate?
        - Can't set on the EXIT_CMD if use crate, because crate will depend on the app for command traits.

* Undo.
    - Needs focus scope.
        - For `undo_scoped`, really needs it.
    - Needs `KEYBOARD` for a config.
        - If we unify focus with input this is already a dependency too.

* Focus.
    - Needs mouse and touch events.
    - Needs WINDOWS.
    - Needs gestures and keyboard for shortcut.
* Mouse.
    - Needs ModifiersState for keyboard.
    - Needs capture.
* Keyboard.
    - Needs FOCUS.
* Gesture
    - Needs WINDOWS, keyboard and mouse.
* Pointer Capture.
    - Needs mouse and touch.
    - Needs WINDOWS.
* Focus and input must be on same crate?
    - Can decouple from WINDOWS?

* WINDOWS.
    - Needs shortcut for command shortcuts.
    - Needs LANG_VAR in L10n.
    - Needs FOCUS.focused for access and IME stuff.
        - Could create a WINDOWS.init_focused_widget(var).
    - Needs FONTS.system_font_aa.
        - Could listen to the RAW event and record (same thing FONTS does).

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