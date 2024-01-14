* Repeatedly opening the markdown example sometimes shows blank spaces where text should be.
    - Blank spaces are the null char (0). Font was `"<empty>"` when it failed, is shaping before font load and then never updating?
    - Waiting a single font ResponseVar times-out, but the task itself of that font loading does not.
        - Bug in the response future?
        - Yes, replacing `wait_into_rsp` loop probing and sleeping *fixes* the bug.
        - Need an actual fix.
    - `WaitUpdateFut` timeout, hook never called.
        - Having trouble tracing modify (bug vanishes if there are prints).
```
ok:

!!: modify Arc(0x23dd0339870)
!!: apply modify Arc(0x23dd0339870)
!!: modify Arc(0x23dd0339ab0)
!!: apply modify Arc(0x23dd0339ab0)
!!: modify Arc(0x23dd0338eb0)
!!: apply modify Arc(0x23dd0338eb0)
!!: modify Arc(0x23dd0339630)
!!: apply modify Arc(0x23dd0339630)

timeout:

!!: modify Arc(0x23dfa6a1da0)
!!: apply modify Arc(0x23dfa6a1da0)
!!: modify Arc(0x23dfa6a2340)
!!: modify Arc(0x23dfa6a2520)
!!: apply modify Arc(0x23dfa6a2340)
!!: apply modify Arc(0x23dfa6a2520)
!!: TIMEOUT Arc(0x23dfa6a2520)
```

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