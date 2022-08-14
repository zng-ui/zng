# Themes TODO

* Implement a `button_theme` that inherits from `theme`.
* Make more widgets themable.
* It looks like most widgets will have a *light* and *dark*  theme pair, maybe we need a `ThemePair` type.
    - That means we can select the correct one on init, without needing to declare a `when` for each widget.