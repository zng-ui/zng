* Text shaping needs "Language" and "Script".
* Expand WidgetInfo to provide a RenderTransform.

* In the `layer` example the `LayerIndex::ADORNER, LayerMode::OFFSET` and `LayerMode::ALL` buttons are not producing the expected(?) result, this is more noticeable by upping the rotation of the buttons to -45.

* Buttons aren't receiving focus. 
  - tracked down the cause to `WidgetInfo::rendered()` returning false for widgets due to `self.info().rendered` never haven been set to true. (zero_ui_core -> widgetinfo.rs -> line 710)
  - the reason of it never been set to true was probably the change to `zero_ui_core -> render.rs` done in the commit 
    `Update webrender.` from ~1 day ago.
    - because `common_item_ps` sets rendered to true (by calling self.widget_rendered()) and `common_item_ps` was called inside 
      `push_widget_hit_area` which was deleted in that commit.