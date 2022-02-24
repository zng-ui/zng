# Baseline

* Layout properties need the baseline to implement alignment.
  Text widget needs the inner most size to calculate the line height that affects the baseline.
  Can we avoid two passes in text layout?
   - Yes, we know the baseline during measure! Just need to communicate it back to a custom `push_border` for arrange.

* Padding also affects the baseline in text.
   - Need to implement padding ourselves, is better performant as well, don't need to push a reference frame. 

* Currently the `WidgetLayoutInfo::baseline` is just an offset, we want layers to be able to find it as well.
   - Maybe we can change it to be from the top and call it `baseline_height`.

## Actions

* Refactor padding as part of `LayoutText`.
* Store the `baseline` in a private field of `ResolvedText` during arrange.
* Implement custom `push_border` for `text!` that uses the `baseline` in `with_inner`.
* Review `Align::BASELINE`, see Flutter `Baseline`, see `baseline_height` idea.

# Other

* Implement `Font::outline`.
* implement underline_skip GLYPHS, see `core::text::shaping::outline`.

* Review child priority integration with `WidgetLayout`, what happens when we add a border in child { }?
   - It does not work:
    - What do we lose if we remove `child_border` and `child_fill`?
    - Or do we make it work like an *anonymous* container widget, `WidgetLayout::with_widget_child` in `new_child_context`? 