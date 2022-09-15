# Themes TODO

* Implement `themable::extend` to support theme chains now that we have full dynamic when support.
    - Refactor widget themes to only have one context_var and two properties `foo::theme::replace` and `foo::theme::extend`.
    - Use `theme::pair` to implement light/dark changes instead of using two themes.

* Review all widget and mixin themes, most should be `theme!` based.
* Make more widgets themable.
    - Checkbox is already in example, needs a theme.

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