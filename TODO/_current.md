* Implement abstraction for operations that only use one axis, we have duplicate algorithms in h_stack, v_stack, grid.
    - Instead of inventing a name for axis, cross-axis we can just have the width be height be swapped?
        - More readable, may cause algorithm that only works for width getting used in height.
    - Implement `stack` panel that can dynamically swap axis of operation.
  because of this.
* Implement abstraction to allow implementing measure and layout in the same code.
    - Can't short-circuit layout in every context, sometimes need to recreate a lot of the layout algorithm, leading to
      duplicate code.

* Grid.
    - Finish `!!:`.
    - Cell align.
    - Column & Row align for when all fixed is less then available.
    - Masonry align?
    - Support `lft` in spacing.
        - And padding? Need to capture padding if the case.

* Implement `markdown!`.
    - List number horizontal alignment.
        - Need something like a grid here as well? To share the column size.
        - Right now the bullet is inside the item already
    - Table, data format and panel?
        - Implement `grid!` with cell align.
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
