These issues where already in master before view-process refactor.
* Opening two instances of config example causes errors.
* Delay open second window (see focus example).
* Scroll up with keyboard does not work after scroll down (see scroll example).

# Text Edit

* Implement `accepts_return`.
* Implement cursor position.
* Implement selection.

# Localization

* Implement resource loader.
    - Can we use `CONFIG` as backing store?
    - Need a "directory-db" mode, a common pattern for apps is having each locale in a different file in the same dir.
    - Review file watcher impl, may need to use a crate that uses the system API now.
* Implement builder.
* Implement pseudo-localization test mode.
* Move `Lang` and lang related stuff to `l10n` module.
* Add variable args in example.
* Test "// l10n-source: test.$lang.flt" comments.

* Other macros:
    - `l10n_txt!("id", "fmt")`, is scrapped and expands to `l10n!("id", "fmt").get()`.
    - `l10n_str!("id", "fmt")`, is scrapped and expands to `l10n!("id", "fmt").get().to_string()` or equivalent.

# Other

* Review external crate re-exports.
    - We re-export `rayon` and `parking_lot`, are we the only ones to do this?
    - Users are expected to import the crate even if our API surface types form it?
* Implement something to only show one tooltip at a time.
* Fix `layer_remove_delay` not receiving `LAYER_REMOVE_REQUESTED_EVENT` after reinit.