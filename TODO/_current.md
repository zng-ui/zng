* Fix memory leak in switch animation.
  - Repeat it the memory doubles every time.
  - Caused by `x = 0` animating to `x = 100` creating a `Length::Expr` that just keeps growing.
    - May need to rethink the entire length expression thing.

* Bidi reorder needs to intertwine the first and last lines.
    -  `النص ثنائي الاتجاه (بالإنجليزية: Bi **directional** text)‏ هو نص يحتوي على نص في كل من`
    - The markdown needs to layout `Bi directional text`, but because we split in 3 texts it
      `text direction Bi`, because of the layout direction.
    - The `wrap!` panel needs even more control of the first&last lines of children.
    - It needs to extend the text with `ARABIC Bi` to `ARABIC ########### Bi` where the `#` marks a blank
      space to fit the `directional` text and the ` text) ########### ARABIC` fragment of the last child.
    - Or we can have the markdown split the text more somehow.
    - Lets try to make a more easy layout API first, something that the `wrap!` panel can easily sort without
      needing to know the full bidi algorithm?

* Implement vertical text.
    - Need to expand `LayoutDirection` to define chars and lines direction.
    - Or a different layout property for the "lines" direction.
    - See `./Layout.md#Direction`.

* Continue "#Parallel UI" in `./Performance.md`.
    - Refactor services into `app_local!` backed structs, with associated functions.
        - Remove `Services`.

* Review all docs.
    - Mentions of threads in particular.