* Repeatedly opening the markdown example sometimes shows blank spaces where text should be.
    - Happens more often if accessibility is enabled.
    - Blank spaces are the null char (0).
    - Font was `"<empty>"` when it failed, is shaping before font load and then never updating?
    - `"<empty>"` is on the list, not an empty list.
    - After fail all requests for the same font return empty font.
    - Font query never responds (after 5s).
    - Waiting a single font ResponseVar times-out, but the task itself of that font loading does not.
        - Bug in the response future?

* `StyleMix` does not capture `extend_style`/`replace_style` on the same widget, so it ends-up ignored. Need
  to promote this pattern.

# Documentation

* Add build dependencies for each operating system on the main `README.md`.
* Add `description`, `documentation`, `repository`, `readme`, `caregories`, `keywords`.
    - Review what other large crates do.
    - Review badges.
* Add better documentation, examples, for all modules in the main crate.
* Review docs in the component crates.
    - They must link to the main crate on the start page.
    - Remove all examples using hidden macro_rules! hack.
        - Search for `///.*macro_rules! _`.
* Add `README.md` for each crate that mentions that it is a component of the project crate.
    - Use `#[doc = include_str!("README.md")]` in all crates.

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