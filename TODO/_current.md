# Text Edit

* Enter inserts `\r` and not `\r\n`.
    - Detect hard-break chars with same algorithm we use during segmentation: https://unicode.org/reports/tr14/#NL
    - Have a substation property set to `\n`, Rust `println!` always uses `\n`.
    - Can have substitution closure property for any char?
* Implement cursor position.
* Implement selection.

# Localization

* Implement resource loader.
    - Resources need to automatically reload when file changes.
    - File name cannot be matched from the lang alone, request needs to map to a `PathBuf` and then 
      use this path to select a variable with loaded resources for the best lang match.
    - The file match needs to be shared between all variables that requested it.
    - The shared file match needs to be a var too, to leverage `SyncConfig`.
* Implement builder.
* Implement pseudo-localization test mode.
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