* Context menu anchored on a `modal` widget is not interactive.
    - WidgetInfo needs a way to track the anchor widget, a general way.
    - Access already has this concept?

# TextInput

* Touch selection.
    - Test with RTL and bidirectional text.
    - Test with multi-line.

* Implement selection toolbar.
    - Like MS Word "Mini Toolbar" on selection and the text selection toolbar on mobile?
    - Implement `selection_toolbar_fn`.
        - Needs to open when a selection finishes creating (mouse/touch release)
            - And close with any interaction that closes POPUP + any mouse/touch/keyboard interaction with the Text widget.
                - Test touch.
        - If the toolbar is not focusable it does not close when changing focus to another app?
    - Have the selection_toolbar_fn args indicate what kind of event created the selection.
        - This way we can have the default only open for touch events, and users can have different toolbars
          without we needing to declare multiple properties.
    - Touch carets vanish when the toolbar opens.

# Accessibility

* All examples must be fully useable with a screen reader.
    - Test OS defaults and NVDA.

# Publish

* Publish if there is no missing component that could cause a core API refactor.

* Rename crates (replace zero-ui with something that has no hyphen). 
    - Z meaning depth opens some possibilities, unfortunately "zui" is already taken.
    - `znest`: Z-dimension (depth) + nest, Z-Nest, US pronunciation "zee nest"? 
    - `zerui`.
    - `nestui`.

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