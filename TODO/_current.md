* Merge `TextAlign` into `Align`.
    - Tried to use `TextAlign` in a `wrap!`, did not work.
    - `TextAlign::START` is something interesting in general, not just for text.
    - `TextAlign::JUSTIFY` is just `FILL` for text?

* Add more metadata for `row!`, `is_last`, `get_rev_index`.
    - Use this to fix the markdown table bottom border line.

* Grid pos-layout align.
    - Cell align.
    - Column & Row align for when all fixed is less then available.
    - Masonry align?
    - Support `lft` in spacing.
        - And padding? Need to capture padding if the case.

* Implement `markdown!`.
    - List number horizontal alignment.
        - Need something like a grid here as well? To share the column size.
        - Right now the bullet is inside the item already
    - Links and footnotes.
        - Implement something that automatically selects the best side of the target widget to open the link tool-tip.
        - Fix file links display (don't show added relative path?).
    - Tool-tips.
        - Implement basic tool-tip.

* Image render requests a parent window, it causes errors because window with parents can't be parent of the image headless window.
    - We need the parent to load the right color-scheme in the image.
    - Allow headless children in any headed window?

* Implement `TextAlign` across multiple inlined texts.
* Implement `LayoutDirection` for `flow!`.

* Continue "#Parallel UI" in `./Performance.md`.
    - Refactor services into `app_local!` backed structs, with associated functions.
        - Remove `Services`.
* Review all docs.
    - Mentions of threads in particular.
