# Publish

* Pick license and code of conduct.
    - https://rust-lang.github.io/api-guidelines/necessities.html#crate-and-its-dependencies-have-a-permissive-license-c-permissive
    - winit only uses apache like us, so we are good?

* Use `cargo release` to review everything.

* Publish zng-webrender and dependencies first.
    - Change dependencies to use the published zng-webrender.
* Make project public.
* Publish project.

* Review prebuild distribution.

# After Publish

* Create issues for each TODO.
* Announce in social media.