# Text Edit

* Implement `accepts_return`.
* Implement cursor position.
* Implement selection.

# WATCHER

* Test the `WATCHER` service.
    - Event not received until the app window is interacted with.
        - Print in the `notify` handle shows an imediate response.
        - Issue is the timer not elapsing.
        - Issue is that the `TIMERS.on_*` does not wake the app so the new timer is never used.
            - Fixed, review other `TIMERS` methods and other services.
        - The `notify` handle also does not run in the app context?
* Use the new service in `CONFIG`.

# Localization

* Implement resource loader.
    - Can we use `CONFIG` as backing store?
    - Need a "directory-db" mode, a common pattern for apps is having each locale in a diferent file in the same dir.
    - Review file watcher impl, may need to use a crate that uses the system API now.
* Implement builder.
* Move `Lang` and lang related stuff to `l10n` module.
* Add variable args in example.
* Test "// l10n-source: test.$lang.flt" comments.

* Other macros:
    - `l10n_txt!("id", "fmt")`, is scrapped and expands to `l10n!("id", "fmt").get()`.
    - `l10n_str!("id", "fmt")`, is scrapped and expands to `l10n!("id", "fmt").get().to_string()` or equivalent.