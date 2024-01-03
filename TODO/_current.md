# Pointer Event Args

* Add helper functions for getting the position in the widget space.

# TextInput

* Interactive carets.
    - Test with multi-line.
    - Dragging caret over second causes both to start moving.
        - In bidi text this can panic also.
    - Use the caret spot to position.
        - Right now it only looks right because the default caret origin is at y=0.

* Implement selection toolbar.
    - Touch carets vanish when the toolbar opens.
        - Because of focus, needs to still show if focus is in toolbar.

* Opening a text file in the editor example causes an accesskit panic.

# Var

* WhenVar does not need to always be contextualized.
    - We already have a normal `build` and a `contextualized_build`.
    - Need a `build_mixed`?
        - Or need only `build` and it returns BoxedVar.
    - Some widget internals expected the Contextualized type.
        - Review this first, those codes could downcast to both build outputs?

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