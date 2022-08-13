# Themes TODO

* The `theme!` init is not working, call to init does not go in.
    - Is there a way to implement this without messing with the widget_new macro?
        - Can we init the nodes as the widget is inited?

* Create a `theme_mixin` using doc-hidden properties, can it work? Need to review mixin priority.
* Implement a `button_theme` that inherits from `theme`.

* Theme generator (selector):
    - Themes can be selected via query that is a predicate closure that runs with the `InfoContext` of the target widget.
    - BUT query runs on init, before info is collected, so there is no useful data to filter with.
    - There is no way to dynamically identify a widget type, even with inspector metadata all we get is a string.
