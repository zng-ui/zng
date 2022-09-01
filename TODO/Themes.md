# Themes TODO

* Implement dynamic when states, so that the same property name can participate of `when` chains declared across
    multiple places.

```rust
themable:

background_color = colors::RED;
when self.is_state {
    background_color = colors::GREEN;
}

theme:

// Case #1, the "default" value changes.
//
// * The property instance needs to change to the new one.
//  - There is no way to known that the theme property will be used in an widget with when.
//  - So all properties need to be configured with a var that can become a when var?
background_color = colors::BLUE;

----------

// Case #2: another state is appended.
//
// * The widget macro will generate the background with the default value + the when condition.
//    - Need to know that the default value is not an override.
//    - Can use the overridden property instance in this case, just append a state.
when self.is_another_state {
    background_color = colors::BLUE;
}

----------

// Case #3: default is overridden and another state is added.
//
// * The default instance is an override in this case, but the inherited `self.is_state` is still valid.
background_color = colors::YELLOW;
when self.is_another_state {
    background_color = colors::BLUE;
}
```

* Properties needs to setup with a variable that can upgrade to a when var.
    - Performance impact? Most properties only set with the LocalVar that gets inlined, changing to a dynamic var will ruin that.
    - What if DynProperty is a closure that generates the property node?

* Implicitly generated properties for when assigns need to be marked so that they are only used if the widget does not already have another default.
    - Not an issue if the only a property factory is recorded.

* Properties that are not `allowed_in_when` can be just instances.

## Other

* Make more widgets themable.
* Create a `ColorVars` in `window!` and derive all widget colors from it.