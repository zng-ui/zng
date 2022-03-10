* Finish `FontList::shape_text`, font fallback.
   - Optimize `split_remove` calls?

* Review child priority integration with `WidgetLayout`, what happens when we add a border in child { }?
   - It does not work:
    - What do we lose if we remove `child_border` and `child_fill`?
    - Or do we make it work like an *anonymous* container widget, `WidgetLayout::with_widget_child` in `new_child_context`? 

* Use `windows-sys`, crates (`parking_lot`, `winit`) are updating to use the official WinAPI crate, after `winit` update we should as well.