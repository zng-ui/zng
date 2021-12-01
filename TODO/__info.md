# TODO

Focus not removed by visibility change.
 See tests/focus/focused_removed_by_collapsing
     tests/focus/focused_removed_by_hiding

## Bounds and Visibility

* If an widget was not rendering in the first `WidgetInfoChangedEvent` its bounds and visibility will be out-of-date.
   The tests/focus.rs/focus_continued_after_widget_id_move fails because of this.
* 