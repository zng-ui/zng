# Themes TODO

* Because themes don't override properties, some unexpected results can happen, for example
    setting `padding` on a theme and on the widget causes a sum of both paddings to be set.

* We need an alternative widget init that collects each property as a dynamic unit, like a `DynProperty` that
    is an `AdoptiveNode` with a property name field.
* Maybe implemented in `widget_new`, triggered by the signature of the constructor functions:
    - Advantages:
        - Allows full control of what properties are included or override.
        - Allows theme implementation.
        - Allows future CSS like theming.
    - Disadvantages:
        - Only widgets with dynamic constructors can be themable.
        - No theme_mixin, `text_input` can't just inherit from `text`.

```rust
#[widget($crate::foo)]
mod foo {

    // normal constructor
    fn new_fill(child: impl UiNode, capture: impl IntoVar<bool>) -> impl UiNode {
        child
    }

    // dynamic constructor:
    //
    // * Always two inputs required:
    //
    // * First is the output of the previous constructor function.
    //    - If we make `UiNode: Any` we can start a custom widget type in `new_child` and cast to the actual type
    //      in each constructor, this allows the removal of the thread_local hack in `Theme`.
    // * Secound is a vec of PropertyInstance of the priority.
    fn new_fill_dyn(child: impl UiNode, properties: Vec<PropertyInstance>, capture: impl IntoVar<bool>) -> impl UiNode {
        child
    }

    // ERRORS:
    //
    // * Declaring both is an error, "only one of `new_fill` or `new_fill_dyn` can be declared".
    // * Overriding a dynamic constructor with a static one, "cannot statically override `new_fill` because it is dynamic in `base::new_fill_dyn`".
    //      - This is so we don't acidentally break parts of themed widgets, is this really an error?
    //      - Can be a warning when diagnostics are stabilized.
}

pub struct DynProperty {
    // Property node.
    pub node: AdoptiveNode,
    // Name used to set the property.
    pub property: &'static str,

    // If was auto set by widget declaration or was set locally in the instance.
    //
    // Dynamic widgets can use this value to *override* properties with the same name.
    pub source: DynPropertySource,
    
    // Unique id for each property, like a `TypeId`?.
    //
    // Not needed on the first implementation.
    pub property_id: PropertyId,
}

pub enum DynPropertySource {
    // Set on the widget declaration.
    Widget,
    // Set on the widget instance.
    Instance,
}
```

# Others

* Rename all "theme" sub-modules of widgets to `vis`.
* Make more widgets themable.

* Implement system theme initial value in view-process.