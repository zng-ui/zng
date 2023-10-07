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
