# Text TODO

* Text Editable
    - Caret.
    - Selection.
* IME.
* `LineBreakVar`.
    - When char is `\n` or `\r` read this var and insert it instead. 
    - Review https://en.wikipedia.org/wiki/Newline

* Implement text align demo in text example.
 - Need to make space, implement tabbed content?
* Implement text clip.
* Implement split/join segments.
* Implement wrapping and justify.
* Implement font-features and ligatures.
    - Create another icon crate with the new Material Icons to demonstrate this.
* Padding affects the baseline incorrectly.
    - Baseline in general is not tracked correctly?
* Copy Chromium font fallback selection https://github.com/chromium/chromium/blob/0e41e52f5a91bb812f1f35a94a379ee3655129b0/third_party/blink/renderer/platform/fonts/win/font_fallback_win.cc#L104

* Text Rendering, enable per-font config, https://docs.rs/webrender_api/0.61.0/x86_64-pc-windows-msvc/webrender_api/struct.FontInstanceOptions.html, integrate this with Renderer level config.

* Hyphenation, use https://sourceforge.net/projects/hunspell/files/Hyphen/2.8/?

# Emoji Rendering

* Can be embedded bitmaps, SVGs or layered glyphs of different colors.
* Looks like webrender expects the glyphs to be pre-processed?
    - Yep, does not support any emoticon directly.
* Newer versions of harfbuzz have function to get the colors.

* We need more than one "fallback" font?
    - Right now we use "Segoe UI Symbol" in Windows.
    - We need to fallback to "Segoe UI Emoji" instead, or have both?
    - See what browsers do, maybe we need a "front" font, that is added on top of other fonts?
    - We have an special `TextSegmentKind::Emoji`, maybe we can have an `emoji_font_family` used exclusively for Emoji segs.

# Underline Skip Glyphs

* Underline padding does not look right for some curved glyphs (parenthesis shows this), consider improving the `h_line_hits`.

# Mixed Content

* Implement text runs composed of multiple styles, same problem as font fallback?
* Implement widgets that derive text runs and styles from the text.
    - Markdown.
    - ANSI coloring (to show basic Inspector in a window).

# Selection

* Implement text selection, need to dynamically replace a text range with a different style.

# Async Fonts

* Refactor fonts to be like the images service, async loading.
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

# Hyphenation & Word Break

* Word break must be only applied when the entire word does fit the line, 
    this does not happen when it is the last word in the `ShapedText` in an inline context.
* Hyphenation does not backtrack, the word hyphenation end-up split as "hy-phe-nation" when it could be "hyphe-nation".
    - This is because the split segments are never rejoined?

# Overflow

* Fade-out.
* Scroll on hover.
* Ellipses.