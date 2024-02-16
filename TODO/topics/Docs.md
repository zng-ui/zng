# Docs TODO

* Material icons don't show in docs.
    - Issue with `zero-ui-material-icons-extensions.html` not being included?
* Widget image/videos rendering from doc-tests.

* `WINDOW` service docs page shows the `Window` widget docs.
    - Same for `SCROLL` and `Scroll`.
    - https://github.com/rust-lang/rust/issues/25879
    - docs.rs builds in Linux so this should be fine there.
    - We could add a `#[cfg(all(windows, doc))] pub use WINDOW as WINDOW_` to review the docs in Windows.