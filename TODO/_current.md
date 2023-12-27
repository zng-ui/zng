# TextInput

* RTL selection background not rendered (selection is set).
    - Set `lang = lang!("he");`.
    - Set `txt = var_from("האלפבית העברי הנוכחי נמצא בשימוש");`.

* Touch selection.
    - Test with RTL and bidirectional text.
    - Test with multi-line.

* Implement selection toolbar.
    - Touch carets vanish when the toolbar opens.
        - Because of focus, needs to still show if focus is in toolbar.

# Color

* Color transition from default dark-mode hover to azure passes through a different color.

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