# Proc-Macros TODO

Proc-macros are mostly implemented, there are some improvements we can make:

* Sort property build actions by importance?
    - Right now we just have one, `easing` but can be many.
* Support custom syntax in widget macros?
```rust
// - the { } after the path is optional, it contains `(<rule>) => { <widget-init> };`.
// - this rules are inserted before all others in the widget macro.
// - the <rule> is a normal macro_rules capture rule.
// - the <widget-init> is the tokens that go inside the braces of the widget macro.
//
// This limitation or only expanding to widget new tokens ensures that the widget type is consistent. Implementers
// can still redirect to a more elaborate macro like `formatx!` in the example, in property value positions.
#[widget($crate::widgets::text {
    ($txt:expr) => {
        txt = $txt;
    };
    ($txt:tt, $($format:tt)*) => {
        txt = $crate::core::text::formatx!($txt, $($format)*);
    };
})]

// expands to:

macro_rules! text {
    ($txt:expr) => {
        $crate::widgets::text! {
            txt = $txt;
        }
    };
    ($txt:tt, $($format:tt)*) => {
        $crate::widgets::text! {
            txt = $crate::core::text::formatx!($txt, $($format)*);
        }
    };

    // normal rules, proabably need a less cool escape, a widget may want to use ">>".
    (>> if!mixin) => { .. }
    (>> if mixin) => { .. }
    ($($tt:tt)*) => { .. }

    // maybe
    (zero_ui_widget: if!mixin) => { .. }
    (zero_ui_widget: ifmixin) => { .. }
    ($($tt:tt)*) => { .. }
}
```

* Reduce the capture boilerplate?
    - Generate capture based on function inputs?

## Partial Assign

Implement syntax to allow assigning only one of the property inputs.

* If set in `when` the when only affects the input.
* If set in general the other values are the defaults or previous assigned value.

* Use cases:
    - Borders usually don't change widths in `when`, an assign of `border.sides = colors::GREEN;` is more compact.
        - Maybe we need better names for the border inputs..
    - Animations may target just one member.

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
* Implement validation for missing `init_handles` call in `init`.

## Difficult

* Pre-build to wasm: 
    Need $crate support, or to be able to read cargo.toml from wasm,
    both aren't natively supported with [`watt`](https://crates.io/crates/watt).