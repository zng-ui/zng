# TextInput

* Fix emoji segmenting, example: "🙎🏻‍♀️"
    - `snap_grapheme_boundary` does not work for "🙎🏻‍♀️" with the quotes.
* Implement cursor position.
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

* Watermark text shows caret, it should not fore multiple reasons:
    - The txt property is not set to a read-write var.
    - Background widgets are not interactive.

* Ctrl+Shift+I when focusing TextInput inserts a tab and still opens the Inspector.
    - We are receiving a TAB for some reason, but we are stopping propagation.
    - Char event is not linked with key press event stop propagation does nothing.
        - Is a different event from Winit.
        - The next version of Winit will fix this: https://github.com/rust-windowing/winit/issues/753

# View!

* `view` really needs to be a widget.
    - In the icon example the view is not centered in the stack because
      stack align does not work with non-widget nodes.

# WINDOW_CTRL

* Refactor into an extension trait.
    - Is more discoverable as an extension trait, maybe suggested by tooling (rustc, ra)?

# Vars

* Var binding updates refactored to apply with the same causing update.
    - Review why we need `apply_update_id` in when, merge vars.
    - Was used to avoid inserting multiple update requests per "cycle".
    - Now is wrong.
    - Case: merge_var with 10 inputs, if all 10 inputs request updates in the same cycle
            we now update the merge_var only once after the first input update, the others are ignored!
        - If we remove the apply_update_id this is fixed, but now the merge_var! updates 10 times, for useless 9 calls.
        - Removing it for now.
* `text.rs` example: cancelling open dialog breaks it.

# View-Process

* Test async dialogs in Windows.
    - Use it if they are modal.
    - Async can continue animations.
* Implement custom event sender.
* Implement OpenGL example.
    - Overlay and texture image.

# Tooltip

* `tooltip` -> `disable_tooltip` do not swap when widget is disabled while the tooltip is visible.

# is_hovered

* `is_hovered` does not update back to `true` when the hovered widget is enabled.