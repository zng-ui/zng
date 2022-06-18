# Text TODO

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
* Review TODOs in code.
* Copy Chromium font fallback selection https://github.com/chromium/chromium/blob/0e41e52f5a91bb812f1f35a94a379ee3655129b0/third_party/blink/renderer/platform/fonts/win/font_fallback_win.cc#L104

* Text Rendering, enable per-font config, https://docs.rs/webrender_api/0.61.0/x86_64-pc-windows-msvc/webrender_api/struct.FontInstanceOptions.html, integrate this with Renderer level config.
* Emoticon rendering, multi-colored fonts.

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