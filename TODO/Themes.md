# Themes TODO

* Create a `theme` "widget" that setups `AdoptiveNode` insertion points for each property priority and returns a `ThemeBundle`.
    - DONE.
* Create a `themable` widget that uses the theme bundle insertion points to inject dynamic theme properties in the tree.
    - ONGOING.

* Widgets can inherit from `theme` to define a default theme, like a `button_theme`.

* Themes can be selected via query that is a predicate closure that runs with the `InfoContext` of the target widget.
    - See `ThemeGenerator`.
