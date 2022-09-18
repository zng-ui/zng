# Styles TODO

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