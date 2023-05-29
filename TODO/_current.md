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
* Implement image copy&paste in image example.

# Localization

* Merge.
* Review wait, how do handles work when request is not found yet?
* Test live update of localization file.