# `do` TODO

* Implement a command that automatically bump crate versions.
    - Try `cargo-semver-checks` to automatically detect breaking changes.
* Implement a command that publishes to `crates.io`.
    - Something simple.
    - Find-out dependency tree (already needed for the auto version bump feature).
    - Generate a list of crates with new versions compared with crates.io.
    - Publish each crate with a generous delay.
        - Maybe verify that each crate is available before moving on to the next.
    - Remove `do release`.