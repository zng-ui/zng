# Proc-Macros TODO

Proc-macros are mostly implemented, there are some improvements we can make:

* Improve property `allowed_in_when` validation for generics, generate a `new` like call for each
  argument, instead of all at once.
* Support doc(cfg).
* Support cfg in captures.
* Allow "property as new_name" syntax in widget_new? Can be used for things like double fancy borders.
* Use `get_` prefix for properties that only return a value, can teach as inverse of normal accessor methods where setting uses the `set_` prefix
and getting only uses the name directly.
* Custom lints for when widgets do not delegate to parent constructor functions that have custom nodes.

## Widget Bind-Self

```rust
#[widget($crate::foo)]
pub mod foo {
    properties! {
        a_property;

        /// This property is set only when `a_property` is and it is a mapping of the a_property.
        b_property = 1 + self.a_property + 3;
    }
}

// # Can we allow handler capture too?

#[widget($crate::foo)]
pub mod foo {
    properties! {
        a_property;

        /// This property is set only when `a_property` is and it is a mapping of the a_property.
        b_property = hn!(self.a_property, |ctx, _| {
            println!(a_property.get(ctx));
        });
    }
}

// We can't reuse the the `when` code for handlers because they are not allowed in `when`.
```

# Difficult

* Figure out a way to enable auto-complete and hover tooltip inside macro code?
* Pre-build to wasm: 
    Need $crate support, or to be able to read cargo.toml from wasm,
    both aren't natively supported with [`watt`](https://crates.io/crates/watt).