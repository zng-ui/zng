# Inline Bidi

* General idea, collect info about items in the joiner rows during measure, define position of these items during layout.
  - This API extents the same pattern of getting the size of first & last rows, then defining the rectangle of their position.
  - Inlined widgets can then announce the continuous areas of the first and last row, so that properties can clip the spaces
    filled by resorted sibling fragments.
* The resorting algorithm it-self needs studying.
  - HTML specs recommends applying the bidi algorithm to text generated from the elements and using the result.
    - There is no way to compute the bidi algorithm in parallel inside a single line.
    - Wrap is done before before sorting at least, so the scope of the problem is a single row, and we already know the row width.
      - But our current `ShapingText` already resorts as soon as it ends a line.
      - We need to resort in the context of the full line.
      - `ShapingText` does not mess with the logical order of segments, so the `wrap!` panel can still resort with the full context.
      - The `ShapingText` needs to store advance/width of segments?
        - Can't use the glyph positions to compute because they may be reordered?
        - We already temporarily compute then in `finish_current_line_bidi`.
        - The position of items and width needs to be in `f32` pixels too.
  - Can we implement something initially with just the `LayoutDirection` for each item?
    - This lets us implement the basic resort layout communication, the clips and debug the segment widths.
    - But after the full class of the characters need to be retained, maybe we can store a sample char of each segment?
    - So we need to invent a temporary algorithm, no lets do the sample char idea.

# Other

* Implement `switch_style!` for toggle.

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