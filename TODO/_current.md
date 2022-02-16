* Implement baseline in widget layout (see CSS vertical-align).
* Finish `text!`
  - review `text!`
  - verify if the `line_height` property is doing what it should

* Review child priority integration with `WidgetLayout`, what happens when we add a border in child { }?
   - It does not work:
    - What do we lose if we remove `child_border` and `child_fill`?
    - Or do we make it work like an *anonymous* container widget, `WidgetLayout::with_widget_child` in `new_child_context`? 