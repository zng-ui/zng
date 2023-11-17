# Text TODO

* Support replace (Insert mode in command line).
* Support `white_space` and `txt_transform` in between text runs in the same wrap.
    - Right now we manually implement this for `Markdown!`.

* Implement justify.

* Padding affects the baseline incorrectly.
    - Baseline in general is not tracked correctly?

* (`SHIFT+Click`):
    - Implement a way to extend selections that were started with double or triple clicks by char instead of by word or line.
    - VSCode does it when you shift click inside text that has already been selected, but it also loses the initial selection.

# Edit

* Edit across multiple texts in the same wrap container.
    - Have a `txt_edit_scope` that is set in a parent widget and joins all editable text inside.
    - Use the focus navigation to jump caret across texts.
        - For left/right use logical nav.
        - For up/down use directional nav.
    - The `txt_edit_scope` must take-over some text commands:
        - Undo, select.
        - Clipboard.
            - Text is joined for copy across widgets.
    - Selection is defined locally in each text in selection.
        - A parent selection type defines the start and end widget only.
        - The start widget contains a selection from the char start to the widget's text end.
        - Touch caret observes this and only renders one of the carets.
            - It knows the direction by the second selection index being in the start or end of the local text.

* Support `white_space` and `txt_transform` in edit mode.
* Spellchecker.
    - Use https://docs.rs/hunspell-rs
    - Used by Firefox and Chrome.

# Emoji Rendering

* Implement COLR v1 (gradients).
* Implement SVG.
* Implement bitmap.

# Underline Skip Glyphs

* Underline padding does not look right for some curved glyphs (parenthesis shows this), consider improving the `h_line_hits`.

# Font Loading

* Support web fonts.

# Shared Text

* Let multiple text widgets share a single text, dynamically splitting the text as each widget fills.
    - This enables complex magazine formatting.

# Hyphenation

* Hyphenation does not backtrack, the word hyphenation end-up split as "hy-phe-nation" when it could be "hyphe-nation".
    - This is because the split segments are never rejoined?
