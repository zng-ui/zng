# Themes TODO

* Implement `child_effect` property to apply an effect on the descendants.
    - Use it to implement disabled as desaturation+opacity of content.
* Implement `theme::inherit` property to *inherit* from a theme generator by creating an extension of it.
* Review all widget and mixin themes, most should be `theme!` based.
* Review text color, should not be full black & white by default.
* Make more widgets themable.
    - Checkbox is already in example, needs a theme.
* Add some color to the default theme.

* Create a `ColorVars` in `window!` and derive all widget colors from it.

* Configurable *importance*:
```rust
theme! {
    #[dyn_importance = 999]
    background_color = colors::PINK;
}
```
* Dynamic `unset!`?
    - Records unsets in the `DynWidget`, remove unsets from the `DynWidgetNode`.
    - Affected by importance?