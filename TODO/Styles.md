# Styles TODO

* Review all widget and mixin styles, most should be `Style!` based.
* Add some color to the default styles.

## Shared Style Ongoing Issue

This does not work:

```rust
// zero_ui::core::widget_base::parallel = false;
Stack! {
    direction = StackDirection::top_to_bottom();
    spacing = 2;
    children = (0..2).map(|i| Button! { child = Text!("Row {i}") }.boxed()).collect::<UiNodeVec>();
    button::extend_style = Style! {
        when *#stack::get_index % 2 == 0 {
            background_color = colors::DARK_BLUE;
        }
    };
}
```

* Only predicted issue with singleton style was nodes getting moved.
* Issue does not happen if `zero_ui::core::widget_base::parallel = false;`.