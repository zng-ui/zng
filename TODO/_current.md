# TextInput

* Implement cursor position.
    - Caret stops blinking while moving with cursor, not resetting timer?
    - Caret animation does not start visible (on focus).

    - Review using `TextPoint` for this?
        - Remove `TextPoint`?

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

# Config

* `FallbackConfig` always inserts a copy on the write config, should stay bound to fallback.
    - We want the "config/fallback" to be used like the "workspace/user" settings of VSCode.
    - When the fallback changes and it is not overridden the config var should update.
    - When it is set, the config file only should update.
        - Like a `CowVar`.
        - But we still have the embedded ultimate fallback.