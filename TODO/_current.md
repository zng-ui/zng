* Text shaping needs "Language" and "Script".
* Expand WidgetInfo to provide a RenderTransform.
* Colors don't match other apps, clear_color and background_color also does not match.
* WindowChangedEvent fired on first resize.

# Layer

* Focus does not recover to new root scope.
    - Disable window.
    - Show overlay with own focus scope.
    Ideal Behavior: focus on the overlay as if the window opened with it.