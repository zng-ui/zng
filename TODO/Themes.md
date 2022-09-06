* `checkbox` sets padding to `0`, but parent padding is applied?
* `text_input` margin not applied.

# Themes TODO

* Implement dynamic when in `DynWidget`.
    - Initialize properties without when for dynamic in `widget_new!`.
    - Initialize when as `DynPropWhenInfo` in `widget_new!`.
    - Generate constructor from dynamic in `#[property]`.
    - Implement dynamic when in `DynWidgetNode`.

## Other

* Make more widgets themable.
* Create a `ColorVars` in `window!` and derive all widget colors from it.