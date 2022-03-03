# Text TODO

* Implement wrapping, text align and justify.
* Implement font-features.
* Review TODOs in code.
* Copy Chromium font fallback selection https://github.com/chromium/chromium/blob/0e41e52f5a91bb812f1f35a94a379ee3655129b0/third_party/blink/renderer/platform/fonts/win/font_fallback_win.cc#L104

# Underline Skip Glyphs

* Underline padding does not look right for some curved glyphs (parenthesis shows this), consider improving the `h_line_hits`.

# Mixed Content

* Implement text runs composed of multiple styles, same problem as font fallback?
* Implement widgets that derive text runs and styles from the text.
    - Markdown.
    - ANSI coloring (to show basic Inspector in a window).

# Selection

* Implement text selection, need to dynamically replace a text range with a different style.