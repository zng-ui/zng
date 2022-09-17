# Styles TODO

* Dynamic `unset!`?
    - Records unsets in the `DynWidget`, remove unsets from the `DynWidgetNode`.
    - Affected by importance.

* Configurable *importance*:
```rust
style! {
    #[dyn_importance = 999]
    background_color = colors::PINK;
}
```

* Review all widget and mixin styles, most should be `style!` based.
* Make more widgets styleable.
    - Checkbox is already in example, needs a style.
* Add some color to the default styles.