# Themes TODO

* Allow marking a required *default* assign for theme whens:
```rust
// the property `background_color` has a default value, so the dynamic when builder will have a default
// but it will not override the a default value set explicitly in a previous theme.
theme! {
    when self.is_focused {
        background_color = colors::RED;
    }
}

// some properties may not have a default value, but theme authors may want to support the same behavior of just
// adding a `when` to a existing property, but not ensures that there is a default elsewhere so:
theme! {
    #[dyn_fallback = true]
    foo = Foo::FOO;
    when self.is_focused {
        foo = Foo::BAR;
    }
}
// in this case the `FOO` value is only used if no default is defined anywhere.
```
    - Need to remove docs suggesting that its possible to declare a property without default.
    - Need to change the builder to not allow this.
    - Could be a generalized as `#[importance = {u16}]`?
        - Implies the importance of the when conditions?
        - Need to set the importance of when conditions?
        - Need this attribute anyway, to allow implementing CSS `!important`.
* Dynamic `unset!`?
    - Records unsets in the `DynWidget`, remove unsets from the `DynWidgetNode`.
    - Affected by importance?

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