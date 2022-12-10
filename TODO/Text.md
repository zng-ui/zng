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
* Implement text start/end aligns.
* Implement text clip.
* Implement split/join segments.
* Implement wrapping and justify.
* Implement font-features and ligatures.
    - Create another icon crate with the new Material Icons to demonstrate this.
* Padding affects the baseline incorrectly.
    - Baseline in general is not tracked correctly?
* Copy Chromium font fallback selection https://github.com/chromium/chromium/blob/0e41e52f5a91bb812f1f35a94a379ee3655129b0/third_party/blink/renderer/platform/fonts/win/font_fallback_win.cc#L104

* Text Rendering, enable per-font config, https://docs.rs/webrender_api/0.61.0/x86_64-pc-windows-msvc/webrender_api/struct.FontInstanceOptions.html, integrate this with Renderer level config.
* Emoticon rendering, multi-colored fonts.

* Hyphenation, use https://sourceforge.net/projects/hunspell/files/Hyphen/2.8/?

* RTL per line.

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
    - This allows complex for complex magazine formatting.

# ANSI Text

* Line numbers.
* Virtualization, only parse/generate visible pages.

# Wrap

* Implement line alignment for `wrap!`. Right now variable font rows don't align.
* Wrapped text background does not look right, need to track every line in `InlineLayout`?
* 