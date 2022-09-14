# Themes TODO

* Mark properties used in when conditions.
* Change widget macros to allow when property without default value in dynamic widgets.
    - This is required specifically for the `theme!` widget, maybe it can be an opt-in flag the the `#[widget]` attribute?

* After dynamic when, refactor theme selection to allow multiple themes.
    - Button has a `base_theme::padding` inherited by `dark_theme` and `ligh_theme`, but if we want to change the button padding for
      all buttons we need to recreate the `theme::pair` selection and instantiate the two *final* themes.
    - Ideally we just set *something* that only has the new `padding` assign.
    - Try to make a *selector* that targets, widget types, ids and `class`.
        - The class can be a normal property in `themable`, it can be captured in `new_dyn` also or we
            can extract it from the dyn properties?
                - Capture is more clear, we already capture `id`.
        - CSS selectors overlap the `when` feature, in our API this is a selector that finds the type and the theme content
            has the `when self.is_hovered` or whatever.
        - The dark/light live match looks very important, maybe something specific for it directly in the selector?

* Make more widgets themable.
    - Checkbox is already in example, needs a theme.
* Create a `ColorVars` in `window!` and derive all widget colors from it.