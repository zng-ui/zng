* Text shaping needs "Language" and "Script".
* Colors don't match other apps, clear_color and background_color also does not match.
* WindowChangedEvent fired on first resize.
* Review `open_widget_display`, needs to be called with widget inner size to calculate default origin (center).

# Layer

* Expand WidgetInfo to provide a RenderTransform.
* Use the `WidgetInnerBoundsNode` to control all inner transforms.
    - Use it to enforce no render outside?

* Focus does not recover to new root scope.
    - Disable window.
    - Show overlay with own focus scope.
    Ideal Behavior: focus on the overlay as if the window opened with it.

* `with_context_var_fold` can be implemented with a `merge_var` like new `enabled`?