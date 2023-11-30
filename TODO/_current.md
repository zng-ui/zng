* Review all #[cfg(dyn_closure)].

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

* All examples must be fully useable with a screen reader.
    - Test OS defaults and NVDA.

# Split Core

* Each app extension.
    - They are mostly contained.
    - Needs `AppExtension` and update types.
    - We need a `zero-ui-app-api` and zero-ui-app pair?
    - They need events, like ConfigManager needs LOW_MEMORY_EVENT.

* Var crate.
    - Replace code var, except:
        - VarSubscribe.
        - state module.
        - easing attribute macro.
        - property_build_action.
        - context::helpers.
    - Implement strong type hook.
        - Review all hook usages.
    - Implement specialized maps now that it is GATs.

* Length types in units crate.
    - Needs impl_from_and_into_var.
        - Will depend on vars crate, that depends on the Txt crate.
        - We are suddenly importing a lot of dependencies.
        - Should be pretty small still, or we could make a "var-api" crate.
    - Needs LAYOUT context? Otherwise can't implement Layout2d and Layout1d.

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