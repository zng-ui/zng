# Text

* Font ligatures with more then one word like "address-book" don't activate the ligature because
we split the text during segmentation.
    - Ligatures are implemented as substitution from glyphs to glyphs.
    - See https://docs.rs/ttf-parser/0.20.0/ttf_parser/gsub/index.html
    - There is no way to get a list of "keywords" that can be used during segmentation.
        - We don't want that anyway, we don't want to depend on the top font for segmentation.
    - We could try matching compound words during shaping.
        - How to detect if a ligature was applied?
    - Implemented detection of ligatures in fonts and in features during text shaping.
        - Use this to go a different path that joins words for a try first.

# `zero-ui`
* Include view crate in the main crate behind features (one for prebuilt, one for just `zero-ui-view`).

# Documentation

* Add build dependencies for each operating system on the main `README.md`.
* Add `description`, `documentation`, `repository`, `readme`, `categories`, `keywords`.
    - Review what other large crates do.
    - Review badges.
* Add better documentation, examples, for all modules in the main crate.
* Review docs in the component crates.
    - They must link to the main crate on the start page.
    - Remove all examples using hidden macro_rules! hack.
        - Search for `///.*macro_rules! _`.

# Publish

* Publish if there is no missing component that could cause a core API refactor.

* Rename crates (replace zero-ui with something that has no hyphen). 
    - Z meaning depth opens some possibilities, unfortunately "zui" is already taken.
    - `znest`: Z-dimension (depth) + nest, Z-Nest, US pronunciation "zee nest"? 
    - `zerui`.
    - `nestui`.

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