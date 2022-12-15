* Image render requests a parent window, it causes errors because window with parents can't be parent of the image headless window.
    - We need the parent to load the right color-scheme in the image.
    - Allow headless children in any headed window?

* Implement `markdown!`.
    - List number horizontal alignment.
        - Need something like a grid here as well? To share the column size.
    - Table, data format and panel?
        - Need grid layout?
    - Links and footnotes.
        - Implement something that automatically selects the best side of the target widget to open the link tool-tip.
        - Fix file links display (don't show added relative path?).
    - Tool-tips.
        - Implement basic tool-tip.

* Implement `TextAlign` across multiple inlined texts.
* Implement `LayoutDirection` for `flow!`.

* Continue "#Parallel UI" in `./Performance.md`.
    - Refactor services into `app_local!` backed structs, with associated functions.
        - Remove `Services`.
* Review all docs.
    - Mentions of threads in particular.
