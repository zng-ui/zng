* Scroll changing to start showing scrollbar causes accesskit panic.
    - Scrollbar inserted, but parent node with updated children list not included.
    - Scrollbar is present in info tree from the start, but it is collapsed.
        - Collapsed widgets are not send to accesskit.
    - Test setting actual Visibility does not cause panic.
        - Something about how the scrollbar is collapsed?

# TextInput

* on_click does not work in TextInput.
    - We stop propagation of mouse input?

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