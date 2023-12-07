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
        - Maybe restyle `ContextMenu!` to be the `default_selection_toolbar`?
```rust
TextInput! {
    txt = var_from("select text to show toolbar");
    text::selection_toolbar = Wgt! {
        size = 40;
        background_color = colors::GREEN.with_alpha(50.pct());
    }
}
```

* Issue, maybe caused by opening without interaction (minimized?)
```
ERROR zero_ui_core::app: updated 1000 times without rendering, probably stuck in an infinite loop
will start skipping updates to render and poll system events
top 20 most frequent update requests (in 500 cycles):
WindowManager//WindowId(1) update (250 times)
WindowManager//WindowId(1) update var of type zero_ui_units::factor::Factor (250 times)
```

# Hit-test

* Root hit-test items outside the root inner are lost.

# Accessibility

*  panicked at 'assertion failed: self.nodes.contains_key(&self.focus)'
    - Run icon example, search z, click a button.
* All examples must be fully useable with a screen reader.
    - Test OS defaults and NVDA.

# Split Crates

* Move widget events to wgt crate.
    - Implement `on_show`, `on_collapse`.

* Split main crate into widget crates.
    - What about properties?

* Add `WINDOWS.register_root_extender` on the default app?
    - `FONTS.system_font_aa`.
    - color scheme.
    - `lang = LANG_VAR`, for accessibility.
```rust
// removed from core
with_context_var_init(a.root, COLOR_SCHEME_VAR, || WINDOW.vars().actual_color_scheme().boxed()).boxed()
```

* Replace a main crate with a declaration of the default app and manually selected re-exports,
  most users should be able to create apps, custom widgets for these apps by simply depending
  on this crate. The re-export must be manual so that some stuff that is public does not get re-exported,
  things like the view_api `WindowId`, or the `ViewWindow`.

* Delete old core and main crate.
* Test everything.
* Merge.

* Refactor transform and visibility changed events to only send one event per frame like INTERACTIVITY_CHANGED_EVENT.
    - Test what happens when info is rebuild,.

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