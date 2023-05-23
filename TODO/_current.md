# TextInput

* Implement cursor position.
    - Caret stops blinking while moving with cursor, not resetting timer?
    - Index is of insert offset, can be str.len for cursor after the last char.
    - Review using `TextPoint` for this?
    - Need to navigate with arrow keys.
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

# Clipboard

* Implement clipboard commands.
    - Already declared in the main crate (move to core).

# Localization

* Implement pseudo-localization test mode.
    - Extendable source, instead of `load_dir`.
* A trait that provides the available locales and locales on demand.
    - Implement in-memory source.
* Locale filter.
    - `lang!("template") is valid, we maybe want to filter these?
    - Or at least document this.

* Test "// l10n-source: test.$lang.flt" comments.

* Review default fluent functions.
    - Some are missing?
* Review fallback in bundle.
    - Bundles support multiple resource overrides, resources can be shared with `Arc` too.
    - If a resource message references another that is missing, does setting-up these aggregate bundles causes
      it to resolve the missing reference on a fallback?
 
* Optimize.
    - `format_fallback` does multiple allocations just to get inputs for the formatter.
    - It is possible to implement something that only allocates the result string?
    - Every message refreshes every update.
