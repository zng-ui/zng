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

```log
!!: with_index_node update in WidgetId("btn-1")/Arc(0x1ab36d78e80) to Some(1)
!!: with_index_node update in WidgetId("btn-0")/Arc(0x1ab36d78be0) to Some(0)
```

* Issue is with when expressions?
    - Yes, same issue for is_hovered

```rust
when *#is_hovered {
    background_color = colors::DARK_BLUE;
}
```

* Same when var ends up in two instances:

```log
WhenInputVar.set
WhenInputVar.new (use)

             WhenInputVar  SettedVar

parallel = false

!!: set when 0x197df0a83e0 Arc(0x197e119d220)
!!: set when 0x197e10cb790 Arc(0x197e119d280)
!!: set when 0x197e10cb2e0 Arc(0x197e119c5c0)
!!: set when 0x197e10cb3a0 Arc(0x197e119cf20)
!!: set when 0x197df0a8170 Arc(0x197e119d4c0)
!!: use when 0x197e10cb790 Arc(0x197e119d280)
!!: use when 0x197e10cb3a0 Arc(0x197e119cf20)
!!: use when 0x197df0a83e0 Arc(0x197e119d220)
!!: use when 0x197e10cb2e0 Arc(0x197e119c5c0)
!!: use when 0x197df0a8170 Arc(0x197e119d4c0)
!!: set when 0x197df0a83e0 Arc(0x197defd2800)
!!: set when 0x197e3d82b00 Arc(0x197defd2860)
!!: set when 0x197e3d81b10 Arc(0x197defd28c0)
!!: set when 0x197e3d82130 Arc(0x197defd2920)
!!: set when 0x197df0a7f30 Arc(0x197defd2b00)
!!: use when 0x197e3d82b00 Arc(0x197defd2860)
!!: use when 0x197e3d82130 Arc(0x197defd2920)
!!: use when 0x197df0a83e0 Arc(0x197defd2800)
!!: use when 0x197e3d81b10 Arc(0x197defd28c0)
!!: use when 0x197df0a7f30 Arc(0x197defd2b00)


==========
parallel = true

!!: set when 0x238206fccc0 Arc(0x23820711480)
!!: set when 0x238206fccc0 Arc(0x23820710ca0) !!
!!: set when 0x238231e8540 Arc(0x238207114e0)
!!: set when 0x238231e2aa0 Arc(0x23820711600)
!!: set when 0x238231e86f0 Arc(0x23820710dc0)
!!: set when 0x238231e25c0 Arc(0x238207116c0)
!!: set when 0x238206fc240 Arc(0x23820711720)
!!: set when 0x238231e2170 Arc(0x238207115a0)
!!: set when 0x238231e2bf0 Arc(0x23820711660)
!!: set when 0x238206fc300 Arc(0x238207117e0)
!!: use when 0x238231e8540 Arc(0x238207114e0)
!!: use when 0x238231e86f0 Arc(0x23820710dc0)
!!: use when 0x238231e25c0 Arc(0x238207116c0)
!!: use when 0x238231e2bf0 Arc(0x23820711660)
!!: use when 0x238206fc300 Arc(0x238207117e0)
!!: use when 0x238206fc240 Arc(0x23820711720)
!!: use when 0x238206fccc0 Arc(0x23820710ca0)
!!: use when 0x238206fccc0 Arc(0x23820710ca0)
!!: use when 0x238231e2170 Arc(0x238207115a0)
!!: use when 0x238231e2aa0 Arc(0x23820711600)
```

* Issue is parallel init of the returned `WhenInputVar::new`.
    - Can't deep clone because the returned var is the same.

# How To Fix

* `WhenInputVar` needs to store multiple values.
    - We may need a node to set context ids, to identify the value in the contextualized var.
    - No guarantee the when var is actualized on init?
        - If this is the case even non-parallel usages might be broken.
        - VERIFY.
* Both input and var are in `WhenInfo`.
    - Var is merged, but its there.
    - Maybe we can refactor when to some sort of closure that generates the condition var only
      after all inputs are resolved.
    - Need large refactor, right now the var is embedded in an `expr_var!` that is created at
      the declaration point, turning the expr to a closure will mess that up.
* On clone of `WhenInfo` visit the placeholder ContextualizedVar, somehow.
    - Could expand the var API to allow visiting all components.
    - And implement a custom var type for this purpose, instead of the ContextualizedVar.
    - Instead of `Arc` is there a data struct that can "clone the graph"?
    - We definitely need a var `deep_clone`.
        - `expr_var!` expands to map or merge for when expressions with input.
        - These use ContextualizedVar inside.
        - No way to visit the captures of the ContextualizedVar closure.
    - Have a `PlaceholderVar<T>`.
        - Implement `AnyVar::clone_with`.