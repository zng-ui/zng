# TextInput

* Touch selection.
    - Test with RTL and bidirectional text.
    - Implement `touch_carets` touch drag.
        - Implement in the layered shape?
        - Hit-test area full shape rectangle.

* Implement selection toolbar.
    - Like MS Word "Mini Toolbar" on selection and the text selection toolbar on mobile?
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

# Accessibility

*  panicked at 'assertion failed: self.nodes.contains_key(&self.focus)'
    - Run icon example, search z, click a button.
* All examples must be fully useable with a screen reader.
    - Test OS defaults and NVDA.

# Split Core

* LAYOUT and units that layout.
    - Steps before merge:
        - Refactor each issue in core first.
        - Use refactored code copy in layout crate.
        - Replace types in core, re-exported at the same place so no breaking change happens.
        - Merge.

    - Depends on widget stuff, helpers only?
        - Yes, only widget measure and layout stuff, including inline constraints.
        - Replace with LAYOUT extension methods?
        - Inline stuff part of constraints snapshot.

    - LayoutDirection type has conversions with Unicode and Harfbuzz crates.
        - Only used internally refactor to helper functions.
    - Align::layout requires WidgetLayout.
        - For translation and underline flag only.
        - Refactor to return translation and flag.

* Each app extension.
    - They are mostly contained.
    - Needs `AppExtension` and update types.
    - We need a `zero-ui-app-api` and zero-ui-app pair?
    - They need events, like ConfigManager needs LOW_MEMORY_EVENT.

* Length types in layout crate.
    - Needs LAYOUT context? Otherwise can't implement Layout2d and Layout1d.
    - Needs var too, for impl_into_var.

## Split Main

* Per widget?

# Publish

* Publish if there is no missing component that could cause a core API refactor.

* Rename crates (replace zero-ui with something that has no hyphen). 
    - Z meaning depth opens some possibilities, unfortunately "zui" is already taken.
    - `znest`: Z-dimension (depth) + nest, Z-Nest, US pronunciation "zee nest"? 

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