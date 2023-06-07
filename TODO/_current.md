# TextInput

* If the text is 3 lines the caret renders in the third line when it's in the second line.
* Fix emoji segmenting, example: "🙎🏻‍♀️"
* Implement cursor position.
    - Review using `TextPoint` for this?
        - Remove `TextPoint`?
        - Refactor `TextPointDisplay` into `CaretPosition` in the main crate.
            - Implement `get_caret_position` getter property.
            - Use case is display in a status bar.

    - Need to find closest insert point from mouse cursor point.
        - Support ligatures (click in middle works).
    
    - Review https://searchfox.org/mozilla-central/source/layout/generic/nsTextFrame.cpp#7534
        - ligated emoji sequence
        - all solved by grapheme clusters? https://unicode.org/reports/tr29/#Grapheme_Cluster_Boundaries
    - Grapheme Clusters crate: https://docs.rs/unicode-segmentation/latest/unicode_segmentation/trait.UnicodeSegmentation.html#tymethod.graphemes
    - "On a given system the backspace key might delete by code point, while the delete key may delete an entire clusters"
        - Observed this in Chrome, Firefox, VS and Word, use "ö̲" to test.
* Support replace (Insert mode in command line).
* Support buttons:
    - up and down arrows
    - page up and page down
* Implement selection.
    - Input replaces selection.
        - Char input, paste, IME
    - Impl cut & copy.
* Research text editors.

* Implement custom node access to text.
    - Clone text var in `ResolvedText`?
    - Getter property `get_transformed_text`, to get the text after whitespace transforms?

* Implement IME.
    - See https://github.com/rust-windowing/winit/issues/1497

# Config

* Implement ability to reset single config.
    - We want to support a Settings screen.
    - So a reset command and a way to indicate to the user that the value is not default.

    - Implement `AnyConfig::remove(&ConfigKey)`.
    - Implement `FallbackConfig::reset(&ConfigKey)`.
        - It sets the variable back to fallback and removes the entry in the top config.
    - Implement `FallbackConfig::is_fallback(&ConfigKey) -> ReadOnlyArcVar<bool>`.
        - It uses the new modify tags to map from the key var to a bool?
    - Implement `FallbackConfigRef`, to work like the `EditableUiNodeList`.
        - Implement `FallbackConfigRef::reset(ConfigKey)`.

# Extend View

* Finish implementing `using_display_items`.
* Review View-Process.md