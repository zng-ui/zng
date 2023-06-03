# TextInput

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
    - home and end
    - up and down arrows
    - page up and page down
    - shift modifier?
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

* WATCHER `SKIP_READ` is breaking or test.

* Test reset.
    - Not working, see `config.rs/fallback_swap`.
    - FallbackConfig loses connection with sources?
        - No amount of updates gets the value, so probably.
    - Sometimes the test works, but due to the fallback loading faster.
    - Review `FallbackConfig::get_raw`.
* Implement ability to reset single config.
    - VSCode settings page can do this.
    - Config visitor?
        - And a method in `FallbackConfig`.