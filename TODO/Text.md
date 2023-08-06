# Text TODO

* `LINE_END_SEQUENCE_VAR`.
    - When char is `\n` or `\r` read this var and insert it instead. 
    - Review https://en.wikipedia.org/wiki/Newline

* Support replace (Insert mode in command line).

* Support `white_space` and `txt_transform` in edit mode.
* Support `white_space` and `txt_transform` in between text runs in the same wrap.

* Getter property `get_transformed_txt`, to get the text after whitespace & transforms?
    - Transformed should be in the SegmentedText already.
    - Whitespace needs processing.

* Implement text clip.
    - Ellipses, fade-out.

* Implement justify.

* Padding affects the baseline incorrectly.
    - Baseline in general is not tracked correctly?

* Spellchecker.
    - Use https://docs.rs/hunspell-rs
    - Used by Firefox and Chrome.

# Emoji Rendering

* Implement COLR v1 (gradients).
* Implement SVG.
* Implement bitmap.

# Underline Skip Glyphs

* Underline padding does not look right for some curved glyphs (parenthesis shows this), consider improving the `h_line_hits`.

# Mixed Content

* Implement text runs composed of multiple styles, same problem as font fallback?
* Implement widgets that derive text runs and styles from the text.
    - Markdown.
    - ANSI coloring (to show basic Inspector in a window).

# Selection

* Implement text selection, need to dynamically replace a text range with a different style.

# Font Loading

* Support web fonts.

# Font Features

* Finish implementing font features
 - main: https://developer.mozilla.org/en-US/docs/Web/CSS/font-feature-settings
 - 5 - https://helpx.adobe.com/pt/fonts/user-guide.html/pt/fonts/using/open-type-syntax.ug.html#calt
 - review - https://harfbuzz.github.io/shaping-opentype-features.html

# Shared Text

* Let multiple text widgets share a single text, dynamically splitting the text as each widget fills.
    - This enables complex magazine formatting.

# ANSI Text

* Line numbers.
* Virtualization, only parse/generate visible pages.

# Hyphenation

* Hyphenation does not backtrack, the word hyphenation end-up split as "hy-phe-nation" when it could be "hyphe-nation".
    - This is because the split segments are never rejoined?

# Overflow

* Fade-out.
* Scroll on hover.
* Ellipses.