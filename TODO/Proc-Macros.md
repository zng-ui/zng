# Proc-Macros TODO

Proc-macros are mostly implemented, there are some improvements we can make:

## Partial Assign

Implement syntax to allow assigning only one of the property inputs.

* If set in `when` the when only affects the input.
* If set in general the other values are the defaults or previous assigned value.

* Use cases:
    - Borders usually don't change widths in `when`, an assign of `border.sides = colors::GREEN;` is more compact.
        - Maybe we need better names for the border inputs..

## Property Value Map

Allow property values to depend of other properties:

```rust
#[widget($crate::foo)]
pub mod foo {
    properties! {
        a_property;

        /// This property is set only when `a_property` has a value and it is a mapping of the a_property.
        b_property = 1 + #a_property + 3;

        /// This property is set only when `a_property` is and it is a mapping of the a_property.
        on_prop = hn!(#a_property, |ctx, _| {
            println!(a_property.get(ctx));
        });
    }
}
```

* How does this integrate with `when`?
    - Not allowed in when assigns?
    - In the example above `a_property` is affected by `when` in the handler.
* Use cases:
    - Border color depend on background color with `when` only assigning background.

## Error & Warnings

* Review all error span hacks when this issue https://github.com/rust-lang/rust/issues/54725 is stable.
* Allow trailing semicolon in widget_new (those are only warnings in Rust, not errors)

## Difficult

* Pre-build to wasm: 
    Need $crate support, or to be able to read cargo.toml from wasm,
    both aren't natively supported with [`watt`](https://crates.io/crates/watt).