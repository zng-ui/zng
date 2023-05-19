# Text Edit

* Implement cursor position.
    - Index is of insert offset, can be str.len for cursor after the last char.
    - Review using `TextPoint` for this?
    - Need to navigate with arrow keys.
        - Support `\r\n` in one key press.
    - Need to find closest insert point from mouse cursor point.
        - Support ligatures (click in middle works).
    - Review https://searchfox.org/mozilla-central/source/layout/generic/nsTextFrame.cpp#7534
        - ligated emoji sequence
        - all solved by grapheme clusters? https://unicode.org/reports/tr29/#Grapheme_Cluster_Boundaries
    - Grapheme Clusters crate: https://docs.rs/unicode-segmentation/latest/unicode_segmentation/trait.UnicodeSegmentation.html#tymethod.graphemes
    - "On a given system the backspace key might delete by code point, while the delete key may delete an entire clusters"
        - Observed this in Chrome, Firefox, VS and Word, use "ö̲" to test.
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

# Tooltip

* Implement something to only show one tooltip at a time.
    - Have an app_local that tracks the current tooltip ID and `layer_remove_cancellable`.
    - When another tooltip opens disable cancel and close the tooltip in the app_local.