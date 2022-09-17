# Styles TODO

* Implement `style::inherit` property to *inherit* from a style generator by creating an extension of it.
* Review all widget and mixin styles, most should be `style!` based.
* Make more widgets styleable.
    - Checkbox is already in example, needs a style.
* Add some color to the default styles.

* Create a `ColorVars` in `window!` and derive all widget colors from it.

* Configurable *importance*:
```rust
style! {
    #[dyn_importance = 999]
    background_color = colors::PINK;
}
```
* Dynamic `unset!`?
    - Records unsets in the `DynWidget`, remove unsets from the `DynWidgetNode`.
    - Affected by importance?