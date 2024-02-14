# Documentation

* Add `description`, `documentation`, `repository`, `readme`, `categories`, `keywords`.
    - Review what other large crates do.
    - Review badges.
* Review docs.
    - Do a full read, look for typos or failed links.
        - Last reviewing `zero_ui::stack`.

# Publish

* Publish if there is no missing component that could cause a core API refactor.

* Rename crates (replace zero-ui with something that has no hyphen). 
    - Z meaning depth opens some possibilities, unfortunately "zui" is already taken.
    - `znest`: Z-dimension (depth) + nest, Z-Nest, US pronunciation "zee nest"? 
    - `nst`: Short for nest (how is pronunciation?)
    - `nstui`: Nest + UI.
    - `znst`: Z + Nest.
    - `zng`: Z + Nest + Graphics (pronunciation: zing).
    - `zngui`: Z + Nest + GUI (pronunciation: zing UI).

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