# Publish

* Set all versions to `0.2`, set versions in dependencies.
    - Can `cargo release` do this automatically?

* Pick license and code of conduct.
    - https://rust-lang.github.io/api-guidelines/necessities.html#crate-and-its-dependencies-have-a-permissive-license-c-permissive
    - winit only uses apache like us, so we are good?

* Use `cargo release` to publish.
    - Need to exclude examples and tests crates.
    - Set `PUBLISH_GRACE_SLEEP=61` to avoid crates.io limits.
        - See https://github.com/crate-ci/cargo-release/issues/726
    - Actually news creates are limited to one every 10 minutes.
        - That's 12 hours or more.
    - Bump versions, set versions for each dependency.
    Together with webrender it should take less then 2 hours to publish all.

* How will the `zng-l10n-scraper` be published?
    - Need to be compatible with `cargo install`.

* Publish zng-webrender and dependencies first.
    - Change dependencies to use the published zng-webrender.
* Make project public.
* Publish project.

* Review prebuild distribution.

# After Publish

* Create issues for each TODO.
* Announce in social media.