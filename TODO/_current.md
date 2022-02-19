* implement underline_skip SPACES and GLYPHS.
    - Fix SPACES first.
    - Get glyph shapes inside ShapedText?
        - use `font_kit` outline, ignore curves, compute intersections (min-max x).
        - Add extra *padding* to intersection cuts.
        - Need to consider line thickness also? See `p` glyph of Times New Roman.

* Review child priority integration with `WidgetLayout`, what happens when we add a border in child { }?
   - It does not work:
    - What do we lose if we remove `child_border` and `child_fill`?
    - Or do we make it work like an *anonymous* container widget, `WidgetLayout::with_widget_child` in `new_child_context`? 
