# Config

- RON backend does not create config file.
- Implement status to replace `is_loading` and allow impl `wait_idle`.
    - Loading.
    - Saving.
    - Idle.

# Text Edit

* Implement cursor position.
    - Index is of insert offset, can be str.len for cursor after the last char.
    - Review using `TextPoint` for this?
    - Need to navigate with arrow keys.
        - Support `\r\n` in one key press.
    - Need to find closest insert point from mouse cursor point.
        - Support ligatures (click in middle works).
    - Review https://searchfox.org/mozilla-central/source/layout/generic/nsTextFrame.cpp#7534
        - Surrogate pairs: https://learn.microsoft.com/en-us/globalization/encoding/surrogate-pairs
        - ligated emoji sequence
* Support replace (Insert mode in command line).
* Implement selection.
* Implement custom node access to text.

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

* Implement something to only show one tooltip at a time.
* Fix `layer_remove_delay` not receiving `LAYER_REMOVE_REQUESTED_EVENT` after reinit.