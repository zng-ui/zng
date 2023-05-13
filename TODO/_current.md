# Config

* Implement TOML, YAML and RON support (behind a feature).
    - https://docs.rs/toml
    - https://docs.rs/serde_yaml
    - https://docs.rs/ron
* Implement directory config.
    - Something to allow implementing localization dir.

* Implement serde for all property values (units, text, color, the multiple enums).

# Text Edit

* Enter inserts `\r` and not `\r\n`.
    - Detect hard-break chars with same algorithm we use during segmentation: https://unicode.org/reports/tr14/#NL
    - Have a substation property set to `\n`, Rust `println!` always uses `\n`.
    - Can have substitution closure property for any char?
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