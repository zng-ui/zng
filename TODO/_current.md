* Review child priority integration with `WidgetLayout`, what happens when we add a border in child { }?
   - It does not work:
    - What do we lose if we remove `child_border` and `child_fill`?
    - Or do we make it work like an *anonymous* container widget, `WidgetLayout::with_widget_child` in `new_child_context`? 
    - Can we fully remove child { }? Widgets then only have new_fill(new_child()) and in new_child they can do "child" properties.
      - Can't `unset!` padding if we do this.